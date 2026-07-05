//! Training cache: tracks which repos and issues have been processed
//! 
//! Enables idempotent training runs: each repo/issue combo is processed exactly once
//! even if `make idud-grow` crashes and restarts.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::RwLock;

/// Single cache entry tracking a processed repo/issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub repo_url: String,
    pub issue_number: u32,
    pub issue_id: String,
    pub processed_at: DateTime<Utc>,
    pub prediction_run_id: Option<String>,
    pub status: String, // "completed", "failed", "pending"
}

/// In-memory training cache with persistent JSON storage
pub struct TrainingCache {
    cache_path: String,
    entries: RwLock<Vec<CacheEntry>>,
    processed_keys: RwLock<HashSet<String>>,
}

impl TrainingCache {
    /// Load or create cache from file
    pub fn new(cache_path: &str) -> Result<Self> {
        let entries = if Path::new(cache_path).exists() {
            let content = std::fs::read_to_string(cache_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Build fast lookup set
        let processed_keys: HashSet<String> = entries
            .iter()
            .map(|e: &CacheEntry| format!("{}#{}", e.repo_url, e.issue_number))
            .collect();

        Ok(Self {
            cache_path: cache_path.to_string(),
            entries: RwLock::new(entries),
            processed_keys: RwLock::new(processed_keys),
        })
    }

    /// Check if a repo/issue has already been processed
    pub fn is_processed(&self, repo_url: &str, issue_number: u32) -> bool {
        let key = format!("{}#{}", repo_url, issue_number);
        self.processed_keys
            .read()
            .unwrap()
            .contains(&key)
    }

    /// Mark a repo/issue as processed
    pub fn mark_processed(
        &self,
        repo_url: &str,
        issue_number: u32,
        issue_id: &str,
        prediction_run_id: Option<String>,
    ) -> Result<()> {
        let key = format!("{}#{}", repo_url, issue_number);

        // Add to in-memory cache
        {
            let mut processed = self.processed_keys.write().unwrap();
            processed.insert(key);
        }

        // Add entry
        {
            let mut entries = self.entries.write().unwrap();
            entries.push(CacheEntry {
                repo_url: repo_url.to_string(),
                issue_number,
                issue_id: issue_id.to_string(),
                processed_at: Utc::now(),
                prediction_run_id,
                status: "completed".to_string(),
            });
        }

        // Persist to disk
        self.persist()?;

        Ok(())
    }

    /// Get all processed repos (for filtering discovery)
    pub fn get_processed_repos(&self) -> Vec<String> {
        let entries = self.entries.read().unwrap();
        let mut repos: Vec<String> = entries
            .iter()
            .map(|e| e.repo_url.clone())
            .collect();
        repos.sort();
        repos.dedup();
        repos
    }

    /// Get count of processed entries for stats
    pub fn count_processed(&self) -> usize {
        self.entries.read().unwrap().len()
    }

    /// Get recent processing stats
    pub fn get_stats(&self) -> CacheStats {
        let entries = self.entries.read().unwrap();
        let total = entries.len();

        let completed = entries.iter().filter(|e| e.status == "completed").count();
        let failed = entries.iter().filter(|e| e.status == "failed").count();
        let pending = entries.iter().filter(|e| e.status == "pending").count();

        let unique_repos = entries
            .iter()
            .map(|e| &e.repo_url)
            .collect::<HashSet<_>>()
            .len();

        CacheStats {
            total_processed: total,
            completed,
            failed,
            pending,
            unique_repos,
            last_processed: entries.last().map(|e| e.processed_at),
        }
    }

    /// Mark entry as failed (for error handling)
    pub fn mark_failed(&self, repo_url: &str, issue_number: u32) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries
            .iter_mut()
            .find(|e| e.repo_url == repo_url && e.issue_number == issue_number)
        {
            entry.status = "failed".to_string();
        }
        drop(entries);
        self.persist()
    }

    /// Clear entire cache (useful for full restart)
    pub fn clear(&self) -> Result<()> {
        {
            let mut entries = self.entries.write().unwrap();
            entries.clear();
        }
        {
            let mut processed = self.processed_keys.write().unwrap();
            processed.clear();
        }
        self.persist()
    }

    /// Persist cache to disk
    fn persist(&self) -> Result<()> {
        let entries = self.entries.read().unwrap();
        let json = serde_json::to_string_pretty(&*entries)?;
        std::fs::write(&self.cache_path, json)?;
        Ok(())
    }
}

/// Statistics about cache contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_processed: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub unique_repos: usize,
    pub last_processed: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cache_new() {
        let tmp = NamedTempFile::new().unwrap();
        let cache = TrainingCache::new(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(cache.count_processed(), 0);
    }

    #[test]
    fn test_mark_processed() {
        let tmp = NamedTempFile::new().unwrap();
        let cache = TrainingCache::new(tmp.path().to_str().unwrap()).unwrap();

        cache
            .mark_processed("https://github.com/test/repo", 123, "issue-123", None)
            .unwrap();

        assert!(cache.is_processed("https://github.com/test/repo", 123));
        assert!(!cache.is_processed("https://github.com/test/repo", 456));
        assert_eq!(cache.count_processed(), 1);
    }

    #[test]
    fn test_cache_persistence() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        {
            let cache = TrainingCache::new(path).unwrap();
            cache
                .mark_processed("https://github.com/test/repo", 123, "issue-123", None)
                .unwrap();
        }

        // Reload and verify
        {
            let cache = TrainingCache::new(path).unwrap();
            assert!(cache.is_processed("https://github.com/test/repo", 123));
            assert_eq!(cache.count_processed(), 1);
        }
    }

    #[test]
    fn test_get_stats() {
        let tmp = NamedTempFile::new().unwrap();
        let cache = TrainingCache::new(tmp.path().to_str().unwrap()).unwrap();

        cache
            .mark_processed("https://github.com/test/repo1", 123, "issue-123", None)
            .unwrap();
        cache
            .mark_processed("https://github.com/test/repo1", 456, "issue-456", None)
            .unwrap();
        cache
            .mark_processed("https://github.com/test/repo2", 789, "issue-789", None)
            .unwrap();

        let stats = cache.get_stats();
        assert_eq!(stats.total_processed, 3);
        assert_eq!(stats.completed, 3);
        assert_eq!(stats.unique_repos, 2);
    }
}
