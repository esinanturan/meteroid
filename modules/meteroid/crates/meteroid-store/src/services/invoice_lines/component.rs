use chrono::{NaiveDate, NaiveTime};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal_macros::dec;

use crate::domain::enums::BillingType;
use crate::domain::*;
use crate::utils::local_id::LocalId;

use crate::services::clients::usage::{GroupedUsageData, UsageData};
use crate::services::invoice_lines::shared::{only_positive, only_positive_decimal};
use common_utils::decimals::ToSubunit;

use super::fees;
use crate::StoreResult;
use crate::errors::StoreError;
use crate::repositories::subscriptions::SubscriptionSlotsInterface;
use crate::services::Services;
use crate::store::PgConn;
use common_domain::ids::BillableMetricId;
use error_stack::{Report, ResultExt};

impl Services {
    pub(super) async fn compute_component<T: SubscriptionFeeInterface>(
        &self,
        conn: &mut PgConn,
        subscription_details: &SubscriptionDetails,
        component: &T,
        periods: ComponentPeriods,
        invoice_date: &NaiveDate,
        precision: u8,
    ) -> StoreResult<Vec<LineItem>> {
        let fixed_period = match periods.advance {
            Some(period) => period,
            None => return Ok(Vec::new()),
        };

        let is_first_period = periods.arrear.is_none();

        let mut lines: Vec<InvoiceLineInner> = vec![];

        match component.fee_ref() {
            SubscriptionFee::Rate { rate } => {
                lines.push(InvoiceLineInner::simple_prorated(
                    rate,
                    &dec!(1),
                    fixed_period,
                    periods.proration_factor,
                    precision,
                )?);
            }
            SubscriptionFee::OneTime { rate, quantity } => {
                // only for first period
                if is_first_period {
                    lines.push(InvoiceLineInner::simple_prorated(
                        rate,
                        &Decimal::from(*quantity),
                        fixed_period,
                        periods.proration_factor,
                        precision,
                    )?);
                }
            }
            SubscriptionFee::Recurring {
                rate,
                quantity,
                billing_type,
            } => match billing_type {
                BillingType::Advance => {
                    lines.push(InvoiceLineInner::simple_prorated(
                        rate,
                        &Decimal::from(*quantity),
                        fixed_period,
                        periods.proration_factor,
                        precision,
                    )?);
                }
                BillingType::Arrears => {
                    if let Some(arrears) = periods.arrear {
                        lines.push(InvoiceLineInner::simple_prorated(
                            rate,
                            &Decimal::from(*quantity),
                            arrears,
                            periods.proration_factor,
                            precision,
                        )?);
                    }
                }
            },
            SubscriptionFee::Slot {
                unit_rate,
                min_slots,
                max_slots,
                unit,
                ..
            } => {
                let slots = self
                    .fetch_slots(conn, invoice_date, unit.clone(), subscription_details) // TODO we need unit instead. That would allow for subscription components not linked to a plan. It'd also match Sequence model
                    .await?
                    .max(min_slots.unwrap_or(0) as u64)
                    .min(max_slots.unwrap_or(u32::MAX) as u64);

                lines.push(InvoiceLineInner::simple_prorated(
                    unit_rate,
                    &Decimal::from(slots),
                    fixed_period,
                    periods.proration_factor,
                    precision,
                )?);
            }
            SubscriptionFee::Capacity {
                rate,
                included,
                overage_rate,
                metric_id,
            } => {
                lines.push(InvoiceLineInner::simple_prorated(
                    rate,
                    &dec!(1),
                    fixed_period,
                    None, // no proration on capacity, as it provides a fixed amount
                    precision,
                )?);

                if let Some(arrear_period) = periods.arrear {
                    if overage_rate > &Decimal::ZERO {
                        let usage = self
                            .fetch_usage(arrear_period.clone(), *metric_id, subscription_details)
                            .await?
                            .single()?;

                        let overage_units = usage - Decimal::from(*included);

                        if overage_units > Decimal::ZERO {
                            let overage_price = overage_rate
                                .to_subunit_opt(precision)
                                .ok_or(StoreError::InvalidDecimal)
                                .attach_printable("Failed to convert overage_rate to subunit")?;
                            let overage_total = overage_price * overage_units.to_i64().unwrap_or(0);

                            let overage_line = InvoiceLineInner {
                                quantity: None,
                                unit_price: None,
                                total: overage_total as u64,
                                period: arrear_period,
                                is_prorated: false,
                                custom_line_name: None,
                                sublines: vec![SubLineItem {
                                    local_id: LocalId::no_prefix(),
                                    name: "Overage".to_string(),
                                    total: overage_total,
                                    quantity: overage_units,
                                    unit_price: *overage_rate,
                                    attributes: None,
                                }],
                            };

                            lines.push(overage_line);
                        }
                    }
                }
            }
            SubscriptionFee::Usage { metric_id, model } => {
                if let Some(arrear_period) = periods.arrear {
                    let usage = self
                        .fetch_usage(arrear_period.clone(), *metric_id, subscription_details)
                        .await?;

                    match model {
                        UsagePricingModel::Matrix { rates } => {
                            let mut sublines = vec![];

                            for rate in rates {
                                // for each rate, we get the quantity matching that rate
                                let quantity = usage
                                    .data
                                    .iter()
                                    .find(|usage| {
                                        let d1 = usage.dimensions.get(&rate.dimension1.key)
                                            == Some(&rate.dimension1.value);

                                        if let Some(dimension2) = &rate.dimension2 {
                                            d1 && usage.dimensions.get(&dimension2.key)
                                                == Some(&dimension2.value)
                                        } else {
                                            d1
                                        }
                                    })
                                    .map(|usage| usage.value)
                                    .unwrap_or(Decimal::ZERO);

                                let price_total = rate.per_unit_price * quantity;

                                let price_cents = only_positive(
                                    price_total
                                        .to_subunit_opt(precision)
                                        .ok_or(Report::new(StoreError::InvalidDecimal))
                                        .attach_printable(
                                            "Failed to convert price_total to subunit",
                                        )?,
                                );

                                if price_cents > 0 {
                                    // we concat rate.dimension1.value and rate.dimension2.value (if defined), separed by a coma. No coma if rate.dimension2 is None
                                    let name = format!(
                                        "{}{}",
                                        rate.dimension1.value,
                                        rate.dimension2
                                            .as_ref()
                                            .map(|d| format!(",{}", d.value))
                                            .unwrap_or_default()
                                    );
                                    sublines.push(SubLineItem {
                                        local_id: LocalId::no_prefix(),
                                        name, // TODO
                                        total: price_cents as i64,
                                        quantity,
                                        unit_price: rate.per_unit_price,
                                        attributes: Some(SubLineAttributes::Matrix {
                                            dimension1_key: rate.dimension1.key.clone(),
                                            dimension1_value: rate.dimension1.value.clone(),
                                            dimension2_key: rate
                                                .dimension2
                                                .as_ref()
                                                .map(|d| d.key.clone()),
                                            dimension2_value: rate
                                                .dimension2
                                                .as_ref()
                                                .map(|d| d.value.clone()),
                                        }),
                                    });
                                }
                            }

                            lines.push(InvoiceLineInner::from_sublines(
                                sublines,
                                arrear_period,
                                None,
                            )?);
                        }
                        model => {
                            let usage_units = usage.single()?;

                            //TODO only if price > 0 & usage > 0

                            match model {
                                UsagePricingModel::PerUnit { rate } => {
                                    lines.push(InvoiceLineInner::simple(
                                        rate,
                                        &usage_units,
                                        arrear_period,
                                        precision,
                                    )?);
                                }
                                UsagePricingModel::Tiered { tiers, block_size } => {
                                    lines.push(fees::compute_tier_price(
                                        usage_units,
                                        tiers,
                                        arrear_period,
                                        precision,
                                        block_size,
                                    )?);
                                }
                                UsagePricingModel::Volume { tiers, block_size } => {
                                    lines.push(fees::compute_volume_price(
                                        usage_units,
                                        tiers,
                                        arrear_period,
                                        precision,
                                        block_size,
                                    )?);
                                }
                                UsagePricingModel::Package { block_size, rate } => {
                                    // TODO we want some additional data in the frontend to display that "x$ per 20", total usage and block usage
                                    let package_size_decimal = Decimal::from(*block_size);
                                    let total_packages =
                                        (usage_units / package_size_decimal).ceil();

                                    let price_total = total_packages * *rate;

                                    lines.push(InvoiceLineInner::from_sublines(
                                        vec![SubLineItem {
                                            local_id: LocalId::no_prefix(),
                                            name: "Package".to_string(),
                                            total: price_total
                                                .to_subunit_opt(precision)
                                                .ok_or(Report::new(StoreError::InvalidDecimal))
                                                .attach_printable(
                                                    "Failed to convert price_total to subunit",
                                                )?,
                                            quantity: total_packages,
                                            unit_price: *rate,
                                            attributes: Some(SubLineAttributes::Package {
                                                raw_usage: usage_units,
                                            }),
                                        }],
                                        arrear_period,
                                        None,
                                    )?);
                                }
                                UsagePricingModel::Matrix { .. } => unreachable!(),
                            };
                        }
                    }
                }
            }
        }
        Ok(lines
            .into_iter()
            .map(|line| LineItem {
                local_id: LocalId::no_prefix(),
                name: line
                    .custom_line_name
                    .unwrap_or_else(|| component.name_ref().clone()),
                quantity: line.quantity,
                unit_price: line.unit_price,
                total: line.total as i64,
                start_date: line.period.start,
                end_date: line.period.end,

                sub_lines: line.sublines,
                is_prorated: line.is_prorated,
                price_component_id: component.price_component_id(),
                product_id: component.product_id(),
                metric_id: component.fee_ref().metric_id(),
                subtotal: line.total as i64, // TODO
                description: None,
            })
            .collect())
    }

    async fn fetch_usage(
        &self,
        period: Period,
        metric_id: BillableMetricId,
        subscription_details: &SubscriptionDetails,
    ) -> StoreResult<UsageData> {
        let metric = subscription_details
            .metrics
            .iter()
            .find(|metric| metric.id == metric_id)
            .ok_or(StoreError::ValueNotFound(format!(
                "metric with id {}",
                metric_id
            )))?;

        let usage = self
            .usage_client
            .fetch_usage(
                &subscription_details.subscription.tenant_id,
                &subscription_details.subscription.customer_id,
                metric,
                period,
            )
            .await?;

        if let Some(factor) = metric.unit_conversion_factor {
            if factor == 0 {
                return Ok(usage);
            }
            let usage = UsageData {
                period: usage.period,
                data: usage
                    .data
                    .iter()
                    .map(|usage| GroupedUsageData {
                        value: usage.value / Decimal::from_i32(factor).unwrap_or(dec!(1)),
                        dimensions: usage.dimensions.clone(),
                    })
                    .collect(),
            };
            return Ok(usage);
        }
        Ok(usage)
    }

    async fn fetch_slots(
        &self,
        conn: &mut PgConn,
        invoice_date: &NaiveDate,
        unit: String,
        subscription_details: &SubscriptionDetails,
    ) -> StoreResult<u64> {
        let quantity = self
            .store
            .get_current_slots_value_with_conn(
                conn,
                subscription_details.subscription.tenant_id,
                subscription_details.subscription.id,
                unit,
                Some(invoice_date.clone().and_time(NaiveTime::MIN)),
            )
            .await?;

        Ok(quantity as u64)
    }
}

pub struct InvoiceLineInner {
    pub total: u64,
    pub quantity: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub period: Period,
    pub custom_line_name: Option<String>,
    pub is_prorated: bool,
    pub sublines: Vec<SubLineItem>,
}

impl InvoiceLineInner {
    pub fn simple_prorated(
        rate: &Decimal,
        quantity: &Decimal,
        period: Period,
        proration_factor: Option<f64>,
        precision: u8,
    ) -> StoreResult<InvoiceLineInner> {
        let unit_price_cents = prorate_dec(*rate, proration_factor);

        let total = rate * quantity;

        let total_cents = prorate(
            total
                .to_subunit_opt(precision)
                .ok_or(Report::new(StoreError::InvalidDecimal))
                .attach_printable("Failed to convert price_total to subunit")?,
            proration_factor,
        );

        Ok(InvoiceLineInner {
            quantity: Some(*quantity),
            unit_price: Some(unit_price_cents),
            total: total_cents,
            period,
            custom_line_name: None,
            is_prorated: proration_factor.is_some_and(|f| f < 1.0),
            sublines: Vec::new(),
        })
    }

    pub fn simple(
        rate: &Decimal,
        quantity: &Decimal,
        period: Period,
        precision: u8,
    ) -> StoreResult<InvoiceLineInner> {
        Self::simple_prorated(rate, quantity, period, None, precision)
    }

    pub fn from_sublines(
        sublines: Vec<SubLineItem>,
        period: Period,
        proration_factor: Option<f64>,
    ) -> StoreResult<InvoiceLineInner> {
        let total = sublines.iter().map(|subline| subline.total).sum::<i64>();
        let total_cents = prorate(total, proration_factor);

        Ok(InvoiceLineInner {
            quantity: None,
            unit_price: None,
            total: total_cents,
            period,
            custom_line_name: None,
            is_prorated: proration_factor.is_some_and(|f| f < 1.0),
            sublines,
        })
    }
}

fn prorate(price_cents: i64, proration_factor: Option<f64>) -> u64 {
    match proration_factor {
        Some(proration_factor) => {
            let prorated_price = (price_cents as f64 * proration_factor).round() as i64;
            only_positive(prorated_price)
        }
        None => only_positive(price_cents),
    }
}

fn prorate_dec(price_cents: Decimal, proration_factor: Option<f64>) -> Decimal {
    match proration_factor {
        Some(proration_factor) => {
            let prorated_price =
                price_cents * Decimal::from_f64(proration_factor).unwrap_or(dec!(1.0));
            only_positive_decimal(prorated_price)
        }
        None => only_positive_decimal(price_cents),
    }
}
