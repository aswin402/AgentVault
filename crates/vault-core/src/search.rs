use crate::capability::models::{CapabilityKind, CapabilityRecord};
use crate::error::VaultError;
use crate::registry::Registry;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub id: Option<String>,
    pub name: String,
    pub kind: CapabilityKind,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub source: String, // "local" or "npm"
    pub score: f64,
}

pub struct SearchEngine {
    registry: Arc<dyn Registry>,
}

impl SearchEngine {
    pub fn new(registry: Arc<dyn Registry>) -> Self {
        Self { registry }
    }

    pub fn calculate_score(query: &str, record: &CapabilityRecord) -> f64 {
        let query_lower = query.to_lowercase();
        let name_lower = record.name.to_lowercase();

        // 1. Exact name match (case-insensitive check)
        if name_lower == query_lower {
            return 1.0;
        }

        // 2. Partial name match (contains substring)
        if name_lower.contains(&query_lower) {
            return 0.8;
        }

        // 3. Tag match (any tag is an exact match to the query, case-insensitive)
        let has_tag_match = record.tags.iter().any(|t| t.to_lowercase() == query_lower);
        if has_tag_match {
            return 0.7;
        }

        // 4. Description keyword match (description contains query as substring)
        if let Some(desc) = &record.description {
            if desc.to_lowercase().contains(&query_lower) {
                return 0.5;
            }
        }

        0.0
    }

    pub fn search_local(&self, query: &str) -> Result<Vec<SearchResult>, VaultError> {
        let records = self.registry.search(query)?;
        let mut results = Vec::new();

        for record in records {
            let score = Self::calculate_score(query, &record);
            if score > 0.3 {
                results.push(SearchResult {
                    id: Some(record.id),
                    name: record.name,
                    kind: record.kind,
                    description: record.description,
                    tags: record.tags,
                    source: "local".to_string(),
                    score,
                });
            }
        }

        // Sort descending by score, then alphabetically by name as tie-breaker
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_score() {
        let record = CapabilityRecord {
            id: "1".to_string(),
            name: "fs-tool".to_string(),
            kind: CapabilityKind::Mcp,
            description: Some("A filesystem tool for reading and writing files".to_string()),
            tags: vec!["fs".to_string(), "local".to_string()],
        };

        // 1. Exact name match
        assert_eq!(SearchEngine::calculate_score("fs-tool", &record), 1.0);
        assert_eq!(SearchEngine::calculate_score("FS-TOOL", &record), 1.0);

        // 2. Partial name match
        assert_eq!(SearchEngine::calculate_score("tool", &record), 0.8);
        assert_eq!(SearchEngine::calculate_score("fs-", &record), 0.8);

        // 3. Tag match
        assert_eq!(SearchEngine::calculate_score("local", &record), 0.7);
        assert_eq!(SearchEngine::calculate_score("LOCAL", &record), 0.7);

        // 4. Description match
        assert_eq!(SearchEngine::calculate_score("reading", &record), 0.5);
        assert_eq!(SearchEngine::calculate_score("writing", &record), 0.5);

        // 5. No match
        assert_eq!(SearchEngine::calculate_score("database", &record), 0.0);
    }
}
