use crate::error::VaultError;
use crate::registry::Registry;
use crate::workflow::models::WorkflowStep;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct DependencyResolver;

impl DependencyResolver {
    /// Topological sort of workflow steps using Kahn's Algorithm.
    /// Returns steps in execution order.
    /// Returns VaultError if a cycle is detected or if a step depends on a non-existent step.
    pub fn resolve(steps: &[WorkflowStep]) -> Result<Vec<&WorkflowStep>, VaultError> {
        let mut adj_list: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut step_map: HashMap<&str, &WorkflowStep> = HashMap::new();

        // Initialize degree and step maps
        for step in steps {
            step_map.insert(&step.name, step);
            in_degree.insert(&step.name, 0);
            adj_list.insert(&step.name, Vec::new());
        }

        // Build adjacency list and in-degrees
        for step in steps {
            for dep in &step.depends_on {
                if !step_map.contains_key(dep.as_str()) {
                    return Err(VaultError::Config {
                        message: format!(
                            "Step '{}' depends on non-existent step '{}'",
                            step.name, dep
                        ),
                    });
                }
                adj_list.get_mut(dep.as_str()).unwrap().push(&step.name);
                *in_degree.get_mut(step.name.as_str()).unwrap() += 1;
            }
        }

        // Find all nodes with in-degree 0
        let mut queue = VecDeque::new();
        for (&name, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(name);
            }
        }

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop_front() {
            if let Some(&step) = step_map.get(node) {
                sorted.push(step);
            }
            if let Some(neighbors) = adj_list.get(node) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        if sorted.len() != steps.len() {
            // Find nodes involved in the cycle
            let cycle_nodes: Vec<String> = in_degree
                .iter()
                .filter(|(_, &deg)| deg > 0)
                .map(|(&name, _)| name.to_string())
                .collect();
            return Err(VaultError::Config {
                message: format!(
                    "Circular dependency detected involving steps: {}",
                    cycle_nodes.join(", ")
                ),
            });
        }

        Ok(sorted)
    }

    /// Check that all capabilities referenced by steps are installed in the vault.
    /// Returns list of missing capabilities.
    pub fn check_dependencies(
        steps: &[WorkflowStep],
        registry: &dyn Registry,
    ) -> Result<Vec<String>, VaultError> {
        let mut missing = Vec::new();
        let mut checked = HashSet::new();

        for step in steps {
            if checked.contains(&step.uses) {
                continue;
            }
            checked.insert(&step.uses);

            if let Some((prefix, name)) = step.uses.split_once(':') {
                match prefix {
                    "mcp" => {
                        if registry.get_mcp(name).is_err() {
                            missing.push(step.uses.clone());
                        }
                    }
                    "skill" => {
                        if registry.get_skill(name).is_err() {
                            missing.push(step.uses.clone());
                        }
                    }
                    _ => {
                        return Err(VaultError::Config {
                            message: format!(
                                "Unknown capability type prefix '{}' in '{}'. Expected 'mcp' or 'skill'.",
                                prefix, step.uses
                            ),
                        });
                    }
                }
            } else {
                return Err(VaultError::Config {
                    message: format!(
                        "Invalid capability reference format '{}'. Expected format 'type:name' (e.g. 'mcp:github').",
                        step.uses
                    ),
                });
            }
        }

        Ok(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::models::WorkflowStep;

    #[test]
    fn test_resolver_valid_dag() {
        let steps = vec![
            WorkflowStep {
                name: "step3".to_string(),
                uses: "mcp:fs".to_string(),
                args: HashMap::new(),
                depends_on: vec!["step2".to_string()],
                condition: None,
            },
            WorkflowStep {
                name: "step1".to_string(),
                uses: "mcp:fs".to_string(),
                args: HashMap::new(),
                depends_on: vec![],
                condition: None,
            },
            WorkflowStep {
                name: "step2".to_string(),
                uses: "mcp:fs".to_string(),
                args: HashMap::new(),
                depends_on: vec!["step1".to_string()],
                condition: None,
            },
        ];

        let resolved = DependencyResolver::resolve(&steps).unwrap();
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].name, "step1");
        assert_eq!(resolved[1].name, "step2");
        assert_eq!(resolved[2].name, "step3");
    }

    #[test]
    fn test_resolver_cycle_detection() {
        let steps = vec![
            WorkflowStep {
                name: "step1".to_string(),
                uses: "mcp:fs".to_string(),
                args: HashMap::new(),
                depends_on: vec!["step2".to_string()],
                condition: None,
            },
            WorkflowStep {
                name: "step2".to_string(),
                uses: "mcp:fs".to_string(),
                args: HashMap::new(),
                depends_on: vec!["step1".to_string()],
                condition: None,
            },
        ];

        let res = DependencyResolver::resolve(&steps);
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("Circular dependency detected"));
    }

    #[test]
    fn test_resolver_missing_step_dependency() {
        let steps = vec![WorkflowStep {
            name: "step1".to_string(),
            uses: "mcp:fs".to_string(),
            args: HashMap::new(),
            depends_on: vec!["nonexistent".to_string()],
            condition: None,
        }];

        let res = DependencyResolver::resolve(&steps);
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("nonexistent"));
    }
}
