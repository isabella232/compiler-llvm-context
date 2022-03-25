//!
//! The LLVM constructor function.
//!

use std::marker::PhantomData;

use inkwell::values::BasicValue;

use crate::context::code_type::CodeType;
use crate::context::function::intrinsic::Intrinsic as IntrinsicFunction;
use crate::context::Context;
use crate::Dependency;
use crate::WriteLLVM;

///
/// The LLVM constructor function.
///
#[derive(Debug)]
pub struct Constructor<B, D>
where
    B: WriteLLVM<D>,
    D: Dependency,
{
    /// The constructor AST representation.
    inner: B,
    /// The `D` phantom data.
    _pd: PhantomData<D>,
}

impl<B, D> Constructor<B, D>
where
    B: WriteLLVM<D>,
    D: Dependency,
{
    ///
    /// A shortcut constructor.
    ///
    pub fn new(inner: B) -> Self {
        Self {
            inner,
            _pd: PhantomData::default(),
        }
    }

    ///
    /// Writes the contract constructor executed flag.
    ///
    fn write_is_executed_flag(context: &mut Context<D>) {
        let storage_key_string = compiler_common::keccak256(
            compiler_common::ABI_STORAGE_IS_CONSTRUCTOR_EXECUTED.as_bytes(),
        );
        let storage_key_value = context.field_const_str(storage_key_string.as_str());

        let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StorageStore);
        context.build_call(
            intrinsic,
            &[
                context.field_const(1).as_basic_value_enum(),
                storage_key_value.as_basic_value_enum(),
                context.field_const(0).as_basic_value_enum(),
            ],
            "is_executed_flag_store",
        );
    }
}

impl<B, D> WriteLLVM<D> for Constructor<B, D>
where
    B: WriteLLVM<D>,
    D: Dependency,
{
    fn declare(&mut self, context: &mut Context<D>) -> anyhow::Result<()> {
        let function_type = context.function_type(0, vec![]);
        context.add_function(
            compiler_common::LLVM_FUNCTION_CONSTRUCTOR,
            function_type,
            Some(inkwell::module::Linkage::Private),
        );

        self.inner.declare(context)
    }

    fn into_llvm(self, context: &mut Context<D>) -> anyhow::Result<()> {
        let function = context
            .functions
            .get(compiler_common::LLVM_FUNCTION_CONSTRUCTOR)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Contract constructor not found"))?;
        context.set_function(function);

        context.set_basic_block(context.function().entry_block);
        context.code_type = Some(CodeType::Deploy);
        self.inner.into_llvm(context)?;
        match context
            .basic_block()
            .get_last_instruction()
            .map(|instruction| instruction.get_opcode())
        {
            Some(inkwell::values::InstructionOpcode::Br) => {}
            Some(inkwell::values::InstructionOpcode::Switch) => {}
            _ => context.build_unconditional_branch(context.function().return_block),
        }

        context.build_throw_block(true);
        context.build_catch_block(true);

        context.set_basic_block(context.function().return_block);
        Self::write_is_executed_flag(context);
        context.build_return(None);

        Ok(())
    }
}
