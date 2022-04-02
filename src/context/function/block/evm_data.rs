//!
//! The LLVM generator function block EVM data.
//!

///
/// The LLVM generator function block EVM data.
///
#[derive(Debug, Clone)]
pub struct EVMData {
    /// The initial stack state hash.
    pub stack_hash: md5::Digest,
}

impl EVMData {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(stack_hash: md5::Digest) -> Self {
        Self { stack_hash }
    }
}
