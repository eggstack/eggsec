#[cfg(feature = "advanced-hunting")]
use crate::hunt::chain::AttackChain;
use crate::types::Severity;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub clusters: Vec<GraphCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    pub severity: Severity,
    pub properties: FxHashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    Vulnerability,
    Asset,
    Service,
    User,
    Network,
    EntryPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    Exploits,
    LeadsTo,
    Accesses,
    CommunicatesWith,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCluster {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<String>,
}

pub struct AttackGraphBuilder;

impl AttackGraphBuilder {
    pub fn from_chains(chains: &[AttackChain]) -> AttackGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut clusters = Vec::new();

        let mut node_ids = FxHashMap::default();

        for chain in chains {
            let cluster_id = format!("cluster-{}", chain.id);
            let mut cluster_nodes = Vec::new();

            for step in &chain.steps {
                let node_id = format!("{}-step-{}", chain.id, step.step_number);
                node_ids.insert(node_id.clone(), true);

                nodes.push(GraphNode {
                    id: node_id.clone(),
                    label: step.vulnerability.clone(),
                    node_type: NodeType::Vulnerability,
                    severity: step.severity,
                    properties: {
                        let mut map = FxHashMap::default();
                        map.insert("prerequisite".to_string(), step.prerequisite.clone());
                        map.insert("impact".to_string(), step.impact.clone());
                        map.insert("evidence".to_string(), step.evidence.clone());
                        map
                    },
                });

                cluster_nodes.push(node_id.clone());

                if step.step_number > 1 {
                    let prev_id = format!("{}-step-{}", chain.id, step.step_number - 1);
                    edges.push(GraphEdge {
                        from: prev_id,
                        to: node_id.clone(),
                        edge_type: EdgeType::LeadsTo,
                        label: "escalates".to_string(),
                    });
                }
            }

            clusters.push(GraphCluster {
                id: cluster_id.clone(),
                label: chain.name.clone(),
                node_ids: cluster_nodes,
            });

            if let Some(_first) = chain.steps.first() {
                let entry_id = format!("{}-entry", chain.id);
                if !node_ids.contains_key(&entry_id) {
                    node_ids.insert(entry_id.clone(), true);
                    nodes.push(GraphNode {
                        id: entry_id.clone(),
                        label: "Initial Access".to_string(),
                        node_type: NodeType::EntryPoint,
                        severity: Severity::Medium,
                        properties: FxHashMap::default(),
                    });
                    edges.push(GraphEdge {
                        from: entry_id,
                        to: format!("{}-step-1", chain.id),
                        edge_type: EdgeType::Exploits,
                        label: "exploits".to_string(),
                    });
                }
            }
        }

        AttackGraph {
            nodes,
            edges,
            clusters,
        }
    }

    pub fn to_html(graph: &AttackGraph) -> Result<String, serde_json::Error> {
        let nodes_json = serde_json::to_string(&graph.nodes)?;
        let edges_json = serde_json::to_string(&graph.edges)?;

        let nodes_json = nodes_json.replace("</", "<\\/");
        let edges_json = edges_json.replace("</", "<\\/");

        Ok(format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Attack Chain Visualization</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <script src="https://unpkg.com/graphlib-dot@0.6.3/dist/graphlib-dot.min.js"></script>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; }}
        #graph {{ width: 100%; height: 800px; border: 1px solid #ccc; }}
        .node {{ fill: #ddd; stroke: #333; stroke-width: 2px; }}
        .edge {{ stroke: #999; stroke-width: 2px; fill: none; }}
        .critical {{ fill: #d32f2f; }}
        .high {{ fill: #f57c00; }}
        .medium {{ fill: #ff9800; }}
        .low {{ fill: #4caf50; }}
        .info {{ fill: #2196f3; }}
    </style>
</head>
<body>
    <h1>Attack Chain Visualization</h1>
    <div id="graph"></div>
    <script>
        const nodes = {nodes_json};
        const edges = {edges_json};
        // D3 visualization would be initialized here
        console.log("Attack graph with", nodes.length, "nodes and", edges.length, "edges");
    </script>
</body>
</html>"#
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_building() {
        let chains = vec![];
        let graph = AttackGraphBuilder::from_chains(&chains);
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_node_types() {
        assert_eq!(NodeType::Vulnerability, NodeType::Vulnerability);
        assert_eq!(NodeType::EntryPoint, NodeType::EntryPoint);
    }

    #[test]
    fn test_to_html_escapes_script_tag() {
        let graph = AttackGraph {
            nodes: vec![GraphNode {
                id: "n1".to_string(),
                label: "</script><script>alert(1)</script>".to_string(),
                node_type: NodeType::Vulnerability,
                severity: Severity::High,
                properties: FxHashMap::default(),
            }],
            edges: vec![],
            clusters: vec![],
        };
        let html = AttackGraphBuilder::to_html(&graph).unwrap();
        assert!(!html.contains("</script><script>"));
        assert!(html.contains("<\\/script>"));
    }
}
