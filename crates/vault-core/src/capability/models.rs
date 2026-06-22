use serde::{Deserialize, Serialize};

/// Identifies what kind of capability we are operating on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Mcp,
    Skill,
    Workflow,
}

impl std::fmt::Display for CapabilityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapabilityKind::Mcp => write!(f, "mcp"),
            CapabilityKind::Skill => write!(f, "skill"),
            CapabilityKind::Workflow => write!(f, "workflow"),
        }
    }
}

/// A high-level capability representation for registry view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityRecord {
    pub id: String,
    pub name: String,
    pub kind: CapabilityKind,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

/// A high-level capability bundle definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub required_mcps: Vec<String>,
    pub required_skills: Vec<String>,
    pub required_workflows: Vec<String>,
}
