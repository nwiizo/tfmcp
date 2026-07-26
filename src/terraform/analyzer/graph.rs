//! Resource dependency graph construction.

use crate::terraform::model::core::TerraformAnalysis;
use crate::terraform::model::graph::{
    DependencyType, ModuleBoundary, ResourceDependencyGraph, ResourceEdge, ResourceNode,
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static REFERENCE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(aws_[a-z_]+|azurerm_[a-z_]+|google_[a-z_]+|kubernetes_[a-z_]+)\.([a-z_0-9]+)"#)
        .expect("Invalid reference regex")
});

static DEPENDS_ON_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"depends_on\s*=\s*\[([^\]]+)\]"#).expect("Invalid depends_on regex")
});

/// Build resource dependency graph
pub fn build_dependency_graph(
    analysis: &TerraformAnalysis,
    file_contents: &HashMap<String, String>,
) -> ResourceDependencyGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Build resource lookup map
    let mut resource_map: HashMap<String, usize> = HashMap::new();

    // Create nodes for each resource
    for resource in &analysis.resources {
        let id = format!("{}.{}", resource.resource_type, resource.name);
        resource_map.insert(id.clone(), nodes.len());

        nodes.push(ResourceNode {
            id: id.clone(),
            resource_type: resource.resource_type.clone(),
            resource_name: resource.name.clone(),
            module_path: analysis.project_directory.clone(),
            file: resource.file.clone(),
            provider: resource.provider.clone(),
        });
    }

    // Find dependencies by scanning file contents
    for (filename, content) in file_contents {
        // Find explicit depends_on
        for cap in DEPENDS_ON_REGEX.captures_iter(content) {
            let deps_str = &cap[1];
            for dep in deps_str.split(',') {
                let dep_trimmed = dep.trim().trim_matches(|c| c == '[' || c == ']');
                if resource_map.contains_key(dep_trimmed) {
                    // Find which resource this depends_on belongs to
                    // This is simplified - in practice we'd need more context
                    for node in &nodes {
                        if content.contains(&format!("resource \"{}\"", node.resource_type)) {
                            edges.push(ResourceEdge {
                                source: node.id.clone(),
                                target: dep_trimmed.to_string(),
                                dependency_type: DependencyType::Explicit,
                                attribute: Some("depends_on".to_string()),
                            });
                            break;
                        }
                    }
                }
            }
        }

        // Find implicit references
        for cap in REFERENCE_REGEX.captures_iter(content) {
            let ref_type = &cap[1];
            let ref_name = &cap[2];
            let ref_id = format!("{ref_type}.{ref_name}");

            if resource_map.contains_key(&ref_id) {
                // Find the resource that contains this reference
                for resource in &analysis.resources {
                    if resource.file == *filename {
                        let source_id = format!("{}.{}", resource.resource_type, resource.name);
                        if source_id != ref_id {
                            // Avoid duplicate edges
                            let edge_exists = edges.iter().any(|e| {
                                e.source == source_id
                                    && e.target == ref_id
                                    && matches!(e.dependency_type, DependencyType::Implicit)
                            });

                            if !edge_exists {
                                edges.push(ResourceEdge {
                                    source: source_id,
                                    target: ref_id.clone(),
                                    dependency_type: DependencyType::Implicit,
                                    attribute: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Create module boundary
    let module_boundaries = vec![ModuleBoundary {
        module_path: analysis.project_directory.clone(),
        resource_ids: nodes.iter().map(|n| n.id.clone()).collect(),
    }];

    ResourceDependencyGraph {
        nodes,
        edges,
        module_boundaries,
    }
}
