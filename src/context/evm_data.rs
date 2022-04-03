//!
//! The LLVM generator EVM data.
//!

use crate::context::argument::Argument;

///
/// The LLVM generator EVM data.
///
#[derive(Debug, Clone)]
pub struct EVMData<'ctx> {
    /// The Solidity compiler version.
    /// Some instruction behave differenly depending on the version.
    pub version: semver::Version,
    /// The static stack allocated for the current function.
    pub stack: Vec<Argument<'ctx>>,
}

impl<'ctx> EVMData<'ctx> {
    /// The default stack size.
    pub const DEFAULT_STACK_SIZE: usize = 64;

    ///
    /// A shortcut constructor.
    ///
    pub fn new(version: semver::Version) -> Self {
        Self {
            version,
            stack: Vec::with_capacity(Self::DEFAULT_STACK_SIZE),
        }
    }
}
