//! Context document repository
//!
//! Table names are PascalCase (created by Drizzle/TypeScript): "ContextDocument"
//! Column names are camelCase: "workflowId", "sourceType", "sourceRef", etc.

use crate::pool::DbPool;
use crate::schema::{NewContextDocument, ContextDocument};
use masday_core::{AppError, Result};
use pgvector::Vector;

pub struct ContextDocumentRepo {
    pool: DbPool,
}

impl ContextDocumentRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new context document
    pub async fn create(&self, doc: &NewContextDocument) -> Result<ContextDocument> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();

        let query = r#"
            INSERT INTO "ContextDocument" (
                id, "workflowId", "sourceType", "sourceRef", title, content,
                metadata, fingerprint, embedding, "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
        "#;

        let embedding_pg: Option<Vector> = doc.embedding.clone();

        let row = client
            .query_one(
                query,
                &[
                    &id,
                    &doc.workflow_id,
                    &doc.source_type,
                    &doc.source_ref,
                    &doc.title,
                    &doc.content,
                    &doc.metadata,
                    &doc.fingerprint,
                    &embedding_pg,
                    &now,
                    &now,
                ],
            )
            .await
            .map_err(|e| AppError::Database(format!("Failed to create context document: {}", e)))?;

        Ok(ContextDocument::from_row(&row))
    }

    /// Get context document by ID
    pub async fn get_by_id(&self, id: &str) -> Result<ContextDocument> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "ContextDocument" WHERE id = $1"#;
        let row = client
            .query_one(query, &[&id])
            .await
            .map_err(|_e| AppError::not_found("ContextDocument", id))?;

        Ok(ContextDocument::from_row(&row))
    }

    /// List all context documents for a workflow
    pub async fn list_by_workflow(&self, workflow_id: &str) -> Result<Vec<ContextDocument>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"
            SELECT * FROM "ContextDocument"
            WHERE "workflowId" = $1
            ORDER BY "createdAt" DESC
        "#;
        let rows = client
            .query(query, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list context documents: {}", e)))?;

        Ok(rows.iter().map(ContextDocument::from_row).collect())
    }

    /// List all context documents by source type
    pub async fn list_by_source_type(&self, source_type: &str, limit: Option<i64>) -> Result<Vec<ContextDocument>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"
            SELECT * FROM "ContextDocument"
            WHERE "sourceType" = $1
            ORDER BY "createdAt" DESC
            LIMIT $2
        "#;
        let rows = client
            .query(query, &[&source_type, &capped])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list context documents by source type: {}", e)))?;

        Ok(rows.iter().map(ContextDocument::from_row).collect())
    }

    /// List all context documents (with optional limit)
    pub async fn list_all(&self, limit: Option<i64>) -> Result<Vec<ContextDocument>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let capped = limit.unwrap_or(100).min(1000);
        let query = r#"SELECT * FROM "ContextDocument" ORDER BY "createdAt" DESC LIMIT $1"#;
        let rows = client
            .query(query, &[&capped])
            .await
            .map_err(|e| AppError::Database(format!("Failed to list all context documents: {}", e)))?;

        Ok(rows.iter().map(ContextDocument::from_row).collect())
    }

    /// Delete a context document
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM "ContextDocument" WHERE id = $1"#, &[&id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete context document: {}", e)))?;

        Ok(result > 0)
    }

    /// Delete all context documents for a workflow
    pub async fn delete_by_workflow(&self, workflow_id: &str) -> Result<u64> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let result = client
            .execute(r#"DELETE FROM "ContextDocument" WHERE "workflowId" = $1"#, &[&workflow_id])
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete workflow context documents: {}", e)))?;

        Ok(result)
    }

    /// Get context document by fingerprint
    pub async fn get_by_fingerprint(&self, fingerprint: &str) -> Result<Option<ContextDocument>> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Database(format!("Failed to get connection: {}", e)))?;

        let query = r#"SELECT * FROM "ContextDocument" WHERE fingerprint = $1 LIMIT 1"#;
        let rows = client
            .query(query, &[&fingerprint])
            .await
            .map_err(|e| AppError::Database(format!("Failed to get context document by fingerprint: {}", e)))?;

        if rows.is_empty() {
            return Ok(None);
        }

        Ok(Some(ContextDocument::from_row(&rows[0])))
    }
}
