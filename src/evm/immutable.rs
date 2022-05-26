//!
//! Translates the contract immutable operations.
//!

use inkwell::values::BasicValue;

use crate::context::function::intrinsic::Intrinsic as IntrinsicFunction;
use crate::context::Context;
use crate::Dependency;

///
/// Translates the contract immutable load.
///
pub fn load<'ctx, D>(
    context: &mut Context<'ctx, D>,
    key: String,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StorageLoad);

    let position = context.field_const_str(compiler_common::keccak256(key.as_bytes()).as_str());
    let is_external_storage = context.field_const(0);
    let value = context
        .build_call(
            intrinsic,
            &[
                position.as_basic_value_enum(),
                is_external_storage.as_basic_value_enum(),
            ],
            "immutable_load",
        )
        .expect("Contract storage always returns a value");
    Ok(Some(value))
}

///
/// Translates the contract immutable store.
///
pub fn store<'ctx, D>(
    context: &mut Context<'ctx, D>,
    key: String,
    value: inkwell::values::IntValue<'ctx>,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StorageStore);

    let position = context.field_const_str(compiler_common::keccak256(key.as_bytes()).as_str());
    let is_external_storage = context.field_const(0);
    context.build_call(
        intrinsic,
        &[
            value.as_basic_value_enum(),
            position.as_basic_value_enum(),
            is_external_storage.as_basic_value_enum(),
        ],
        "immutable_store",
    );
    Ok(None)
}
