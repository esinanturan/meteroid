use testcontainers::clients::Cli;

use crate::helpers;
use crate::meteroid_it;
use crate::meteroid_it::container::SeedLevel;
use meteroid_grpc::meteroid::api;
use meteroid_grpc::meteroid::api::users::v1::UserRole;

#[tokio::test]
async fn test_webhook_endpoint_out() {
    // Generic setup
    helpers::init::logging();
    let docker = Cli::default();
    let (_postgres_container, postgres_connection_string) =
        meteroid_it::container::start_postgres(&docker);
    let setup =
        meteroid_it::container::start_meteroid(postgres_connection_string, SeedLevel::MINIMAL)
            .await;

    let auth = meteroid_it::svc_auth::login(setup.channel.clone()).await;
    assert_eq!(auth.user.unwrap().role, UserRole::Admin as i32);

    let clients = meteroid_it::clients::AllClients::from_channel(
        setup.channel.clone(),
        auth.token.clone().as_str(),
        "a712afi5lzhk",
    );

    let events_to_listen = vec![
        api::webhooks::out::v1::WebhookEventType::CustomerCreated as i32,
        api::webhooks::out::v1::WebhookEventType::SubscriptionCreated as i32,
    ];

    // create endpoint 1
    let created = clients
        .webhooks_out
        .clone()
        .create_webhook_endpoint(api::webhooks::out::v1::CreateWebhookEndpointRequest {
            url: "https://example.com".to_string(),
            description: Some("Test".to_string()),
            events_to_listen: events_to_listen.clone(),
        })
        .await
        .unwrap()
        .into_inner()
        .endpoint
        .unwrap();

    assert_eq!(created.url.as_str(), "https://example.com");
    assert_eq!(created.description, Some("Test".to_string()));
    assert_eq!(created.events_to_listen, events_to_listen.clone());
    assert!(created.enabled);
    assert!(meteroid::webhook::Webhook::new(created.secret.as_str()).is_ok());

    // list endpoints
    let listed = clients
        .webhooks_out
        .clone()
        .list_webhook_endpoints(api::webhooks::out::v1::ListWebhookEndpointsRequest {})
        .await
        .unwrap()
        .into_inner()
        .endpoints;

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], created);
    // teardown
    meteroid_it::container::terminate_meteroid(setup.token, setup.join_handle).await
}