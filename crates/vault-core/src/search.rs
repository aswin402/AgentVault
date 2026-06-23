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
    client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NpmSearchResponse {
    objects: Vec<NpmSearchObject>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NpmSearchObject {
    package: NpmPackageInfo,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NpmPackageInfo {
    name: String,
    version: String,
    description: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
}

impl SearchEngine {
    pub fn new(registry: Arc<dyn Registry>) -> Self {
        Self {
            registry,
            client: reqwest::Client::new(),
        }
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

    pub async fn search_npm(&self, query: &str) -> Result<Vec<SearchResult>, VaultError> {
        let response = self
            .client
            .get("https://registry.npmjs.org/-/v1/search")
            .query(&[("text", query), ("size", "20")])
            .send()
            .await?
            .text()
            .await?;

        let results = parse_npm_response(query, &response)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        Ok(results)
    }
}

fn is_mcp_relevant(info: &NpmPackageInfo) -> bool {
    let name_lower = info.name.to_lowercase();
    let desc_lower = info
        .description
        .as_ref()
        .map(|d| d.to_lowercase())
        .unwrap_or_default();

    if name_lower.contains("mcp")
        || name_lower.contains("model-context-protocol")
        || desc_lower.contains("mcp")
        || desc_lower.contains("model-context-protocol")
    {
        return true;
    }

    for kw in &info.keywords {
        let kw_lower = kw.to_lowercase();
        if kw_lower == "mcp" || kw_lower == "mcp-server" || kw_lower == "model-context-protocol" {
            return true;
        }
    }

    false
}

fn calculate_npm_score(query: &str, info: &NpmPackageInfo) -> f64 {
    let query_lower = query.to_lowercase();
    let name_lower = info.name.to_lowercase();

    if name_lower == query_lower {
        1.0
    } else if name_lower.contains(&query_lower) {
        0.8
    } else if info
        .description
        .as_ref()
        .map(|d| d.to_lowercase().contains(&query_lower))
        .unwrap_or(false)
    {
        0.5
    } else {
        0.4
    }
}

fn parse_npm_response(
    query: &str,
    response_body: &str,
) -> Result<Vec<SearchResult>, serde_json::Error> {
    let response: NpmSearchResponse = serde_json::from_str(response_body)?;
    let mut results = Vec::new();
    for obj in response.objects {
        let info = obj.package;
        if !is_mcp_relevant(&info) {
            continue;
        }

        let score = calculate_npm_score(query, &info);
        results.push(SearchResult {
            id: None,
            name: info.name,
            kind: CapabilityKind::Mcp,
            description: info.description,
            tags: info.keywords,
            source: "npm".to_string(),
            score,
        });
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

    #[test]
    fn test_parse_npm_response() {
        let response_json = r#"{
            "objects": [
                {
                    "package": {
                        "name": "mcp-server-filesystem",
                        "version": "0.1.0",
                        "description": "An MCP server for file operations",
                        "keywords": ["mcp", "filesystem"]
                    }
                },
                {
                    "package": {
                        "name": "unrelated-library",
                        "version": "1.0.0",
                        "description": "A random js utility library",
                        "keywords": ["utils"]
                    }
                },
                {
                    "package": {
                        "name": "my-mcp-plugin",
                        "version": "2.0.0",
                        "description": "Cool stuff",
                        "keywords": ["plugin"]
                    }
                }
            ]
        }"#;

        let results = parse_npm_response("mcp", response_json).unwrap();
        assert_eq!(results.len(), 2);

        // First result should be "mcp-server-filesystem" (score 0.8 since it contains "mcp")
        assert_eq!(results[0].name, "mcp-server-filesystem");
        assert_eq!(results[0].score, 0.8);
        assert_eq!(results[0].source, "npm");

        // Second result should be "my-mcp-plugin" (score 0.8 since it contains "mcp")
        assert_eq!(results[1].name, "my-mcp-plugin");
        assert_eq!(results[1].score, 0.8);

        // "unrelated-library" should be filtered out by relevance heuristics.
    }
}
