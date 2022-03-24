//!
//! The LLVM generator function block EVM data.
//!

///
/// The LLVM generator function block EVM data.
///
#[derive(Debug, Clone)]
pub struct EVMData {
    /// The call trace.
    pub call_trace: Vec<usize>,
    /// The block vertical tags buffer.
    pub vertical_tags_buffer: Vec<usize>,
}

impl EVMData {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(call_trace: Vec<usize>, vertical_tags_buffer: Vec<usize>) -> Self {
        Self {
            call_trace,
            vertical_tags_buffer,
        }
    }
}
