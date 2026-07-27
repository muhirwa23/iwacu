//! Constraint Satisfaction Matrix for deterministic planning

use serde::{Deserialize, Serialize};

/// A single constraint that must be satisfied before proceeding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub required: bool,
    pub options: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
}

/// Constraint Satisfaction Matrix output by the Plan node
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstraintSatisfactionMatrix {
    pub constraints: Vec<Constraint>,
}

impl ConstraintSatisfactionMatrix {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_constraint(
        &mut self,
        id: impl Into<String>,
        required: bool,
        options: Vec<String>,
        default: Option<String>,
    ) {
        self.constraints.push(Constraint {
            id: id.into(),
            required,
            options,
            default,
        });
    }

    /// Check if all required constraints have a satisfiable value
    pub fn is_satisfied(&self) -> bool {
        self.constraints
            .iter()
            .all(|c| !c.required || c.default.is_some() || !c.options.is_empty())
    }
}
