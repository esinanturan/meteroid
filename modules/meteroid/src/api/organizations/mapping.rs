pub mod organization {
    use super::super::super::tenants::mapping::tenants as tenants_mapping;
    use crate::api::shared::conversions::ProtoConv;
    use meteroid_grpc::meteroid::api::organizations::v1 as server;
    use meteroid_store::domain;

    pub fn domain_to_proto(domain: domain::Organization) -> server::Organization {
        server::Organization {
            id: domain.id.as_proto(),
            slug: domain.slug,
            created_at: domain.created_at.as_proto(),
            trade_name: domain.trade_name,
        }
    }

    pub fn domain_with_tenants_to_proto(
        domain: domain::OrganizationWithTenants,
    ) -> server::OrganizationWithTenant {
        server::OrganizationWithTenant {
            id: domain.organization.id.as_proto(),
            slug: domain.organization.slug,
            created_at: domain.organization.created_at.as_proto(),
            trade_name: domain.organization.trade_name,
            tenants: domain
                .tenants
                .into_iter()
                .map(tenants_mapping::domain_to_server)
                .collect(),
        }
    }
}
