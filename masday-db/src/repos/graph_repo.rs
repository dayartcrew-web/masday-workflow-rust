//! Knowledge graph repository
//!
//! Table names are snake_case: "graph_nodes", "graph_edges"
//! Column names are snake_case: "node_type", "created_at", etc.

use crate::pool::DbPool;
use crate::schema::{GraphEdge, GraphNode, NewGraphEdge, NewGraphNode};
use masday_core::{AppError, Result};
use tracing::debug;

pub struct GraphRepo {
    pool: DbPool,
}

impl GraphRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Add a node to the knowledge graph
    pub async fn add_node(&self, node: &NewGraphNode) -> Result<GraphNode> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        // Serialize properties to serde_json::Value for jsonb column
        let props_value: Option<serde_json::Value> = node.properties.clone();

        let query = r#"
            INSERT INTO graph_nodes (id, node_type, name, properties, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
        "#;

        let id_ref: &str = &id;
        let nt_ref: &str = &node.node_type;
        let name_ref: &str = &node.name;

        let row = client
            .query_one(query, &[&id_ref, &nt_ref, &name_ref, &props_value, &now])
            .await
            .map_err(|e| AppError::Database(format!("Failed to add node: {}", e)))?;

        Ok(GraphNode::from_row(&row))
    }

    /// Add an edge to the knowledge graph
    pub async fn add_edge(&self, edge: &NewGraphEdge) -> Result<GraphEdge> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO graph_edges (id, source_node_id, target_node_id, relation_type, weight, bidirectional, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
        "#;

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &edge.source_node_id,
                    &edge.target_node_id,
                    &edge.relation_type,
                    &edge.weight,
                    &edge.bidirectional,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to add edge: {}", e)))?;

        Ok(GraphEdge::from_row(&row))
    }

    /// Get a node by ID
    pub async fn get_node(&self, id: &str) -> Result<GraphNode> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM graph_nodes WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("GraphNode", id))?;

        Ok(GraphNode::from_row(&row))
    }

    /// Search nodes by type and name pattern
    pub async fn search_nodes(
        &self,
        node_type: &str,
        name_pattern: &str,
        limit: i64,
    ) -> Result<Vec<GraphNode>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let search_pattern = format!("%{}%", name_pattern);

        let query = r#"
            SELECT * FROM graph_nodes
            WHERE ($1 = '' OR node_type = $1) AND name ILIKE $2
            ORDER BY created_at DESC
            LIMIT $3
        "#;

        let rows = client
            .query(query, &[&node_type, &search_pattern, &limit])
            .await
            .map_err(|e| AppError::Database(format!("Failed to search nodes: {}", e)))?;

        Ok(rows.iter().map(GraphNode::from_row).collect())
    }

    /// Auto-link nodes based on Jaccard similarity threshold
    pub async fn auto_link(&self, node_id: &str, threshold: f64) -> Result<Vec<GraphEdge>> {
        let source_node = match self.get_node(node_id).await {
            Ok(node) => node,
            Err(_) => return Ok(Vec::new()),
        };

        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM graph_nodes
            WHERE node_type = $1 AND id != $2
            ORDER BY created_at DESC
            LIMIT 10
        "#;

        let rows = client
            .query(query, &[&source_node.node_type, &node_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to find similar nodes: {}", e)))?;

        let mut created_edges = Vec::new();

        for row in rows {
            let target_id: String = row.get("id");
            let target_name: String = row.get("name");

            let similarity = compute_name_similarity(&source_node.name, &target_name);

            if similarity >= threshold {
                let edge = NewGraphEdge {
                    source_node_id: node_id.to_string(),
                    target_node_id: target_id.clone(),
                    relation_type: "similar".to_string(),
                    weight: Some(similarity),
                    bidirectional: Some(true),
                };

                match self.add_edge(&edge).await {
                    Ok(created_edge) => {
                        debug!(
                            "Auto-linked {} -> {} with similarity {}",
                            node_id, target_id, similarity
                        );
                        created_edges.push(created_edge);
                    }
                    Err(e) => {
                        debug!("Failed to create auto-link edge: {}", e);
                    }
                }
            }
        }

        Ok(created_edges)
    }

    /// Get edges for a node (both incoming and outgoing)
    pub async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM graph_edges
            WHERE source_node_id = $1 OR target_node_id = $1
            ORDER BY created_at DESC
        "#;

        let rows = client
            .query(query, &[&node_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get node edges: {}", e)))?;

        Ok(rows.iter().map(GraphEdge::from_row).collect())
    }

    /// Delete a node
    pub async fn delete_node(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        // First delete all edges connected to this node
        let edge_query =
            r#"DELETE FROM graph_edges WHERE source_node_id = $1 OR target_node_id = $1"#;
        client
            .execute(edge_query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete node edges: {}", e)))?;

        // Then delete the node
        let query = r#"DELETE FROM graph_nodes WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete node: {}", e)))?;

        Ok(rows_affected > 0)
    }

    /// Delete an edge
    pub async fn delete_edge(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"DELETE FROM graph_edges WHERE id = $1"#;
        let rows_affected = client
            .execute(query, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete edge: {}", e)))?;

        Ok(rows_affected > 0)
    }
}

/// Compute simple similarity between two names using Jaccard-like approach
fn compute_name_similarity(name1: &str, name2: &str) -> f64 {
    let words1: std::collections::HashSet<&str> = name1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = name2.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 0.0;
    }

    let intersection = words1.intersection(&words2).count() as f64;
    let union = words1.union(&words2).count() as f64;

    if union == 0.0 {
        return 0.0;
    }

    intersection / union
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_name_similarity() {
        assert!((compute_name_similarity("test workflow", "test workflow") - 1.0).abs() < 0.01);
        let sim = compute_name_similarity("test workflow one", "test workflow two");
        assert!((sim - 0.5).abs() < 0.01);
        assert_eq!(compute_name_similarity("foo bar", "baz qux"), 0.0);
        assert_eq!(compute_name_similarity("", ""), 0.0);
    }
}
