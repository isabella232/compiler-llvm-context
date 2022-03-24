//!
//! The LLVM selector function.
//!

use std::marker::PhantomData;

use crate::context::code_type::CodeType;
use crate::context::Context;
use crate::Dependency;
use crate::WriteLLVM;

///
/// The LLVM selector function.
///
#[derive(Debug, Default)]
pub struct Selector<B, D>
where
    B: WriteLLVM<D>,
    D: Dependency,
{
    /// The selector AST representation.
    inner: B,
    /// The `D` phantom data.
    _pd: PhantomData<D>,
}

impl<B, D> Selector<B, D>
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
}

impl<B, D> WriteLLVM<D> for Selector<B, D>
where
    B: WriteLLVM<D>,
    D: Dependency,
{
    fn prepare(context: &mut Context<D>) -> anyhow::Result<()> {
        let function_type = context.function_type(0, vec![]);
        context.add_function(
            compiler_common::LLVM_FUNCTION_SELECTOR,
            function_type,
            Some(inkwell::module::Linkage::Private),
        );

        Ok(())
    }

    fn declare(&mut self, context: &mut Context<D>) -> anyhow::Result<()> {
        self.inner.declare(context)
    }

    fn into_llvm(self, context: &mut Context<D>) -> anyhow::Result<()> {
        let function = context
            .functions
            .get(compiler_common::LLVM_FUNCTION_SELECTOR)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Contract selector not found"))?;
        context.set_function(function);

        context.set_basic_block(context.function().entry_block);
        context.code_type = Some(CodeType::Runtime);
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
        context.build_return(None);

        Ok(())
    }
}
