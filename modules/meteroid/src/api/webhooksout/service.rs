use tonic::{Request, Response, Status};

use common_grpc::middleware::server::auth::RequestExt;
use meteroid_grpc::meteroid::api::webhooks::out::v1::webhooks_service_server::WebhooksService;
use meteroid_grpc::meteroid::api::webhooks::out::v1::{
    CreateWebhookEndpointRequest, CreateWebhookEndpointResponse, GetWebhookEndpointRequest,
    GetWebhookEndpointResponse, ListWebhookEndpointsRequest, ListWebhookEndpointsResponse,
};
use meteroid_store::repositories::webhooks::WebhooksInterface;

use crate::api::webhooksout::error::WebhookApiError;
use crate::api::webhooksout::mapping::endpoint;
use crate::api::webhooksout::WebhooksServiceComponents;

#[tonic::async_trait]
impl WebhooksService for WebhooksServiceComponents {
    #[tracing::instrument(skip_all)]
    async fn create_webhook_endpoint(
        &self,
        request: Request<CreateWebhookEndpointRequest>,
    ) -> Result<Response<CreateWebhookEndpointResponse>, Status> {
        let tenant_id = request.tenant()?;

        let req = request.into_inner();

        let domain = endpoint::new_req_to_domain(tenant_id, req)?;

        let endpoint = self
            .store
            .insert_webhook_out_endpoint(domain)
            .await
            .map(endpoint::to_proto)
            .map_err(Into::<WebhookApiError>::into)?;

        Ok(Response::new(CreateWebhookEndpointResponse {
            endpoint: Some(endpoint),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn get_webhook_endpoint(
        &self,
        request: Request<GetWebhookEndpointRequest>,
    ) -> Result<Response<GetWebhookEndpointResponse>, Status> {
        let tenant_id = request.tenant()?;

        let req = request.into_inner();

        let endpoint = self
            .store
            .get_webhook_out_endpoint(tenant_id, req.id)
            .await
            .map(endpoint::to_proto)
            .map_err(Into::<WebhookApiError>::into)?;

        Ok(Response::new(GetWebhookEndpointResponse {
            endpoint: Some(endpoint),
        }))
    }

    #[tracing::instrument(skip_all)]
    async fn list_webhook_endpoints(
        &self,
        request: Request<ListWebhookEndpointsRequest>,
    ) -> Result<Response<ListWebhookEndpointsResponse>, Status> {
        let tenant_id = request.tenant()?;

        let inner = request.into_inner();

        let page = self
            .store
            .list_webhook_out_endpoints(
                tenant_id,
                Some(endpoint::list_request_to_domain_filter(inner)),
            )
            .await
            .map(endpoint::page_to_proto)
            .map_err(Into::<WebhookApiError>::into)?;

        Ok(Response::new(page))
    }
}
