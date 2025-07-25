use crate::config::Config;
use crate::eventbus::{create_eventbus_memory, setup_eventbus_handlers};
use meteroid_store::Store;
use meteroid_store::store::StoreConfig;

static STORE: tokio::sync::OnceCell<Store> = tokio::sync::OnceCell::const_new();

pub async fn get_store() -> &'static Store {
    STORE
        .get_or_init(|| async {
            let config = Config::get();

            let mailer = meteroid_mailer::service::mailer_service(config.mailer.clone());
            let oauth = meteroid_oauth::service::OauthServices::new(config.oauth.clone());

            let store = Store::new(StoreConfig {
                database_url: config.database_url.clone(),
                crypt_key: config.secrets_crypt_key.clone(),
                jwt_secret: config.jwt_secret.clone(),
                multi_organization_enabled: config.multi_organization_enabled,
                skip_email_validation: !config.mailer_enabled(),
                public_url: config.public_url.clone(),
                eventbus: create_eventbus_memory(),
                mailer,
                oauth,
                domains_whitelist: config.domains_whitelist(),
            })
            .expect("Failed to initialize store");

            setup_eventbus_handlers(store.clone(), config.clone()).await;

            store
        })
        .await
}
