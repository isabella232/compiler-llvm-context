//!
//! The LLVM generator EVM data.
//!

///
/// The LLVM generator EVM data.
///
#[derive(Debug, Default, Clone)]
pub struct EVMData<'ctx> {
    /// The static stack allocated for the current function.
    pub stack: Vec<inkwell::values::PointerValue<'ctx>>,
}
