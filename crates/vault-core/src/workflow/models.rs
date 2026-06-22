use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A workflow definition in the vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowEntry {
    /// Unique identifier (ULID/UUID).
    pub id: String,

    /// Short machine-friendly name, unique within the vault.
    pub name: String,

    /// Human-readable description.
    pub description: Option<String>,

    /// Ordered list of workflow steps.
    pub steps: Vec<WorkflowStep>,

    /// Names of other capabilities (MCPs, skills) this workflow depends on.
    pub dependencies: Vec<String>,

    /// Timestamp when this workflow was first installed.
    pub installed_at: DateTime<Utc>,
}

/// A single step in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowStep {
    /// Human-readable name for this step.
    pub name: String,

    /// Optional MCP server ID this step invokes.
    pub mcp_id: Option<String>,

    /// Optional skill ID this step invokes.
    pub skill_id: Option<String>,

    /// Step-specific configuration key-value pairs.
    pub config: HashMap<String, String>,

    /// Names of other steps this step depends on (must complete first).
    pub depends_on: Vec<String>,
}
