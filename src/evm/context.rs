//!
//! Translates the contract context getter calls.
//!

use inkwell::values::BasicValue;

use crate::context::function::intrinsic::Intrinsic as IntrinsicFunction;
use crate::context::Context;
use crate::Dependency;

///
/// Translates the contract context getter calls.
///
pub fn get<'ctx, D>(
    context: &mut Context<'ctx, D>,
    context_value: compiler_common::ContextValue,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::GetFromContext);
    let value = context
        .build_call(
            intrinsic,
            &[context
                .field_const(context_value.into())
                .as_basic_value_enum()],
            "context_get_call",
        )
        .expect("Contract context always returns a value");
    Ok(Some(value))
}
