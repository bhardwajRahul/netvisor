use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::base::Host,
    ip_addresses::r#impl::base::IPAddress,
    services::r#impl::virtualization::ServiceVirtualization,
    shared::entities::EntityDiscriminants,
    subnets::r#impl::types::SubnetType,
    topology::{
        service::{
            anchor_planner::ChildAnchorPlanner,
            context::TopologyContext,
            element_rules::{ElementMatchData, apply_element_rules},
        },
        types::{
            edges::Edge,
            grouping::GroupingConfig,
            layout::{Ixy, Uxy},
            nodes::{ContainerChild, ContainerType, ElementEntityType, Node, NodeType},
        },
    },
};

pub struct GraphBuilder {
    consolidated_docker_subnets: HashMap<Uuid, Vec<Uuid>>,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            consolidated_docker_subnets: HashMap::new(),
        }
    }

    /// Compute which subnet ID each host's Docker bridges should be grouped under.
    /// Returns a map of host_id → primary_subnet_id (first DockerBridge subnet found).
    fn compute_docker_bridge_grouping(ctx: &TopologyContext) -> HashMap<Uuid, Uuid> {
        let mut mapping: HashMap<Uuid, Uuid> = HashMap::new();
        for ip_address in ctx.ip_addresses {
            let Some(subnet) = ctx.get_subnet_by_id(ip_address.base.subnet_id) else {
                continue;
            };
            if subnet.base.subnet_type == SubnetType::DockerBridge {
                mapping.entry(ip_address.base.host_id).or_insert(subnet.id);
            }
        }
        mapping
    }

    /// Main entry point: group children by subnet and create all child nodes
    pub fn create_subnet_child_nodes(
        &mut self,
        ctx: &TopologyContext,
        all_edges: &mut [Edge],
        grouping: &GroupingConfig,
    ) -> (HashSet<Uuid>, Vec<Node>) {
        let docker_bridge_host_subnet_id_to_group_on = if grouping.should_group_docker_bridges() {
            Self::compute_docker_bridge_grouping(ctx)
        } else {
            HashMap::new()
        };
        let children_by_subnet = self.group_children_by_subnet(
            ctx,
            all_edges,
            grouping,
            docker_bridge_host_subnet_id_to_group_on,
        );
        let mut child_nodes = Vec::new();

        let subnet_ids: HashSet<Uuid> = children_by_subnet
            .iter()
            .map(|(subnet_id, children)| {
                self.create_child_nodes(*subnet_id, children, ctx, &mut child_nodes);
                *subnet_id
            })
            .collect();

        (subnet_ids, child_nodes)
    }

    fn determine_subnet_child_header_text(
        &self,
        ctx: &TopologyContext,
        ip_address: &IPAddress,
        host: &Host,
        subnet_type: &SubnetType,
    ) -> Option<String> {
        let host_interfaces = ctx.get_ip_addresses_for_host(host.id);
        let host_has_name = host.base.name != "Unknown Device" && !host.base.name.is_empty();

        // P1: Docker containers — always show "Docker @", never VM header
        if *subnet_type == SubnetType::DockerBridge {
            let header_text = if host_has_name {
                Some("Docker @ ".to_owned() + &host.base.name.clone())
            } else {
                // Generate a label from non-docker ip_address, if there is one
                host_interfaces
                    .iter()
                    .find(|i| {
                        ctx.get_subnet_from_ip_address_id(i.id)
                            .map(|s| s.base.subnet_type != SubnetType::DockerBridge)
                            .unwrap_or(false)
                    })
                    .map(|i| "Docker @ ".to_owned() + &i.base.ip_address.to_string())
            };

            return header_text;
        }

        // P2: Virtualized hosts — show the VM's own hostname
        // (VM status is indicated via colored text in the frontend)
        if ctx.get_host_is_virtualized_by(&host.id).is_some() && host_has_name {
            return Some(host.base.name.clone());
        }

        // P3: Show host if it differs from the first service name + isn't shown via interface edges
        // and if it also isn't just the interface IP
        let host_services = ctx.get_services_for_host(host.id);
        let first_service_name_matches_host_name = match host_services.first() {
            Some(first_service) => first_service.base.name == host.base.name,
            None => false,
        };

        let host_name_is_interface_ip = ip_address.base.ip_address.to_string() == host.base.name;

        // Count of other ip_addresses that will actually have a node (ie services on that interface > 0)
        // so an interface edge will be created
        let ip_addresses_with_node: Vec<&&IPAddress> = host_interfaces
            .iter()
            .filter(|i| !ctx.get_services_bound_to_ip_address(i.id).is_empty())
            .collect();

        if !host_name_is_interface_ip
            && !first_service_name_matches_host_name
            && host_has_name
            && ip_addresses_with_node.len() < 2
        {
            return Some(host.base.name.clone());
        }

        None
    }

    /// Group host ip_addresses by subnet
    /// If group_docker_bridges_by_host is true, all DockerBridge ip_addresses for a given host
    /// are consolidated into one subnet
    fn group_children_by_subnet(
        &mut self,
        ctx: &TopologyContext,
        all_edges: &mut [Edge],
        grouping: &GroupingConfig,
        docker_bridge_host_subnet_id_to_group_on: HashMap<Uuid, Uuid>,
    ) -> HashMap<Uuid, Vec<ContainerChild>> {
        let mut children_by_subnet: HashMap<Uuid, Vec<ContainerChild>> = HashMap::new();

        // Track DockerBridge ip_addresses by host (only used if grouping is enabled)
        // Map: (host_id, primary_subnet_id) -> Vec<subnet_id>)
        let mut docker_subnets_by_host: HashMap<(Uuid, Uuid), Vec<Uuid>> = HashMap::new();

        for ip_address in ctx.ip_addresses {
            let Some(host) = ctx.get_host_by_id(ip_address.base.host_id) else {
                continue;
            };
            let subnet = ctx.get_subnet_by_id(ip_address.base.subnet_id);
            if subnet
                .map(|s| s.base.subnet_type.exclude_from_topology())
                .unwrap_or(false)
            {
                continue;
            }
            let subnet_type = subnet.map(|s| s.base.subnet_type).unwrap_or_default();

            // Update source/target handles for edges
            let edges = ChildAnchorPlanner::plan_anchors(ip_address.id, all_edges, ctx);

            let header_text =
                self.determine_subnet_child_header_text(ctx, ip_address, host, &subnet_type);

            let child = ContainerChild {
                id: ip_address.id,
                host_id: host.id,
                size: Uxy::default(),
                header: header_text,
                ip_address_id: Some(ip_address.id),
                edges,
            };

            // Special handling for DockerBridge (only if grouping is enabled)
            if grouping.should_group_docker_bridges()
                && matches!(subnet_type, SubnetType::DockerBridge)
            {
                if let Some(subnet_grouping_id) =
                    docker_bridge_host_subnet_id_to_group_on.get(&host.id)
                {
                    docker_subnets_by_host
                        .entry((host.id, *subnet_grouping_id))
                        .or_default()
                        .push(ip_address.base.subnet_id);

                    children_by_subnet
                        .entry(*subnet_grouping_id)
                        .or_default()
                        .push(child);
                }
            } else {
                children_by_subnet
                    .entry(ip_address.base.subnet_id)
                    .or_default()
                    .push(child);
            }
        }

        // Consolidate all DockerBridge children into their primary subnet (only if grouping is enabled)
        if grouping.should_group_docker_bridges() {
            for ((_, grouping_id), mut subnet_ids) in docker_subnets_by_host {
                // Remove duplicates and sort for consistency
                subnet_ids.sort();
                subnet_ids.dedup();

                // Store the consolidation mapping
                self.consolidated_docker_subnets
                    .insert(grouping_id, subnet_ids);
            }
        }

        children_by_subnet
    }

    /// Create child (element) nodes for a subnet
    fn create_child_nodes(
        &mut self,
        subnet_id: Uuid,
        children: &[ContainerChild],
        ctx: &TopologyContext,
        child_nodes: &mut Vec<Node>,
    ) {
        // Create element nodes for all children
        // Positions are zeroed — the frontend computes layout via elkjs
        for child in children.iter() {
            let mut node = Node::element(
                child.id,
                subnet_id,
                child.host_id,
                ElementEntityType::IPAddress {
                    subnet_id,
                    ip_address_id: child.ip_address_id,
                },
            );
            node.size = child.size;
            node.header = child.header.clone();
            child_nodes.push(node);
        }

        // Create nested group containers for ByServiceCategory and ByTag rules
        self.create_nested_group_containers(subnet_id, children, ctx, child_nodes);
    }

    /// Create nested Container nodes for ByServiceCategory and ByTag grouping rules (ClientSide mode only).
    /// First-match-wins: nodes already claimed by an earlier rule are not reassigned.
    fn create_nested_group_containers(
        &self,
        _subnet_id: Uuid,
        children: &[ContainerChild],
        ctx: &TopologyContext,
        child_nodes: &mut Vec<Node>,
    ) {
        let grouping = GroupingConfig::from_request_options(&ctx.options.request);
        let children_by_id: HashMap<Uuid, &ContainerChild> =
            children.iter().map(|c| (c.id, c)).collect();

        let _ = apply_element_rules(child_nodes, &grouping.element_rules, |node| {
            let child = children_by_id.get(&node.id)?;
            let categories = ctx
                .services
                .iter()
                .filter(|s| s.base.host_id == child.host_id)
                .map(|s| s.base.service_definition.category())
                .collect();
            let mut tag_ids: HashSet<Uuid> = ctx
                .hosts
                .iter()
                .find(|h| h.id == child.host_id)
                .map(|h| h.base.tags.iter().copied().collect())
                .unwrap_or_default();
            // Inherit service tags so ByTag rules match via host→interface
            for service in ctx
                .services
                .iter()
                .filter(|s| s.base.host_id == child.host_id)
            {
                tag_ids.extend(service.base.tags.iter().copied());
            }
            // Resolve compose_project only for elements inside Docker subnets.
            // LAN ip_addresses shouldn't be grouped by stack.
            let is_docker_subnet = ctx
                .subnets
                .iter()
                .find(|s| s.id == _subnet_id)
                .map(|s| s.base.subnet_type.is_container_network())
                .unwrap_or(false);
            let compose_project = if !is_docker_subnet {
                None
            } else {
                let mut projects: HashSet<&str> = HashSet::new();
                let services_iter: Box<dyn Iterator<Item = _>> =
                    if let Some(iface_id) = child.ip_address_id {
                        // Interface-specific: only services bound to this interface
                        Box::new(ctx.services.iter().filter(move |s| {
                            s.base.host_id == child.host_id
                                && s.base
                                    .bindings
                                    .iter()
                                    .any(|b| b.ip_address_id() == Some(iface_id))
                        }))
                    } else {
                        // Fallback: all services on the host
                        Box::new(
                            ctx.services
                                .iter()
                                .filter(|s| s.base.host_id == child.host_id),
                        )
                    };
                for service in services_iter {
                    if let Some(ServiceVirtualization::Docker(dv)) = &service.base.virtualization
                        && let Some(ref project) = dv.compose_project
                    {
                        projects.insert(project.as_str());
                    }
                }
                if projects.len() == 1 {
                    projects.into_iter().next().map(String::from)
                } else {
                    None
                }
            };
            Some(ElementMatchData {
                categories,
                tag_ids,
                element_entity: EntityDiscriminants::IPAddress,
                virtualizer_service_id: None,
                compose_project,
                native_vlan_id: None,
                vlan_number: None,
                vlan_name: None,
                is_trunk_port: false,
                oper_status: None,
            })
        }, None, None);

        // Post-process: set associated_service_definition on Stack subcontainers (always Docker)
        for node in child_nodes.iter_mut() {
            if let NodeType::Container {
                container_type: ContainerType::Stack,
                associated_service_definition,
                ..
            } = &mut node.node_type
            {
                *associated_service_definition = Some("Docker".to_string());
            }
        }
    }

    /// Create subnet container nodes
    /// Positions and sizes are zeroed — the frontend computes layout via elkjs
    pub fn create_subnet_nodes(
        &self,
        ctx: &TopologyContext,
        subnet_ids: &HashSet<Uuid>,
    ) -> Vec<Node> {
        subnet_ids
            .iter()
            .map(|subnet_id| {
                // Build display header from subnet metadata
                let header = if let Some(cids) = self.consolidated_docker_subnets.get(subnet_id) {
                    Some(
                        "Docker Bridge: (".to_owned()
                            + &ctx
                                .subnets
                                .iter()
                                .filter(|s| cids.contains(&s.id))
                                .map(|s| s.base.cidr.to_string())
                                .join(", ")
                            + ")",
                    )
                } else if let Some(subnet) = ctx.subnets.iter().find(|s| s.id == *subnet_id) {
                    use crate::server::shared::types::metadata::TypeMetadataProvider;
                    let type_name = subnet.base.subnet_type.name();
                    let cidr = subnet.base.cidr.to_string();
                    let show_label = subnet.base.subnet_type.show_label();
                    let name_or_type = if subnet.base.name != cidr {
                        subnet.base.name.clone()
                    } else if show_label {
                        type_name.to_string()
                    } else {
                        String::new()
                    };
                    Some(if name_or_type.is_empty() {
                        cidr
                    } else {
                        format!("{}: {}", name_or_type, cidr)
                    })
                } else {
                    None
                };

                let will_accept_edges = self.consolidated_docker_subnets.contains_key(subnet_id);
                Node {
                    id: *subnet_id,
                    node_type: NodeType::Container {
                        container_type: ContainerType::Subnet,
                        parent_container_id: None,
                        entity_id: Some(*subnet_id),
                        layer_hint: None,
                        icon: None,
                        color: None,
                        associated_service_definition: None,
                    },
                    position: Ixy { x: 0, y: 0 },
                    size: Uxy { x: 0, y: 0 },
                    header,
                    element_rule_id: None,
                    will_accept_edges,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::services::r#impl::base::{Service, ServiceBase};
    use crate::server::services::r#impl::categories::ServiceCategory;
    use crate::server::services::r#impl::definitions::ServiceDefinition;
    use crate::server::services::r#impl::patterns::Pattern;
    use crate::server::shared::types::Color;
    use crate::server::tags::r#impl::base::{Tag, TagBase};
    use crate::server::topology::service::context::TopologyContext;
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::grouping::{ElementRule, IdentifiedRule};
    use chrono::Utc;

    /// Test service definition that returns ReverseProxy category
    #[derive(PartialEq, Eq, Hash, Clone)]
    struct ReverseProxyServiceDef;

    impl ServiceDefinition for ReverseProxyServiceDef {
        fn name(&self) -> &'static str {
            "TestReverseProxy"
        }
        fn description(&self) -> &'static str {
            "Test"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::ReverseProxy
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    fn make_host(name: &str, tags: Vec<Uuid>) -> Host {
        Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: name.to_string(),
                tags,
                ..Default::default()
            },
        }
    }

    fn make_service(host_id: Uuid, def: Box<dyn ServiceDefinition>) -> Service {
        make_service_with_tags(host_id, def, vec![])
    }

    fn make_service_with_tags(
        host_id: Uuid,
        def: Box<dyn ServiceDefinition>,
        tags: Vec<Uuid>,
    ) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: def,
                tags,
                ..Default::default()
            },
        }
    }

    fn make_tag(name: &str) -> Tag {
        Tag {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: name.to_string(),
                description: None,
                color: Color::Yellow,
                organization_id: Uuid::new_v4(),
                is_application: false,
            },
        }
    }

    fn make_element_node(id: Uuid, host_id: Uuid, container_id: Uuid) -> Node {
        let mut node = Node::element(
            id,
            container_id,
            host_id,
            ElementEntityType::IPAddress {
                subnet_id: container_id,
                ip_address_id: Some(id),
            },
        );
        node.size = Uxy { x: 100, y: 50 };
        node
    }

    fn make_container_child(id: Uuid, host_id: Uuid) -> ContainerChild {
        ContainerChild {
            id,
            host_id,
            ip_address_id: Some(id),
            header: None,
            size: Uxy { x: 100, y: 50 },
            edges: vec![],
        }
    }

    #[test]
    fn test_nested_group_first_match_wins() {
        let tag = make_tag("MyTag");
        // Host that matches both ByServiceCategory(ReverseProxy) AND ByTag(tag)
        let host_both = make_host("host-both", vec![tag.id]);
        // Host that only matches the tag
        let host_tag_only = make_host("host-tag-only", vec![tag.id]);

        let svc = make_service(host_both.id, Box::new(ReverseProxyServiceDef));

        let subnet_id = Uuid::new_v4();
        let child_both_id = Uuid::new_v4();
        let child_tag_id = Uuid::new_v4();

        let children = vec![
            make_container_child(child_both_id, host_both.id),
            make_container_child(child_tag_id, host_tag_only.id),
        ];

        let mut child_nodes = vec![
            make_element_node(child_both_id, host_both.id, subnet_id),
            make_element_node(child_tag_id, host_tag_only.id, subnet_id),
        ];

        // Rules: ByServiceCategory first, then ByTag
        // Use Application view so ByServiceCategory is applicable
        let mut options = TopologyOptions::default();
        options.request.view = crate::server::topology::types::views::TopologyView::Application;
        options.request.element_rules = vec![
            IdentifiedRule::new(ElementRule::ByServiceCategory {
                categories: vec![ServiceCategory::ReverseProxy],
                title: Some("Infra".to_string()),
                is_infra_rule: false,
            }),
            IdentifiedRule::new(ElementRule::ByTag {
                tag_ids: vec![tag.id],
                title: None,
            }),
        ];

        let hosts = vec![host_both.clone(), host_tag_only.clone()];
        let services = vec![svc];
        let tags = vec![tag.clone()];

        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
        );

        let planner = GraphBuilder::new();
        planner.create_nested_group_containers(subnet_id, &children, &ctx, &mut child_nodes);

        // Find group containers
        let groups: Vec<&Node> = child_nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        assert_eq!(groups.len(), 2, "Should create two group containers");

        let cat_group = groups
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedServiceCategory,
                        ..
                    }
                )
            })
            .expect("Should have NestedServiceCategory");

        let tag_group = groups
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .expect("Should have NestedTag");

        // First-match-wins: host_both should be in the category group (first rule)
        let both_node = child_nodes.iter().find(|n| n.id == child_both_id).unwrap();
        if let NodeType::Element { container_id, .. } = &both_node.node_type {
            assert_eq!(
                *container_id, cat_group.id,
                "Overlapping host should be in first-match container (NestedServiceCategory)"
            );
        }

        // host_tag_only should be in the tag group (only matches tag rule)
        let tag_node = child_nodes.iter().find(|n| n.id == child_tag_id).unwrap();
        if let NodeType::Element { container_id, .. } = &tag_node.node_type {
            assert_eq!(
                *container_id, tag_group.id,
                "Tag-only host should be in NestedTag"
            );
        }

        // Verify headers contain custom titles
        assert_eq!(
            cat_group.header.as_deref(),
            Some("Infra"),
            "Category group header should be custom title"
        );
        assert!(
            tag_group.header.is_none(),
            "Tag group with no custom title should have no header"
        );

        // Verify element_rule_id is set
        assert!(
            cat_group.element_rule_id.is_some(),
            "Category group should have element_rule_id"
        );
        assert!(
            tag_group.element_rule_id.is_some(),
            "Tag group should have element_rule_id"
        );
    }

    #[test]
    fn test_nested_group_reversed_order_flips_priority() {
        let tag = make_tag("TestTag");
        let host = make_host("overlap-host", vec![tag.id]);
        let svc = make_service(host.id, Box::new(ReverseProxyServiceDef));

        let subnet_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let children = vec![make_container_child(child_id, host.id)];

        // This time: ByTag FIRST, then ByServiceCategory
        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![
            IdentifiedRule::new(ElementRule::ByTag {
                tag_ids: vec![tag.id],
                title: Some("Tagged".to_string()),
            }),
            IdentifiedRule::new(ElementRule::ByServiceCategory {
                categories: vec![ServiceCategory::ReverseProxy],
                title: Some("Infra".to_string()),
                is_infra_rule: false,
            }),
        ];

        let hosts = vec![host.clone()];
        let services = vec![svc];
        let tags = vec![tag.clone()];

        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
        );

        let mut child_nodes = vec![make_element_node(child_id, host.id, subnet_id)];

        let planner = GraphBuilder::new();
        planner.create_nested_group_containers(subnet_id, &children, &ctx, &mut child_nodes);

        // Find the tag group (should be first match now)
        let tag_group = child_nodes
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .expect("Should have NestedTag");

        // Host should be in tag group (first rule wins)
        let element = child_nodes.iter().find(|n| n.id == child_id).unwrap();
        if let NodeType::Element { container_id, .. } = &element.node_type {
            assert_eq!(
                *container_id, tag_group.id,
                "When ByTag is first, overlapping host should be in NestedTag"
            );
        }

        // NestedServiceCategory should not be created when all its matches are already claimed
        // (since the only matching host was claimed by tag rule)
        let cat_group = child_nodes.iter().find(|n| {
            matches!(
                n.node_type,
                NodeType::Container {
                    container_type: ContainerType::NestedServiceCategory,
                    ..
                }
            )
        });
        assert!(
            cat_group.is_none(),
            "NestedServiceCategory should not be created when all its matches are already claimed"
        );
    }

    #[test]
    fn test_bytag_inherits_service_tags_in_l3() {
        // Tag assigned to SERVICE only, not to the host
        let tag = make_tag("ServiceTag");
        let host = make_host("host-no-tags", vec![]); // Host has NO tags
        let svc = make_service_with_tags(host.id, Box::new(ReverseProxyServiceDef), vec![tag.id]);

        let subnet_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let children = vec![make_container_child(child_id, host.id)];
        let mut child_nodes = vec![make_element_node(child_id, host.id, subnet_id)];

        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![tag.id],
            title: Some("ServiceTagGroup".to_string()),
        })];

        let hosts = vec![host.clone()];
        let services = vec![svc];
        let tags = vec![tag.clone()];

        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
        );

        let planner = GraphBuilder::new();
        planner.create_nested_group_containers(subnet_id, &children, &ctx, &mut child_nodes);

        // Should create a NestedTag container via service tag inheritance
        let tag_group = child_nodes
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .expect("Should create NestedTag from service tag inheritance");

        // The interface element should be grouped under the tag container
        let element = child_nodes.iter().find(|n| n.id == child_id).unwrap();
        if let NodeType::Element { container_id, .. } = &element.node_type {
            assert_eq!(
                *container_id, tag_group.id,
                "IP address should be grouped by service tag inheritance"
            );
        }

        assert_eq!(
            tag_group.header.as_deref(),
            Some("ServiceTagGroup"),
            "Tag group should have custom title"
        );
    }
}
