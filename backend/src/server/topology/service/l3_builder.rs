use std::collections::HashMap;
use uuid::Uuid;

use super::{
    context::TopologyContext, edge_builder::EdgeBuilder, subnet_graph_builder::SubnetGraphBuilder,
    view::ViewBuilder,
};
use crate::server::shared::types::metadata::EntityMetadataProvider;
use crate::server::topology::types::{
    edges::Edge,
    grouping::GroupingConfig,
    nodes::{Node, NodeType},
};

pub struct L3Builder;

impl ViewBuilder for L3Builder {
    fn build(&self, ctx: &TopologyContext, grouping: &GroupingConfig) -> (Vec<Node>, Vec<Edge>) {
        // Create all edges
        let mut all_edges = Vec::new();

        all_edges.extend(EdgeBuilder::create_interface_edges(ctx));
        all_edges.extend(EdgeBuilder::create_dependency_edges(ctx));
        all_edges.extend(EdgeBuilder::create_vm_host_edges(ctx));
        all_edges.extend(EdgeBuilder::create_containerized_service_edges(ctx));
        all_edges.extend(EdgeBuilder::create_physical_link_edges(ctx));

        // Create nodes (positions zeroed — frontend computes layout via elkjs)
        let mut subnet_builder = SubnetGraphBuilder::new();
        let (subnet_ids, child_nodes) =
            subnet_builder.create_subnet_child_nodes(ctx, &mut all_edges, grouping);

        let mut subnet_nodes = subnet_builder.create_subnet_nodes(ctx, &subnet_ids);

        // Set icon and color on container nodes from subnet metadata
        let subnet_map: HashMap<Uuid, &crate::server::subnets::r#impl::types::SubnetType> = ctx
            .subnets
            .iter()
            .map(|s| (s.id, &s.base.subnet_type))
            .collect();
        for node in &mut subnet_nodes {
            if let NodeType::Container {
                ref mut icon,
                ref mut color,
                ..
            } = node.node_type
                && let Some(subnet_type) = subnet_map.get(&node.id)
            {
                *icon = Some(subnet_type.icon().to_string());
                *color = Some(subnet_type.color().to_string());
            }
        }

        let all_nodes: Vec<Node> = subnet_nodes.into_iter().chain(child_nodes).collect();
        (all_nodes, all_edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
    use crate::server::services::r#impl::base::{Service, ServiceBase};
    use crate::server::services::r#impl::categories::ServiceCategory;
    use crate::server::services::r#impl::definitions::ServiceDefinition;
    use crate::server::services::r#impl::patterns::Pattern;
    use crate::server::shared::types::Color;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::tags::r#impl::base::{Tag, TagBase};
    use crate::server::topology::service::context::TopologyContext;
    use crate::server::topology::service::view::ViewBuilder;
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::grouping::{ElementRule, IdentifiedRule};
    use crate::server::topology::types::nodes::{ContainerType, NodeType};
    use chrono::Utc;
    use cidr::{IpCidr, Ipv4Cidr};
    use std::net::Ipv4Addr;

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct TestServiceDef;

    impl ServiceDefinition for TestServiceDef {
        fn name(&self) -> &'static str {
            "TestService"
        }
        fn description(&self) -> &'static str {
            "Test"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::Development
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    #[test]
    fn test_l3_bytag_ignores_service_tags() {
        // Regression: L3Builder used to match IPAddress elements on service
        // tags via inheritance. Now only the host's own tags count.
        let network_id = Uuid::new_v4();
        let subnet_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let ip_address_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();

        let host = Host {
            id: host_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: "test-host".to_string(),
                network_id,
                tags: vec![], // NO host tags
                ..Default::default()
            },
            ..Default::default()
        };

        let subnet = Subnet {
            id: subnet_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap()),
                network_id,
                name: "test-subnet".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let ip_address = IPAddress {
            id: ip_address_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: IPAddressBase {
                network_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                ..Default::default()
            },
            ..Default::default()
        };

        let service = Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                network_id,
                service_definition: Box::new(TestServiceDef),
                tags: vec![tag_id], // Service HAS the tag
                ..Default::default()
            },
            ..Default::default()
        };

        let tag = Tag {
            id: tag_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: "ServiceTag".to_string(),
                description: None,
                color: Color::Orange,
                organization_id: Uuid::new_v4(),
                is_application: false,
            },
            ..Default::default()
        };

        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![tag_id],
            title: Some("Tagged".to_string()),
        })];

        let hosts = vec![host];
        let ip_addresses = vec![ip_address];
        let subnets = vec![subnet];
        let services = vec![service];
        let tags = vec![tag];

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let grouping = GroupingConfig::from_request_options(
            &ctx.options.request,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );
        let builder = L3Builder;
        let (nodes, _edges) = builder.build(&ctx, &grouping);

        // ByTag on a tag applied only to a service (not the host) should NOT
        // create a NestedTag container — IP addresses resolve their tag target
        // to the owning Host, and the host has no tags.
        assert!(
            !nodes.iter().any(|n| matches!(
                n.node_type,
                NodeType::Container {
                    container_type: ContainerType::NestedTag,
                    ..
                }
            )),
            "ByTag should not match IP addresses on service-only tags"
        );
    }
}
