//!
//! Translates the contract storage operations.
//!

use inkwell::values::BasicValue;

use crate::context::function::intrinsic::Intrinsic as IntrinsicFunction;
use crate::context::Context;
use crate::Dependency;

///
/// Translates the contract storage load.
///
pub fn load<'ctx, D>(
    context: &mut Context<'ctx, D>,
    arguments: [inkwell::values::BasicValueEnum<'ctx>; 1],
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StorageLoad);

    let position = arguments[0];
    let is_external_storage = context.field_const(0);
    let value = context
        .build_call(
            intrinsic,
            &[position, is_external_storage.as_basic_value_enum()],
            "storage_load",
        )
        .expect("Contract storage always returns a value");
    Ok(Some(value))
}

///
/// Translates the contract storage store.
///
pub fn store<'ctx, D>(
    context: &mut Context<'ctx, D>,
    arguments: [inkwell::values::BasicValueEnum<'ctx>; 2],
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StorageStore);

    let position = arguments[0];
    let value = arguments[1];
    let is_external_storage = context.field_const(0);
    context.build_call(
        intrinsic,
        &[value, position, is_external_storage.as_basic_value_enum()],
        "storage_store",
    );
    Ok(None)
}
