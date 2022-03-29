//!
//! The LLVM generator function block EVM data.
//!

///
/// The LLVM generator function block EVM data.
///
#[derive(Debug, Clone)]
pub struct EVMData {
    /// The stack pattern.
    pub stack_pattern: String,
}

impl EVMData {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(stack_pattern: String) -> Self {
        Self { stack_pattern }
    }
}
