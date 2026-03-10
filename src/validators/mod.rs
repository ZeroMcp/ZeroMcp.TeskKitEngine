pub mod determinism;
pub mod error_path;
pub mod metadata;
pub mod protocol_val;
pub mod schema;

use crate::engine::result::ValidationError;

/// All validators conform to this pattern: given some context, return
/// a list of validation errors (empty means passed).
#[allow(dead_code)]
pub type ValidationResult = Vec<ValidationError>;
