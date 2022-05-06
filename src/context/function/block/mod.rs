//!
//! The LLVM generator function block.
//!

pub mod evm_data;
pub mod key;

use self::evm_data::EVMData;

///
/// The LLVM generator function block.
///
#[derive(Debug, Clone)]
pub struct Block<'ctx> {
    /// The inner block.
    pub inner: inkwell::basic_block::BasicBlock<'ctx>,

    /// The EVM compiler data.
    pub evm_data: Option<EVMData>,
}

impl<'ctx> Block<'ctx> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(inner: inkwell::basic_block::BasicBlock<'ctx>) -> Self {
        Self {
            inner,
            evm_data: None,
        }
    }

    ///
    /// A shortcut constructor for the EVM compiler.
    ///
    pub fn new_evm(inner: inkwell::basic_block::BasicBlock<'ctx>, evm_data: EVMData) -> Self {
        let mut object = Self::new(inner);
        object.evm_data = Some(evm_data);
        object
    }

    ///
    /// Returns the EVM data reference.
    ///
    /// # Panics
    /// If the EVM data has not been initialized.
    ///
    pub fn evm(&self) -> &EVMData {
        self.evm_data
            .as_ref()
            .expect("The EVM data must have been initialized")
    }

    ///
    /// Returns the EVM data mutable reference.
    ///
    /// # Panics
    /// If the EVM data has not been initialized.
    ///
    pub fn evm_mut(&mut self) -> &mut EVMData {
        self.evm_data
            .as_mut()
            .expect("The EVM data must have been initialized")
    }
}
