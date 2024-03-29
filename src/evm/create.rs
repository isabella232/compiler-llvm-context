//!
//! Translates the contract creation instructions.
//!

use inkwell::values::BasicValue;

use crate::context::address_space::AddressSpace;
use crate::context::function::intrinsic::Intrinsic as IntrinsicFunction;
use crate::context::Context;
use crate::Dependency;

///
/// Translates the contract `create` instruction.
///
pub fn create<'ctx, D>(
    context: &mut Context<'ctx, D>,
    value: inkwell::values::IntValue<'ctx>,
    input_offset: inkwell::values::IntValue<'ctx>,
    input_size: inkwell::values::IntValue<'ctx>,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    create2(context, value, input_offset, input_size, None)
}

///
/// Translates the contract `create2` instruction.
///
pub fn create2<'ctx, D>(
    context: &mut Context<'ctx, D>,
    value: inkwell::values::IntValue<'ctx>,
    input_offset: inkwell::values::IntValue<'ctx>,
    input_size: inkwell::values::IntValue<'ctx>,
    salt: Option<inkwell::values::IntValue<'ctx>>,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    crate::evm::check_value_zero(context, value);

    let hash_pointer =
        context.access_memory(input_offset, AddressSpace::Heap, "create_hash_pointer");
    let hash = context.build_load(hash_pointer, "create_hash_value");

    let constructor_input_offset = context.builder().build_int_add(
        input_offset,
        context.field_const(compiler_common::SIZE_FIELD as u64),
        "create_input_offset",
    );
    let constructor_input_size = context.builder().build_int_sub(
        input_size,
        context.field_const(compiler_common::SIZE_FIELD as u64),
        "create_input_size",
    );
    let counter_value_key = context.field_const_str(
        compiler_common::keccak256(
            compiler_common::ABI_STORAGE_DEPLOYED_CONTRACTS_COUNTER.as_bytes(),
        )
        .as_str(),
    );
    let counter_value = context
        .build_call(
            context.get_intrinsic_function(IntrinsicFunction::StorageLoad),
            &[
                counter_value_key.as_basic_value_enum(),
                context.field_const(0).as_basic_value_enum(),
            ],
            "create_counter_load",
        )
        .expect("Contract storage always returns a value")
        .into_int_value();
    let salt = call_keccak256_salt(
        context,
        constructor_input_offset,
        constructor_input_size,
        counter_value,
        salt,
    )?;

    let address = call_address_precompile(context, hash.into_int_value(), salt.into_int_value())?;

    let is_call_successful = call_constructor(
        context,
        address.into_int_value(),
        constructor_input_offset,
        constructor_input_size,
    )?;

    let counter_value_incremented = context.builder().build_int_add(
        counter_value,
        context.field_const(1),
        "create_counter_value_incremented",
    );
    context.build_call(
        context.get_intrinsic_function(IntrinsicFunction::StorageStore),
        &[
            counter_value_incremented.as_basic_value_enum(),
            counter_value_key.as_basic_value_enum(),
            context.field_const(0).as_basic_value_enum(),
        ],
        "create_counter_store",
    );

    let address = context.builder().build_int_mul(
        address.into_int_value(),
        is_call_successful.into_int_value(),
        "create_address_validated",
    );

    Ok(Some(address.as_basic_value_enum()))
}

///
/// Translates the contract hash instruction, which is actually used to set the hash of the contract
/// being created, or other related auxiliary data.
///
/// `dataoffset` in Yul, `PUSH [$]` in legacy assembly.
///
pub fn contract_hash<'ctx, D>(
    context: &mut Context<'ctx, D>,
    identifier: String,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let parent = context.module().get_name().to_str().expect("Always valid");

    if identifier.ends_with("_deployed") || identifier.as_str() == parent {
        return Ok(Some(context.field_const(0).as_basic_value_enum()));
    }

    let hash_value = context
        .compile_dependency(identifier.as_str())
        .map(|hash| context.field_const_str(hash.as_str()))
        .map(inkwell::values::BasicValueEnum::IntValue)?;

    Ok(Some(hash_value))
}

///
/// Translates the contract hash size instruction, which is actually used to set the hash of the
/// contract being created, or other related auxiliary data.
///
/// `datasize` in Yul, `PUSH #[$]` in legacy assembly.
///
pub fn contract_hash_size<'ctx, D>(
    context: &mut Context<'ctx, D>,
    identifier: String,
) -> anyhow::Result<Option<inkwell::values::BasicValueEnum<'ctx>>>
where
    D: Dependency,
{
    let parent = context.module().get_name().to_str().expect("Always valid");

    if identifier.ends_with("_deployed") || identifier.as_str() == parent {
        return Ok(Some(context.field_const(0).as_basic_value_enum()));
    }

    Ok(Some(
        context
            .field_const(compiler_common::SIZE_FIELD as u64)
            .as_basic_value_enum(),
    ))
}

///
/// Gets the `keccak256` of the salt, which consists of the constructor arguments, nonce, and the
/// salt provided by Yul.
///
fn call_keccak256_salt<'ctx, D>(
    context: &mut Context<'ctx, D>,
    constructor_input_offset: inkwell::values::IntValue<'ctx>,
    constructor_input_size: inkwell::values::IntValue<'ctx>,
    counter_value: inkwell::values::IntValue<'ctx>,
    salt: Option<inkwell::values::IntValue<'ctx>>,
) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::SwitchContext);
    context.build_call(intrinsic, &[], "salt_keccak256_switch_context");

    let mut input_size = context.builder().build_int_add(
        constructor_input_size,
        context.field_const(compiler_common::SIZE_FIELD as u64),
        "salt_keccak256_input_size_with_counter",
    );
    if salt.is_some() {
        input_size = context.builder().build_int_add(
            input_size,
            context.field_const(compiler_common::SIZE_FIELD as u64),
            "salt_keccak256_input_size_with_salt",
        );
    }

    let child_pointer_header = context.access_memory(
        context.field_const(
            (compiler_common::ABI_MEMORY_OFFSET_HEADER * compiler_common::SIZE_FIELD) as u64,
        ),
        AddressSpace::Child,
        "salt_keccak256_child_pointer_header",
    );
    context.build_store(child_pointer_header, input_size);

    let child_offset_data = context.field_const(
        (compiler_common::ABI_MEMORY_OFFSET_DATA * compiler_common::SIZE_FIELD) as u64,
    );
    let child_pointer_data = context.access_memory(
        child_offset_data,
        AddressSpace::Child,
        "salt_keccak256_child_pointer_data",
    );
    let child_offset_constructor_data = child_offset_data;
    let child_pointer_constructor_data = child_pointer_data;
    let constructor_input_pointer = context.access_memory(
        constructor_input_offset,
        AddressSpace::Heap,
        "salt_keccak256_heap_pointer_constructor_data",
    );
    context.build_memcpy(
        IntrinsicFunction::MemoryCopyToChild,
        child_pointer_constructor_data,
        constructor_input_pointer,
        constructor_input_size,
        "salt_keccak256_memcpy_to_child",
    );

    let child_offset_counter = context.builder().build_int_add(
        child_offset_constructor_data,
        constructor_input_size,
        "salt_keccak256_child_offset_counter",
    );
    let child_pointer_counter = context.access_memory(
        child_offset_counter,
        AddressSpace::Child,
        "salt_keccak256_child_pointer_counter",
    );
    context.build_store(child_pointer_counter, counter_value);

    if let Some(salt) = salt {
        let child_offset_salt = context.builder().build_int_add(
            child_offset_counter,
            context.field_const(compiler_common::SIZE_FIELD as u64),
            "salt_keccak256_child_offset_salt",
        );
        let child_pointer_salt = context.access_memory(
            child_offset_salt,
            AddressSpace::Child,
            "salt_keccak256_child_pointer_salt",
        );
        context.build_store(child_pointer_salt, salt);
    }

    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::StaticCall);
    let call_definition = context.builder().build_left_shift(
        context.field_const_str(compiler_common::ABI_ADDRESS_KECCAK256),
        context.field_const((compiler_common::BITLENGTH_X32) as u64),
        "",
    );
    context.build_call(
        intrinsic,
        &[call_definition.as_basic_value_enum()],
        "salt_keccak256_call_external",
    );

    let result = context.build_load(child_pointer_data, "salt_keccak256_result");

    Ok(result)
}

///
/// Calls the `create` precompile, which returns the newly deployed contract address.
///
fn call_address_precompile<'ctx, D>(
    context: &mut Context<'ctx, D>,
    hash: inkwell::values::IntValue<'ctx>,
    salt: inkwell::values::IntValue<'ctx>,
) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::SwitchContext);
    context.build_call(intrinsic, &[], "create_precompile_switch_context");

    let child_pointer_header = context.access_memory(
        context.field_const(
            (compiler_common::ABI_MEMORY_OFFSET_HEADER * compiler_common::SIZE_FIELD) as u64,
        ),
        AddressSpace::Child,
        "create_precompile_child_pointer_header",
    );
    let input_size = context.field_const((compiler_common::SIZE_FIELD * 2) as u64);
    context.build_store(child_pointer_header, input_size);

    let child_offset_data = context.field_const(
        (compiler_common::ABI_MEMORY_OFFSET_DATA * compiler_common::SIZE_FIELD) as u64,
    );
    let child_pointer_data = context.access_memory(
        child_offset_data,
        AddressSpace::Child,
        "create_precompile_child_pointer_hash",
    );
    let child_pointer_hash = child_pointer_data;
    context.build_store(child_pointer_hash, hash);

    let child_offset_salt = context.builder().build_int_add(
        child_offset_data,
        context.field_const(compiler_common::SIZE_FIELD as u64),
        "create_precompile_child_offset_salt",
    );
    let child_pointer_salt = context.access_memory(
        child_offset_salt,
        AddressSpace::Child,
        "create_precompile_child_pointer_salt",
    );
    context.build_store(child_pointer_salt, salt);

    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::FarCall);
    let call_definition = context.builder().build_left_shift(
        context.field_const_str(compiler_common::ABI_ADDRESS_CREATE),
        context.field_const((compiler_common::BITLENGTH_X32) as u64),
        "",
    );
    context.build_call(
        intrinsic,
        &[call_definition.as_basic_value_enum()],
        "create_precompile_call_external",
    );

    let result = context.build_load(child_pointer_data, "create_precompile_result");

    Ok(result)
}

///
/// Calls the constructor of the newly deployed contract.
///
fn call_constructor<'ctx, D>(
    context: &mut Context<'ctx, D>,
    address: inkwell::values::IntValue<'ctx>,
    constructor_input_offset: inkwell::values::IntValue<'ctx>,
    constructor_input_size: inkwell::values::IntValue<'ctx>,
) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>>
where
    D: Dependency,
{
    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::SwitchContext);
    context.build_call(intrinsic, &[], "create_switch_context");

    let child_header_data = context.builder().build_or(
        constructor_input_size,
        context.field_const_str("00000000000000010000000000000000"),
        "child_header_data",
    );

    let child_pointer_header = context.access_memory(
        context.field_const(
            (compiler_common::ABI_MEMORY_OFFSET_HEADER * compiler_common::SIZE_FIELD) as u64,
        ),
        AddressSpace::Child,
        "create_child_pointer_header",
    );
    context.build_store(child_pointer_header, child_header_data);

    let destination = context.access_memory(
        context.field_const(
            (compiler_common::ABI_MEMORY_OFFSET_DATA * compiler_common::SIZE_FIELD) as u64,
        ),
        AddressSpace::Child,
        "create_child_input_destination",
    );
    let source = context.access_memory(
        constructor_input_offset,
        AddressSpace::Heap,
        "create_child_input_source",
    );

    context.build_memcpy(
        IntrinsicFunction::MemoryCopyToChild,
        destination,
        source,
        constructor_input_size,
        "create_memcpy_to_child",
    );

    let intrinsic = context.get_intrinsic_function(IntrinsicFunction::FarCall);
    let call_definition = context.builder().build_left_shift(
        address,
        context.field_const((compiler_common::BITLENGTH_X32) as u64),
        "",
    );
    let is_call_successful = context
        .build_call(
            intrinsic,
            &[call_definition.as_basic_value_enum()],
            "create_call",
        )
        .expect("IntrinsicFunction always returns a flag");

    Ok(is_call_successful)
}
