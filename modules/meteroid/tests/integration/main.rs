// cf https://xxchan.me/cs/2023/02/17/optimize-rust-comptime-en.html#step-4-single-binary-integration-test

mod e2e;
mod helpers;
mod metering_it;
mod meteroid_it;
mod test_add_ons;
mod test_auth_api_key;
mod test_auth_jwt;
mod test_basic;
mod test_billable_metric;
mod test_coupon;
mod test_customer;
mod test_idempotency;
mod test_idempotency_cache;
mod test_instance;
mod test_internal;
// mod test_payment;
mod test_billing;
mod test_plan;
mod test_product;
mod test_product_family;
mod test_schedule;
mod test_slot_transaction;
mod test_stats;
mod test_subscription;
mod test_tenant;
mod test_user;
mod test_webhooks_out;

mod data;
mod test_metering_ingestion;
