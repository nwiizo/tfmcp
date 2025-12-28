//! Terraform graph output for dependency visualization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Graph node representing a resource or module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: GraphNodeType,
    pub provider: Option<String>,
}

/// Type of graph node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeType {
    Resource,
    DataSource,
    Module,
    Provider,
    Variable,
    Output,
    Root,
}

/// Graph edge representing a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: GraphEdgeType,
}

/// Type of graph edge
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphEdgeType {
    DependsOn,
    Reference,
    Provider,
    Module,
}

/// Complete graph result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub dot_output: String,
    pub statistics: GraphStatistics,
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_nodes: i32,
    pub total_edges: i32,
    pub resource_count: i32,
    pub data_source_count: i32,
    pub module_count: i32,
    pub provider_count: i32,
    pub max_depth: i32,
}

/// Graph type filter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphType {
    Plan,
    Apply,
}

impl std::str::FromStr for GraphType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "plan" => Ok(GraphType::Plan),
            "apply" => Ok(GraphType::Apply),
            _ => Err(anyhow::anyhow!(
                "Unknown graph type: {}. Valid types: plan, apply",
                s
            )),
        }
    }
}

/// Generate terraform graph
pub fn generate_graph(
    terraform_path: &Path,
    project_dir: &Path,
    graph_type: Option<GraphType>,
) -> anyhow::Result<TerraformGraph> {
    let mut cmd = Command::new(terraform_path);
    cmd.arg("graph");

    // Add type filter if specified
    if let Some(gt) = &graph_type {
        match gt {
            GraphType::Plan => cmd.arg("-type=plan"),
            GraphType::Apply => cmd.arg("-type=apply"),
        };
    }

    let output = cmd.current_dir(project_dir).output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to generate graph: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let dot_output = String::from_utf8_lossy(&output.stdout).to_string();

    // Parse DOT output to extract nodes and edges
    let (nodes, edges) = parse_dot_output(&dot_output);

    // Calculate statistics
    let statistics = calculate_statistics(&nodes, &edges);

    Ok(TerraformGraph {
        nodes,
        edges,
        dot_output,
        statistics,
    })
}

/// Parse DOT format output from terraform graph
fn parse_dot_output(dot: &str) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut node_map: HashMap<String, GraphNode> = HashMap::new();

    for line in dot.lines() {
        let line = line.trim();

        // Skip empty lines and graph declarations
        if line.is_empty()
            || line.starts_with("digraph")
            || line.starts_with('}')
            || line.starts_with("compound")
            || line.starts_with("newrank")
            || line.starts_with("subgraph")
        {
            continue;
        }

        // Parse edges (format: "node1" -> "node2")
        if line.contains("->") {
            if let Some((from, to)) = parse_edge_line(line) {
                edges.push(GraphEdge {
                    from: from.clone(),
                    to: to.clone(),
                    edge_type: determine_edge_type(&from, &to),
                });
            }
        }
        // Parse node definitions
        else if line.contains('[') && line.contains(']') {
            if let Some(node) = parse_node_line(line) {
                node_map.insert(node.id.clone(), node);
            }
        }
        // Simple node (just the name in quotes)
        else if line.starts_with('"') && line.ends_with('"') {
            let id = line.trim_matches('"').to_string();
            node_map
                .entry(id.clone())
                .or_insert_with(|| create_node_from_id(&id));
        }
    }

    nodes.extend(node_map.into_values());
    (nodes, edges)
}

/// Parse an edge line from DOT format
fn parse_edge_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.split("->").collect();
    if parts.len() != 2 {
        return None;
    }

    let from = parts[0]
        .trim()
        .trim_matches('"')
        .trim_matches(';')
        .to_string();
    let to = parts[1]
        .trim()
        .trim_matches('"')
        .trim_matches(';')
        .split('[')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .to_string();

    if from.is_empty() || to.is_empty() {
        return None;
    }

    Some((from, to))
}

/// Parse a node definition line from DOT format
fn parse_node_line(line: &str) -> Option<GraphNode> {
    // Extract node ID (between first set of quotes)
    let id_start = line.find('"')?;
    let id_end = line[id_start + 1..].find('"')? + id_start + 1;
    let id = line[id_start + 1..id_end].to_string();

    // Extract label if present
    let label = if let Some(label_start) = line.find("label") {
        let label_content = &line[label_start..];
        if let Some(start) = label_content.find('"') {
            if let Some(end) = label_content[start + 1..].find('"') {
                label_content[start + 1..start + 1 + end].to_string()
            } else {
                id.clone()
            }
        } else {
            id.clone()
        }
    } else {
        id.clone()
    };

    Some(create_node_from_id_with_label(&id, &label))
}

/// Create a node from its ID
fn create_node_from_id(id: &str) -> GraphNode {
    create_node_from_id_with_label(id, id)
}

/// Create a node from its ID and label
fn create_node_from_id_with_label(id: &str, label: &str) -> GraphNode {
    let (node_type, provider) = determine_node_type(id);

    GraphNode {
        id: id.to_string(),
        label: label.to_string(),
        node_type,
        provider,
    }
}

/// Determine node type from ID
fn determine_node_type(id: &str) -> (GraphNodeType, Option<String>) {
    if id.starts_with("[root]") || id == "root" {
        return (GraphNodeType::Root, None);
    }

    if id.starts_with("provider[") || id.starts_with("provider.") {
        let provider_name = id
            .split('/')
            .next_back()
            .unwrap_or(id)
            .trim_end_matches(']')
            .trim_end_matches('"')
            .to_string();
        return (GraphNodeType::Provider, Some(provider_name));
    }

    if id.starts_with("module.") {
        return (GraphNodeType::Module, None);
    }

    if id.starts_with("var.") {
        return (GraphNodeType::Variable, None);
    }

    if id.starts_with("output.") {
        return (GraphNodeType::Output, None);
    }

    if id.starts_with("data.") {
        let provider = extract_provider_from_resource(id);
        return (GraphNodeType::DataSource, provider);
    }

    // Must be a resource
    let provider = extract_provider_from_resource(id);
    (GraphNodeType::Resource, provider)
}

/// Extract provider name from resource ID
fn extract_provider_from_resource(id: &str) -> Option<String> {
    // Resources are typically named like "aws_instance.example" or "[root] aws_instance.example"
    let resource_type = id
        .trim_start_matches("[root]")
        .trim_start_matches("data.")
        .trim()
        .split('.')
        .next()?;

    // Extract provider from resource type (e.g., "aws" from "aws_instance")
    let provider = resource_type.split('_').next()?;
    Some(provider.to_string())
}

/// Determine edge type based on from and to nodes
fn determine_edge_type(from: &str, to: &str) -> GraphEdgeType {
    if from.starts_with("provider[") || to.starts_with("provider[") {
        return GraphEdgeType::Provider;
    }

    if from.starts_with("module.") || to.starts_with("module.") {
        return GraphEdgeType::Module;
    }

    // Default to reference
    GraphEdgeType::Reference
}

/// Calculate graph statistics
fn calculate_statistics(nodes: &[GraphNode], edges: &[GraphEdge]) -> GraphStatistics {
    let mut resource_count = 0;
    let mut data_source_count = 0;
    let mut module_count = 0;
    let mut provider_count = 0;

    for node in nodes {
        match node.node_type {
            GraphNodeType::Resource => resource_count += 1,
            GraphNodeType::DataSource => data_source_count += 1,
            GraphNodeType::Module => module_count += 1,
            GraphNodeType::Provider => provider_count += 1,
            _ => {}
        }
    }

    // Calculate max depth (simplified - count longest path from root)
    let max_depth = calculate_max_depth(nodes, edges);

    GraphStatistics {
        total_nodes: nodes.len() as i32,
        total_edges: edges.len() as i32,
        resource_count,
        data_source_count,
        module_count,
        provider_count,
        max_depth,
    }
}

/// Calculate maximum depth in the graph
fn calculate_max_depth(nodes: &[GraphNode], edges: &[GraphEdge]) -> i32 {
    // Build adjacency list
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        adj.entry(&edge.from).or_default().push(&edge.to);
    }

    // Find root nodes
    let root_nodes: Vec<&str> = nodes
        .iter()
        .filter(|n| n.node_type == GraphNodeType::Root || n.id.contains("[root]"))
        .map(|n| n.id.as_str())
        .collect();

    // BFS from root nodes to find max depth
    let mut max_depth = 0;
    let mut visited: HashMap<&str, i32> = HashMap::new();

    for root in root_nodes {
        let mut queue = vec![(root, 0)];
        while let Some((node, depth)) = queue.pop() {
            if let Some(&prev_depth) = visited.get(node) {
                if prev_depth >= depth {
                    continue;
                }
            }
            visited.insert(node, depth);
            max_depth = max_depth.max(depth);

            if let Some(neighbors) = adj.get(node) {
                for neighbor in neighbors {
                    queue.push((neighbor, depth + 1));
                }
            }
        }
    }

    max_depth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_edge_line() {
        let result = parse_edge_line(r#""aws_instance.example" -> "aws_vpc.main""#);
        assert!(result.is_some());
        let (from, to) = result.unwrap();
        assert_eq!(from, "aws_instance.example");
        assert_eq!(to, "aws_vpc.main");
    }

    #[test]
    fn test_determine_node_type() {
        let (node_type, provider) = determine_node_type("aws_instance.example");
        assert_eq!(node_type, GraphNodeType::Resource);
        assert_eq!(provider, Some("aws".to_string()));

        let (node_type, _) = determine_node_type("data.aws_ami.latest");
        assert_eq!(node_type, GraphNodeType::DataSource);

        let (node_type, _) = determine_node_type("module.vpc");
        assert_eq!(node_type, GraphNodeType::Module);

        let (node_type, _) = determine_node_type("[root]");
        assert_eq!(node_type, GraphNodeType::Root);
    }

    #[test]
    fn test_extract_provider() {
        assert_eq!(
            extract_provider_from_resource("aws_instance.example"),
            Some("aws".to_string())
        );
        assert_eq!(
            extract_provider_from_resource("google_compute_instance.main"),
            Some("google".to_string())
        );
    }
}
