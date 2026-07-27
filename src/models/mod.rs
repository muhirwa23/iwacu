//! Core data models for the Autonomous Symbiote

pub mod constraint;
pub mod context;
pub mod diagnostic;
pub mod message;
pub mod mutation;

pub use constraint::{Constraint, ConstraintSatisfactionMatrix};
pub use context::{ContextPayload, SymbolInfo};
pub use diagnostic::{Diagnostic, DiagnosticGraph, ErrorDependency};
pub use message::*;
pub use mutation::MutationResult;
