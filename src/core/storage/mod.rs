use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

/// A lightweight topology snapshot intended for high-throughput AI retrieval.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopologySnapshot {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

/// A structural graph node persisted in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopologyNode {
    pub hash: String,
    pub source_path: String,
    pub kind: String,
    pub label: String,
}

/// A structural graph edge persisted in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopologyEdge {
    pub parent_hash: String,
    pub child_hash: String,
    pub kind: String,
}

/// A storage trait for the graph engine.
#[async_trait]
pub trait Database {
    /// Connect to or create the underlying storage.
    async fn connect(&self, path: &Path) -> Result<()>;

    /// Persist a topology snapshot to the backend.
    async fn store_topology(&self, snapshot: &TopologySnapshot) -> Result<()>;

    /// Return a lightweight structural view for AI tooling.
    async fn get_topology(&self) -> Result<TopologySnapshot>;

    /// Return the raw source fragment for a node if it exists.
    async fn get_raw_node(&self, hash: &str) -> Result<Option<String>>;
}

/// SQLite-backed implementation of the graph database.
#[derive(Debug, Default)]
pub struct SqliteDatabase {
    path: Mutex<Option<PathBuf>>,
}

impl SqliteDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    fn connection(&self) -> Result<Connection> {
        let guard = self.path.lock().unwrap();
        let path = guard
            .clone()
            .ok_or_else(|| anyhow::anyhow!("database is not connected"))?;
        Connection::open(path).context("failed to open sqlite connection")
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn connect(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create sqlite parent directory")?;
        }

        let connection = Connection::open(path).context("failed to open sqlite connection")?;
        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS graph_nodes (
                hash TEXT PRIMARY KEY,
                source_path TEXT NOT NULL,
                kind TEXT NOT NULL,
                label TEXT NOT NULL,
                source_fragment TEXT
            );

            CREATE TABLE IF NOT EXISTS graph_edges (
                parent_hash TEXT NOT NULL,
                child_hash TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'contains',
                PRIMARY KEY (parent_hash, child_hash)
            );
            "#,
        )?;

        {
            let mut path_guard = self.path.lock().unwrap();
            *path_guard = Some(path.to_path_buf());
        }
        Ok(())
    }

    async fn store_topology(&self, snapshot: &TopologySnapshot) -> Result<()> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;

        for node in &snapshot.nodes {
            tx.execute(
                "INSERT OR REPLACE INTO graph_nodes (hash, source_path, kind, label, source_fragment) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![node.hash, node.source_path, node.kind, node.label, None::<String>],
            )?;
        }

        for edge in &snapshot.edges {
            tx.execute(
                "INSERT OR REPLACE INTO graph_edges (parent_hash, child_hash, kind) VALUES (?1, ?2, ?3)",
                params![edge.parent_hash, edge.child_hash, edge.kind],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    async fn get_topology(&self) -> Result<TopologySnapshot> {
        let conn = self.connection()?;
        let mut node_stmt =
            conn.prepare("SELECT hash, source_path, kind, label FROM graph_nodes ORDER BY hash")?;
        let nodes = node_stmt
            .query_map([], |row| {
                Ok(TopologyNode {
                    hash: row.get(0)?,
                    source_path: row.get(1)?,
                    kind: row.get(2)?,
                    label: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut edge_stmt = conn.prepare("SELECT parent_hash, child_hash, kind FROM graph_edges ORDER BY parent_hash, child_hash")?;
        let edges = edge_stmt
            .query_map([], |row| {
                Ok(TopologyEdge {
                    parent_hash: row.get(0)?,
                    child_hash: row.get(1)?,
                    kind: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(TopologySnapshot { nodes, edges })
    }

    async fn get_raw_node(&self, hash: &str) -> Result<Option<String>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare("SELECT source_fragment FROM graph_nodes WHERE hash = ?1")?;
        let row = stmt
            .query_row(params![hash], |row| row.get::<_, Option<String>>(0))
            .optional()?;
        Ok(row.flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn sqlite_database_round_trips_topology() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("idud.sqlite");
        let database = SqliteDatabase::new();
        database.connect(&db_path).await.unwrap();

        let snapshot = TopologySnapshot {
            nodes: vec![TopologyNode {
                hash: "node-1".to_string(),
                source_path: "src/lib.rs".to_string(),
                kind: "function".to_string(),
                label: "run".to_string(),
            }],
            edges: vec![TopologyEdge {
                parent_hash: "root".to_string(),
                child_hash: "node-1".to_string(),
                kind: "contains".to_string(),
            }],
        };

        database.store_topology(&snapshot).await.unwrap();
        let round_trip = database.get_topology().await.unwrap();
        assert_eq!(round_trip.nodes.len(), 1);
        assert_eq!(round_trip.edges.len(), 1);
    }
}
