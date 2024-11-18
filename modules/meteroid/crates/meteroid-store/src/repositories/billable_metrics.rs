use diesel_async::scoped_futures::ScopedFutureExt;
use error_stack::Report;
use uuid::Uuid;

use common_eventbus::Event;
use diesel_models::billable_metrics::{BillableMetricRow, BillableMetricRowNew};
use diesel_models::product_families::ProductFamilyRow;

use crate::domain::{
    BillableMetric, BillableMetricMeta, BillableMetricNew, PaginatedVec, PaginationRequest,
};
use crate::errors::StoreError;
use crate::utils::local_id::{IdType, LocalId};
use crate::{domain, Store, StoreResult};

#[async_trait::async_trait]
pub trait BillableMetricInterface {
    async fn find_billable_metric_by_id(
        &self,
        id: Uuid,
        tenant_id: Uuid,
    ) -> StoreResult<domain::BillableMetric>;

    async fn list_billable_metrics(
        &self,
        tenant_id: Uuid,
        pagination: PaginationRequest,
        product_family_local_id: String,
    ) -> StoreResult<PaginatedVec<domain::BillableMetricMeta>>;

    async fn insert_billable_metric(
        &self,
        billable_metric: domain::BillableMetricNew,
    ) -> StoreResult<domain::BillableMetric>;
}

#[async_trait::async_trait]
impl BillableMetricInterface for Store {
    async fn find_billable_metric_by_id(
        &self,
        id: Uuid,
        tenant_id: Uuid,
    ) -> StoreResult<domain::BillableMetric> {
        let mut conn = self.get_conn().await?;

        BillableMetricRow::find_by_id(&mut conn, id, tenant_id)
            .await
            .map_err(Into::into)
            .and_then(TryInto::try_into)
    }

    async fn list_billable_metrics(
        &self,
        tenant_id: Uuid,
        pagination: PaginationRequest,
        product_family_local_id: String,
    ) -> StoreResult<PaginatedVec<BillableMetricMeta>> {
        let mut conn = self.get_conn().await?;

        let rows = BillableMetricRow::list(
            &mut conn,
            tenant_id,
            pagination.into(),
            product_family_local_id,
        )
        .await
        .map_err(Into::<Report<StoreError>>::into)?;

        let res: PaginatedVec<BillableMetricMeta> = PaginatedVec {
            items: rows.items.into_iter().map(|s| s.into()).collect(),
            total_pages: rows.total_pages,
            total_results: rows.total_results,
        };

        Ok(res)
    }

    async fn insert_billable_metric(
        &self,
        billable_metric: BillableMetricNew,
    ) -> StoreResult<BillableMetric> {
        let mut conn = self.get_conn().await?;

        let family = ProductFamilyRow::find_by_local_id_and_tenant_id(
            &mut conn,
            &billable_metric.family_local_id,
            billable_metric.tenant_id,
        )
        .await
        .map_err(Into::<Report<StoreError>>::into)?;

        // TODO create product if None ?

        let insertable_entity = BillableMetricRowNew {
            id: Uuid::now_v7(),
            local_id: LocalId::generate_for(IdType::BillableMetric),
            name: billable_metric.name,
            description: billable_metric.description,
            code: billable_metric.code,
            aggregation_type: billable_metric.aggregation_type.into(),
            aggregation_key: billable_metric.aggregation_key,
            unit_conversion_factor: billable_metric.unit_conversion_factor,
            unit_conversion_rounding: billable_metric.unit_conversion_rounding.map(Into::into),
            segmentation_matrix: billable_metric
                .segmentation_matrix
                .map(|x| {
                    serde_json::to_value(&x).map_err(|e| {
                        StoreError::SerdeError(
                            "Failed to serialize segmentation_matrix".to_string(),
                            e,
                        )
                    })
                })
                .transpose()?,
            usage_group_key: billable_metric.usage_group_key,
            created_by: billable_metric.created_by,
            tenant_id: billable_metric.tenant_id,
            product_family_id: family.id,
            product_id: billable_metric.product_id,
        };

        let res: BillableMetric = self
            .transaction_with(&mut conn, |conn| {
                async move {
                    let res: BillableMetric = insertable_entity
                        .insert(conn)
                        .await
                        .map_err(Into::<Report<StoreError>>::into)
                        .and_then(TryInto::try_into)?;

                    let _ = &self
                        .usage_client
                        .register_meter(&res.tenant_id, &res)
                        .await
                        .map_err(|x| {
                            StoreError::MeteringServiceError(
                                "Failed to register meter".to_string(),
                                x,
                            )
                        })?;

                    Ok(res)
                }
                .scope_boxed()
            })
            .await?;

        let _ = self
            .eventbus
            .publish(Event::billable_metric_created(
                res.created_by,
                res.id,
                res.tenant_id,
            ))
            .await;

        Ok(res)
    }
}
