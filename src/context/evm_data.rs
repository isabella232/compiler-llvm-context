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
    /// The static stack pointer offset in the current function.
    pub stack_offset: usize,
}

impl<'ctx> EVMData<'ctx> {
    ///
    /// Returns the stack pointer value.
    ///
    pub fn stack_pointer(&self, offset: usize) -> inkwell::values::PointerValue<'ctx> {
        self.stack[self.stack_offset - offset]
    }

    ///
    /// Increases the stack pointer value.
    ///
    pub fn increase_stack_pointer(&mut self, offset: usize) {
        self.stack_offset += offset;
    }

    ///
    /// Decreases the stack pointer value.
    ///
    pub fn decrease_stack_pointer(&mut self, offset: usize) {
        self.stack_offset -= offset;
    }
}
