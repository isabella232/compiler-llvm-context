//!
//! The LLVM entry function.
//!

use inkwell::values::BasicValue;

use crate::context::address_space::AddressSpace;
use crate::context::function::intrinsic::Intrinsic as IntrinsicFunction;
use crate::context::Context;
use crate::Dependency;
use crate::WriteLLVM;

///
/// The LLVM entry function.
///
/// The function is a wrapper managing the constructor and selector calling logic.
///
#[derive(Debug, Default)]
pub struct Entry {}

impl Entry {
    ///
    /// Returns the constructor call flag.
    ///
    fn is_constructor_call<'ctx, D>(
        context: &mut Context<'ctx, D>,
    ) -> inkwell::values::IntValue<'ctx>
    where
        D: Dependency,
    {
        let header = context.read_header(AddressSpace::Parent);
        context.builder().build_right_shift(
            header,
            context.field_const((8 * compiler_common::BITLENGTH_BYTE) as u64),
            false,
            "header_constructor_bit",
        )
    }

    ///
    /// Returns the constructor having executed flag.
    ///
    fn read_is_executed_flag<'ctx, D>(
        context: &mut Context<'ctx, D>,
    ) -> inkwell::values::IntValue<'ctx>
    where
        D: Dependency,
    {
        let storage_key_string = compiler_common::keccak256(
            compiler_common::ABI_STORAGE_IS_CONSTRUCTOR_EXECUTED.as_bytes(),
        );
        let storage_key_value = context.field_const_str(storage_key_string.as_str());

        let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StorageLoad);
        context
            .build_call(
                intrinsic,
                &[
                    storage_key_value.as_basic_value_enum(),
                    context.field_const(0).as_basic_value_enum(),
                ],
                "is_executed_flag_load",
            )
            .expect("Contract storage always returns a value")
            .into_int_value()
    }
}

impl<D> WriteLLVM<D> for Entry
where
    D: Dependency,
{
    fn declare(&mut self, context: &mut Context<D>) -> anyhow::Result<()> {
        let function_type = context.function_type(0, vec![]);
        context.add_function(
            compiler_common::LLVM_FUNCTION_ENTRY,
            function_type,
            Some(inkwell::module::Linkage::External),
        );

        Ok(())
    }

    fn into_llvm(self, context: &mut Context<D>) -> anyhow::Result<()> {
        let function = context
            .functions
            .get(compiler_common::LLVM_FUNCTION_ENTRY)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Contract entry not found"))?;
        context.set_function(function);

        let constructor = context
            .functions
            .get(compiler_common::LLVM_FUNCTION_CONSTRUCTOR)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Contract constructor not found"))?;
        let selector = context
            .functions
            .get(compiler_common::LLVM_FUNCTION_SELECTOR)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Contract selector not found"))?;

        context.set_basic_block(context.function().entry_block);
        let is_executed_flag = Self::read_is_executed_flag(context);
        let is_executed_flag_zero = context.builder().build_int_compare(
            inkwell::IntPredicate::EQ,
            is_executed_flag,
            context.field_const(0),
            "is_executed_flag_zero",
        );
        let is_executed_flag_one = context.builder().build_int_compare(
            inkwell::IntPredicate::EQ,
            is_executed_flag,
            context.field_const(1),
            "is_executed_flag_one",
        );
        let is_constructor_call = Self::is_constructor_call(context);
        let is_constructor_call_zero = context.builder().build_int_compare(
            inkwell::IntPredicate::EQ,
            is_constructor_call,
            context.field_const(0),
            "is_constructor_call_zero",
        );
        let is_constructor_call_one = context.builder().build_int_compare(
            inkwell::IntPredicate::EQ,
            is_constructor_call,
            context.field_const(1),
            "is_constructor_call_one",
        );
        let is_error_double_constructor_call = context.builder().build_and(
            is_constructor_call_one,
            is_executed_flag_one,
            "is_error_double_constructor_call",
        );
        let is_error_expected_constructor_call = context.builder().build_and(
            is_constructor_call_zero,
            is_executed_flag_zero,
            "is_error_expected_constructor_call",
        );
        let is_constructor_call = context.builder().build_and(
            is_constructor_call_one,
            is_executed_flag_zero,
            "is_constructor_call",
        );

        let double_constructor_call_block =
            context.append_basic_block("error_double_constructor_call_block");
        let expected_constructor_call_check_block =
            context.append_basic_block("expected_constructor_call_check_block");
        let expected_constructor_call_block =
            context.append_basic_block("error_expected_constructor_call_block");
        let constructor_call_check_block =
            context.append_basic_block("constructor_call_check_block");
        let constructor_call_block = context.append_basic_block("constructor_call_block");
        let selector_call_block = context.append_basic_block("selector_call_block");

        context.build_conditional_branch(
            is_error_double_constructor_call,
            double_constructor_call_block,
            expected_constructor_call_check_block,
        );

        context.set_basic_block(double_constructor_call_block);
        context.write_error(compiler_common::ABI_ERROR_DOUBLE_CONSTRUCTOR_CALL);
        context.build_unconditional_branch(context.function().throw_block);

        context.set_basic_block(expected_constructor_call_check_block);
        context.build_conditional_branch(
            is_error_expected_constructor_call,
            expected_constructor_call_block,
            constructor_call_check_block,
        );

        context.set_basic_block(expected_constructor_call_block);
        context.write_error(compiler_common::ABI_ERROR_EXPECTED_CONSTRUCTOR_CALL);
        context.build_unconditional_branch(context.function().throw_block);

        context.set_basic_block(constructor_call_check_block);
        context.build_conditional_branch(
            is_constructor_call,
            constructor_call_block,
            selector_call_block,
        );

        context.set_basic_block(constructor_call_block);
        context.build_invoke(constructor.value, &[], "constructor_call");
        context.build_unconditional_branch(context.function().return_block);

        context.set_basic_block(selector_call_block);
        context.build_invoke(selector.value, &[], "selector_call");
        context.build_unconditional_branch(context.function().return_block);

        context.build_throw_block(false);
        context.build_catch_block(false);

        context.set_basic_block(context.function().return_block);
        context.build_return(None);

        Ok(())
    }
}
