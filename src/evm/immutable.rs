//!
//! Translates the contract immutable operations.
//!

use inkwell::values::BasicValue;

use crate::context::Context;
use crate::Dependency;

///
/// Translates the contract immutable load.
///
pub fn load<'ctx, 'dep, D>(
    context: &mut Context<'ctx, 'dep, D>,
    key: String,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let position = context.field_const_str(compiler_common::keccak256(key.as_bytes()).as_str());
    let value = context
        .build_call(
            context.runtime.storage_load,
            &[position.as_basic_value_enum()],
            "immutable_load",
        )
        .expect("Contract storage always returns a value");
    Ok(Some(value))
}

///
/// Translates the contract immutable store.
///
pub fn store<'ctx, 'dep, D>(
    context: &mut Context<'ctx, 'dep, D>,
    key: String,
    value: inkwell::values::IntValue<'ctx>,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let position = context.field_const_str(compiler_common::keccak256(key.as_bytes()).as_str());
    context.build_call(
        context.runtime.storage_store,
        &[value.as_basic_value_enum(), position.as_basic_value_enum()],
        "immutable_store",
    );
    Ok(None)
}