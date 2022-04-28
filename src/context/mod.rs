//!
//! The LLVM generator context.
//!

pub mod address_space;
pub mod argument;
pub mod code_type;
pub mod evm_data;
pub mod function;
pub mod r#loop;
pub mod optimizer;

use std::collections::HashMap;

use inkwell::types::BasicType;
use inkwell::values::BasicValue;

use crate::dump_flag::DumpFlag;
use crate::Dependency;

use self::address_space::AddressSpace;
use self::code_type::CodeType;
use self::evm_data::EVMData;
use self::function::evm_data::EVMData as FunctionEVMData;
use self::function::intrinsic::Intrinsic as IntrinsicFunction;
use self::function::r#return::Return as FunctionReturn;
use self::function::runtime::Runtime;
use self::function::Function;
use self::optimizer::Optimizer;
use self::r#loop::Loop;

///
/// The LLVM generator context.
///
pub struct Context<'ctx, 'dep, D>
where
    D: Dependency,
{
    /// The inner LLVM context.
    llvm: &'ctx inkwell::context::Context,
    /// The inner LLVM context builder.
    builder: inkwell::builder::Builder<'ctx>,
    /// The optimizing tools.
    optimizer: Optimizer<'ctx>,
    /// The current module.
    module: inkwell::module::Module<'ctx>,
    /// The current function.
    function: Option<Function<'ctx>>,
    /// The loop context stack.
    loop_stack: Vec<Loop<'ctx>>,

    /// The current contract code type, if known.
    pub code_type: Option<CodeType>,
    /// The runtime functions.
    pub runtime: Runtime<'ctx>,
    /// The declared functions.
    pub functions: HashMap<String, Function<'ctx>>,

    /// The project dependency manager.
    dependency_manager: Option<&'dep mut D>,
    /// Whether to dump the specified IRs.
    dump_flags: Vec<DumpFlag>,

    /// The EVM compiler data.
    evm_data: Option<EVMData<'ctx>>,
}

impl<'ctx, 'dep, D> Context<'ctx, 'dep, D>
where
    D: Dependency,
{
    /// The functions hashmap default capacity.
    const FUNCTION_HASHMAP_INITIAL_CAPACITY: usize = 64;
    /// The loop stack default capacity.
    const LOOP_STACK_INITIAL_CAPACITY: usize = 16;

    ///
    /// Initializes a new LLVM context.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        llvm: &'ctx inkwell::context::Context,
        machine: &inkwell::targets::TargetMachine,
        optimization_level_middle: inkwell::OptimizationLevel,
        optimization_level_back: inkwell::OptimizationLevel,
        module_name: &str,
        dependency_manager: Option<&'dep mut D>,
        dump_flags: Vec<DumpFlag>,
    ) -> Self {
        let module = llvm.create_module(module_name);
        module.set_triple(&machine.get_triple());
        module.set_data_layout(&machine.get_target_data().get_data_layout());

        let optimizer = Optimizer::new(&module, optimization_level_middle, optimization_level_back);

        let runtime = Runtime::new(llvm, &module);

        Self {
            llvm,
            builder: llvm.create_builder(),
            optimizer,
            module,
            function: None,
            loop_stack: Vec::with_capacity(Self::LOOP_STACK_INITIAL_CAPACITY),

            code_type: None,
            runtime,
            functions: HashMap::with_capacity(Self::FUNCTION_HASHMAP_INITIAL_CAPACITY),

            dependency_manager,
            dump_flags,

            evm_data: None,
        }
    }

    ///
    /// Initializes a new EVM LLVM context.
    ///
    #[allow(clippy::too_many_arguments)]
    pub fn new_evm(
        llvm: &'ctx inkwell::context::Context,
        machine: &inkwell::targets::TargetMachine,
        optimization_level_middle: inkwell::OptimizationLevel,
        optimization_level_back: inkwell::OptimizationLevel,
        module_name: &str,
        dependency_manager: Option<&'dep mut D>,
        dump_flags: Vec<DumpFlag>,
        evm_data: EVMData<'ctx>,
    ) -> Self {
        let mut object = Self::new(
            llvm,
            machine,
            optimization_level_middle,
            optimization_level_back,
            module_name,
            dependency_manager,
            dump_flags,
        );
        object.evm_data = Some(evm_data);
        object
    }

    ///
    /// Returns the LLVM IR builder.
    ///
    pub fn builder(&self) -> &inkwell::builder::Builder<'ctx> {
        &self.builder
    }

    ///
    /// Returns the current module reference.
    ///
    pub fn module(&self) -> &inkwell::module::Module<'ctx> {
        &self.module
    }

    ///
    /// Checks whether the specified dump flag is set.
    ///
    pub fn has_dump_flag(&self, dump_flag: DumpFlag) -> bool {
        self.dump_flags.contains(&dump_flag)
    }

    ///
    /// Optimizes the current module.
    ///
    /// Should be only run when the entire module has been translated.
    ///
    /// Only returns `true` if any of the passes modified the function.
    ///
    pub fn optimize(&self) -> bool {
        let mut is_optimized = false;

        for (_, function) in self.functions.iter() {
            is_optimized |= self.optimizer.run_on_function(function.value);
        }
        is_optimized |= self.optimizer.run_on_module(self.module());

        is_optimized
    }

    ///
    /// Verifies the current module.
    ///
    /// # Panics
    /// If verification fails.
    ///
    pub fn verify(&self) -> anyhow::Result<()> {
        self.module()
            .verify()
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    ///
    /// Compiles a contract dependency, if the dependency manager is set.
    ///
    pub fn compile_dependency(&mut self, name: &str) -> anyhow::Result<String> {
        self.dependency_manager
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("The dependency manager is unset"))
            .and_then(|manager| {
                manager.compile(
                    name,
                    self.module.get_name().to_str().expect("Always valid"),
                    self.optimizer.level_middle(),
                    self.optimizer.level_back(),
                    self.dump_flags.clone(),
                )
            })
    }

    ///
    /// Gets a deployed library address.
    ///
    pub fn resolve_library(&self, path: &str) -> anyhow::Result<inkwell::values::IntValue<'ctx>> {
        self.dependency_manager
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("The dependency manager is unset"))
            .and_then(|manager| {
                let address = manager.resolve_library(path)?;
                Ok(self.field_const_str(address.as_str()))
            })
    }

    ///
    /// Appends a function to the current module.
    ///
    pub fn add_function(
        &mut self,
        name: &str,
        r#type: inkwell::types::FunctionType<'ctx>,
        linkage: Option<inkwell::module::Linkage>,
    ) {
        let value = self.module().add_function(name, r#type, linkage);
        for index in 0..value.count_params() {
            if value
                .get_nth_param(index)
                .map(|argument| argument.get_type().is_pointer_type())
                .unwrap_or_default()
            {
                value.set_param_alignment(index, compiler_common::SIZE_FIELD as u32);
            }
        }

        value.set_personality_function(self.runtime.personality);

        let entry_block = self.llvm.append_basic_block(value, "entry");
        let throw_block = self.llvm.append_basic_block(value, "throw");
        let catch_block = self.llvm.append_basic_block(value, "catch");
        let return_block = self.llvm.append_basic_block(value, "return");

        let function = Function::new(
            name.to_owned(),
            value,
            entry_block,
            throw_block,
            catch_block,
            return_block,
            None,
        );
        self.functions.insert(name.to_string(), function.clone());
    }

    ///
    /// Appends a function to the current module.
    ///
    pub fn add_function_evm(
        &mut self,
        name: &str,
        r#type: inkwell::types::FunctionType<'ctx>,
        linkage: Option<inkwell::module::Linkage>,
        evm_data: FunctionEVMData<'ctx>,
    ) {
        self.add_function(name, r#type, linkage);
        self.functions
            .get_mut(name)
            .expect("Always exists")
            .evm_data = Some(evm_data);
    }

    ///
    /// Returns the current function.
    ///
    pub fn function(&self) -> &Function<'ctx> {
        self.function.as_ref().expect("Must be declared before use")
    }

    ///
    /// Returns the current function as a mutable reference.
    ///
    pub fn function_mut(&mut self) -> &mut Function<'ctx> {
        self.function.as_mut().expect("Must be declared before use")
    }

    ///
    /// Sets the current function.
    ///
    /// # Panics
    /// If the function with `name` does not exist.
    ///
    pub fn set_function(&mut self, function: Function<'ctx>) {
        self.function = Some(function);
    }

    ///
    /// Sets the return entity for the current function.
    ///
    pub fn set_function_return(&mut self, r#return: FunctionReturn<'ctx>) {
        let name = self.function().name.clone();

        self.functions
            .get_mut(name.as_str())
            .expect("Always exists")
            .set_return(r#return.clone());
        self.function_mut().set_return(r#return);
    }

    ///
    /// Returns the specified intrinsic function.
    ///
    pub fn get_intrinsic_function(
        &self,
        intrinsic: IntrinsicFunction,
    ) -> inkwell::values::FunctionValue<'ctx> {
        self.module()
            .get_intrinsic_function(intrinsic.name(), intrinsic.argument_types(self).as_slice())
            .unwrap_or_else(|| panic!("Intrinsic function `{}` does not exist", intrinsic.name()))
    }

    ///
    /// Appends a new basic block to the current function.
    ///
    pub fn append_basic_block(&self, name: &str) -> inkwell::basic_block::BasicBlock<'ctx> {
        self.llvm.append_basic_block(self.function().value, name)
    }

    ///
    /// Sets the current basic block.
    ///
    pub fn set_basic_block(&self, block: inkwell::basic_block::BasicBlock<'ctx>) {
        self.builder.position_at_end(block);
    }

    ///
    /// Returns the current basic block.
    ///
    pub fn basic_block(&self) -> inkwell::basic_block::BasicBlock<'ctx> {
        self.builder.get_insert_block().expect("Always exists")
    }

    ///
    /// Pushes a new loop context to the stack.
    ///
    pub fn push_loop(
        &mut self,
        body_block: inkwell::basic_block::BasicBlock<'ctx>,
        continue_block: inkwell::basic_block::BasicBlock<'ctx>,
        join_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) {
        self.loop_stack
            .push(Loop::new(body_block, continue_block, join_block));
    }

    ///
    /// Pops the current loop context from the stack.
    ///
    pub fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    ///
    /// Returns the current loop context.
    ///
    pub fn r#loop(&self) -> &Loop<'ctx> {
        self.loop_stack
            .last()
            .expect("The current context is not in a loop")
    }

    ///
    /// Builds a stack allocation instruction.
    ///
    /// Sets the alignment to 256 bits.
    ///
    pub fn build_alloca<T: BasicType<'ctx>>(
        &self,
        r#type: T,
        name: &str,
    ) -> inkwell::values::PointerValue<'ctx> {
        let pointer = self.builder.build_alloca(r#type, name);
        self.basic_block()
            .get_last_instruction()
            .expect("Always exists")
            .set_alignment(compiler_common::SIZE_FIELD as u32)
            .expect("Alignment is valid");
        pointer
    }

    ///
    /// Builds a stack store instruction.
    ///
    /// Sets the alignment to 256 bits for stack and 1 bit for heap, parent, and child.
    ///
    pub fn build_store<V: BasicValue<'ctx>>(
        &self,
        pointer: inkwell::values::PointerValue<'ctx>,
        value: V,
    ) {
        let instruction = self.builder.build_store(pointer, value);

        let alignment = if inkwell::AddressSpace::from(AddressSpace::Stack)
            == pointer.get_type().get_address_space()
        {
            compiler_common::SIZE_FIELD
        } else {
            1
        };

        instruction
            .set_alignment(alignment as u32)
            .expect("Alignment is valid");
    }

    ///
    /// Builds a stack load instruction.
    ///
    /// Sets the alignment to 256 bits for stack and 1 bit for heap, parent, and child.
    ///
    pub fn build_load(
        &self,
        pointer: inkwell::values::PointerValue<'ctx>,
        name: &str,
    ) -> inkwell::values::BasicValueEnum<'ctx> {
        let value = self.builder.build_load(pointer, name);

        let alignment = if inkwell::AddressSpace::from(AddressSpace::Stack)
            == pointer.get_type().get_address_space()
        {
            compiler_common::SIZE_FIELD
        } else {
            1
        };

        self.basic_block()
            .get_last_instruction()
            .expect("Always exists")
            .set_alignment(alignment as u32)
            .expect("Alignment is valid");
        value
    }

    ///
    /// Builds a conditional branch.
    ///
    /// Checks if there are no other terminators in the block.
    ///
    pub fn build_conditional_branch(
        &self,
        comparison: inkwell::values::IntValue<'ctx>,
        then_block: inkwell::basic_block::BasicBlock<'ctx>,
        else_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) {
        if self.basic_block().get_terminator().is_some() {
            return;
        }

        self.builder
            .build_conditional_branch(comparison, then_block, else_block);
    }

    ///
    /// Builds an unconditional branch.
    ///
    /// Checks if there are no other terminators in the block.
    ///
    pub fn build_unconditional_branch(
        &self,
        destination_block: inkwell::basic_block::BasicBlock<'ctx>,
    ) {
        if self.basic_block().get_terminator().is_some() {
            return;
        }

        self.builder.build_unconditional_branch(destination_block);
    }

    ///
    /// Builds a call.
    ///
    /// Checks if there are no other terminators in the block.
    ///
    pub fn build_call(
        &self,
        function: inkwell::values::FunctionValue<'ctx>,
        args: &[inkwell::values::BasicValueEnum<'ctx>],
        name: &str,
    ) -> Option<inkwell::values::BasicValueEnum<'ctx>> {
        let call_site_value = self.builder.build_call(function, args, name);

        if name == compiler_common::LLVM_FUNCTION_CXA_THROW {
            return call_site_value.try_as_basic_value().left();
        }

        for index in 0..function.count_params() {
            if function
                .get_nth_param(index)
                .map(|argument| argument.get_type().is_pointer_type())
                .unwrap_or_default()
            {
                call_site_value.set_alignment_attribute(
                    inkwell::attributes::AttributeLoc::Param(index),
                    compiler_common::SIZE_FIELD as u32,
                );
            }
        }

        if call_site_value
            .try_as_basic_value()
            .map_left(|value| value.is_pointer_value())
            .left_or_default()
        {
            call_site_value.set_alignment_attribute(
                inkwell::attributes::AttributeLoc::Return,
                compiler_common::SIZE_FIELD as u32,
            );
        }

        call_site_value.try_as_basic_value().left()
    }

    ///
    /// Builds an invoke.
    ///
    /// Checks if there are no other terminators in the block.
    ///
    pub fn build_invoke(
        &self,
        function: inkwell::values::FunctionValue<'ctx>,
        args: &[inkwell::values::BasicValueEnum<'ctx>],
        name: &str,
    ) -> Option<inkwell::values::BasicValueEnum<'ctx>> {
        let join_block = self.append_basic_block("join");

        let call_site_value = self.builder.build_invoke(
            function,
            args,
            join_block,
            self.function().catch_block,
            name,
        );

        for index in 0..function.count_params() {
            if function
                .get_nth_param(index)
                .map(|argument| argument.get_type().is_pointer_type())
                .unwrap_or_default()
            {
                call_site_value.set_alignment_attribute(
                    inkwell::attributes::AttributeLoc::Param(index),
                    compiler_common::SIZE_FIELD as u32,
                );
            }
        }

        if call_site_value
            .try_as_basic_value()
            .map_left(|value| value.is_pointer_value())
            .left_or_default()
        {
            call_site_value.set_alignment_attribute(
                inkwell::attributes::AttributeLoc::Return,
                compiler_common::SIZE_FIELD as u32,
            );
        }

        self.set_basic_block(join_block);

        call_site_value.try_as_basic_value().left()
    }

    ///
    /// Builds a memory copy call.
    ///
    /// Sets the alignment to 1 bit for heap, parent, and child.
    ///
    pub fn build_memcpy(
        &self,
        intrinsic: IntrinsicFunction,
        destination: inkwell::values::PointerValue<'ctx>,
        source: inkwell::values::PointerValue<'ctx>,
        size: inkwell::values::IntValue<'ctx>,
        name: &str,
    ) {
        let intrinsic = self.get_intrinsic_function(intrinsic);

        let call_site_value = self.builder.build_call(
            intrinsic,
            &[
                destination.as_basic_value_enum(),
                source.as_basic_value_enum(),
                size.as_basic_value_enum(),
                self.integer_type(compiler_common::BITLENGTH_BOOLEAN)
                    .const_zero()
                    .as_basic_value_enum(),
            ],
            name,
        );

        call_site_value.set_alignment_attribute(inkwell::attributes::AttributeLoc::Param(0), 1);
        call_site_value.set_alignment_attribute(inkwell::attributes::AttributeLoc::Param(1), 1);
    }

    ///
    /// Builds a return.
    ///
    /// Checks if there are no other terminators in the block.
    ///
    pub fn build_return(&self, value: Option<&dyn BasicValue<'ctx>>) {
        if self.basic_block().get_terminator().is_some() {
            return;
        }

        self.builder.build_return(value);
    }

    ///
    /// Builds an unreachable.
    ///
    /// Checks if there are no other terminators in the block.
    ///
    pub fn build_unreachable(&self) {
        if self.basic_block().get_terminator().is_some() {
            return;
        }

        self.builder.build_unreachable();
    }

    ///
    /// Builds an exception catching block sequence.
    ///
    pub fn build_catch_block(&self, handles_long_return: bool) {
        self.set_basic_block(self.function().catch_block);

        let landing_pad_type = self.structure_type(vec![
            self.integer_type(compiler_common::BITLENGTH_BYTE)
                .ptr_type(AddressSpace::Stack.into())
                .as_basic_type_enum(),
            self.integer_type(compiler_common::BITLENGTH_X32)
                .as_basic_type_enum(),
        ]);
        self.builder.build_landing_pad(
            landing_pad_type,
            self.runtime.personality,
            vec![self
                .integer_type(compiler_common::BITLENGTH_BYTE)
                .ptr_type(AddressSpace::Stack.into())
                .const_zero()
                .as_basic_value_enum()],
            "landing",
        );

        if handles_long_return {
            let no_long_return_block = self.append_basic_block("no_long_return_block");
            let long_return_flag_pointer = self.access_memory(
                self.field_const(
                    (compiler_common::ABI_MEMORY_OFFSET_LONG_RETURN * compiler_common::SIZE_FIELD)
                        as u64,
                ),
                AddressSpace::Heap,
                "long_return_flag_pointer",
            );
            let long_return_flag = self.build_load(long_return_flag_pointer, "long_return_flag");
            let is_long_return_flag_set = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                long_return_flag.into_int_value(),
                self.field_const(1),
                "is_long_return_flag_set",
            );
            self.build_conditional_branch(
                is_long_return_flag_set,
                self.function().return_block,
                no_long_return_block,
            );
            self.set_basic_block(no_long_return_block);
        }

        self.build_call(
            self.runtime.cxa_throw,
            vec![
                self.integer_type(compiler_common::BITLENGTH_BYTE)
                    .ptr_type(AddressSpace::Stack.into())
                    .const_null()
                    .as_basic_value_enum();
                3
            ]
            .as_slice(),
            compiler_common::LLVM_FUNCTION_CXA_THROW,
        );
        self.build_unreachable();
    }

    ///
    /// Builds an error throwing block sequence.
    ///
    pub fn build_throw_block(&self, is_upper_level: bool) {
        self.set_basic_block(self.function().throw_block);

        if is_upper_level {
            let no_long_return_block = self.append_basic_block("no_long_return_block");
            let long_return_flag_pointer = self.access_memory(
                self.field_const(
                    (compiler_common::ABI_MEMORY_OFFSET_LONG_RETURN * compiler_common::SIZE_FIELD)
                        as u64,
                ),
                AddressSpace::Heap,
                "long_return_flag_pointer",
            );
            let long_return_flag = self.build_load(long_return_flag_pointer, "long_return_flag");
            let is_long_return_flag_set = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                long_return_flag.into_int_value(),
                self.field_const(1),
                "is_long_return_flag_set",
            );
            self.build_conditional_branch(
                is_long_return_flag_set,
                self.function().return_block,
                no_long_return_block,
            );
            self.set_basic_block(no_long_return_block);
        }

        self.build_call(
            self.runtime.cxa_throw,
            vec![
                self.integer_type(compiler_common::BITLENGTH_BYTE)
                    .ptr_type(AddressSpace::Stack.into())
                    .const_null()
                    .as_basic_value_enum();
                3
            ]
            .as_slice(),
            compiler_common::LLVM_FUNCTION_CXA_THROW,
        );
        self.build_unreachable();
    }

    ///
    /// Reads the data size from the specified memory.
    ///
    pub fn read_header(&self, address_space: AddressSpace) -> inkwell::values::IntValue<'ctx> {
        let header_pointer = self.access_memory(
            self.field_const(
                (compiler_common::ABI_MEMORY_OFFSET_HEADER * compiler_common::SIZE_FIELD) as u64,
            ),
            address_space,
            "header_pointer",
        );
        self.build_load(header_pointer, "header_value")
            .into_int_value()
    }

    ///
    /// Writes the data size to the specified memory.
    ///
    pub fn write_header(
        &self,
        header: inkwell::values::IntValue<'ctx>,
        address_space: AddressSpace,
    ) {
        let header_pointer = self.access_memory(
            self.field_const(
                (compiler_common::ABI_MEMORY_OFFSET_HEADER * compiler_common::SIZE_FIELD) as u64,
            ),
            address_space,
            "header_pointer",
        );
        self.build_store(header_pointer, header);
    }

    ///
    /// Writes the error data to the parent memory.
    ///
    pub fn write_error(&self, message: &'static str) {
        self.write_header(
            self.field_const(compiler_common::SIZE_X32 as u64),
            AddressSpace::Parent,
        );

        let error_hash = compiler_common::keccak256(message.as_bytes());
        let error_code = self.field_const_str(error_hash.as_str());
        let error_code_shifted = self.builder.build_left_shift(
            error_code,
            self.field_const(
                (compiler_common::BITLENGTH_BYTE
                    * (compiler_common::SIZE_FIELD - compiler_common::SIZE_X32))
                    as u64,
            ),
            "error_code_shifted",
        );
        let parent_error_code_pointer = self.access_memory(
            self.field_const(
                (compiler_common::ABI_MEMORY_OFFSET_DATA * compiler_common::SIZE_FIELD) as u64,
            ),
            AddressSpace::Parent,
            "parent_error_code_pointer",
        );
        self.build_store(parent_error_code_pointer, error_code_shifted);
    }

    ///
    /// Returns a field type constant.
    ///
    pub fn field_const(&self, value: u64) -> inkwell::values::IntValue<'ctx> {
        self.field_type().const_int(value, false)
    }

    ///
    /// Returns a field type constant from a decimal or hexadecimal string.
    ///
    pub fn field_const_str(&self, value: &str) -> inkwell::values::IntValue<'ctx> {
        match value.strip_prefix("0x") {
            Some(hexadecimal) => self.field_const_str_hex(hexadecimal),
            None => self.field_const_str_hex(value),
        }
    }

    ///
    /// Returns a field type constant from a hexadecimal string.
    ///
    pub fn field_const_str_dec(&self, value: &str) -> inkwell::values::IntValue<'ctx> {
        self.field_type()
            .const_int_from_string(value, inkwell::types::StringRadix::Decimal)
            .unwrap_or_else(|| panic!("Invalid string constant `{}`", value))
    }

    ///
    /// Returns a field type constant from a hexadecimal string.
    ///
    pub fn field_const_str_hex(&self, value: &str) -> inkwell::values::IntValue<'ctx> {
        self.field_type()
            .const_int_from_string(
                value.strip_prefix("0x").unwrap_or(value),
                inkwell::types::StringRadix::Hexadecimal,
            )
            .unwrap_or_else(|| panic!("Invalid string constant `{}`", value))
    }

    ///
    /// Returns the void type.
    ///
    pub fn void_type(&self) -> inkwell::types::VoidType<'ctx> {
        self.llvm.void_type()
    }

    ///
    /// Returns the integer type of the specified bitlength.
    ///
    pub fn integer_type(&self, bitlength: usize) -> inkwell::types::IntType<'ctx> {
        self.llvm.custom_width_int_type(bitlength as u32)
    }

    ///
    /// Returns the default field type.
    ///
    pub fn field_type(&self) -> inkwell::types::IntType<'ctx> {
        self.llvm
            .custom_width_int_type(compiler_common::BITLENGTH_FIELD as u32)
    }

    ///
    /// Returns the structure type with specified fields.
    ///
    pub fn structure_type(
        &self,
        field_types: Vec<inkwell::types::BasicTypeEnum<'ctx>>,
    ) -> inkwell::types::StructType<'ctx> {
        self.llvm.struct_type(field_types.as_slice(), false)
    }

    ///
    /// Returns the function type for the specified parameters.
    ///
    pub fn function_type(
        &self,
        return_values_length: usize,
        mut argument_types: Vec<inkwell::types::BasicTypeEnum<'ctx>>,
    ) -> inkwell::types::FunctionType<'ctx> {
        match return_values_length {
            0 => self
                .llvm
                .void_type()
                .fn_type(argument_types.as_slice(), false),
            1 => self.field_type().fn_type(argument_types.as_slice(), false),
            length => {
                let return_types: Vec<_> = vec![self.field_type().as_basic_type_enum(); length];
                let return_type = self
                    .llvm
                    .struct_type(return_types.as_slice(), false)
                    .ptr_type(AddressSpace::Stack.into());
                argument_types.insert(0, return_type.as_basic_type_enum());
                return_type.fn_type(argument_types.as_slice(), false)
            }
        }
    }

    ///
    /// Returns the memory pointer to `address_space` at `offset` bytes.
    ///
    pub fn access_memory(
        &self,
        offset: inkwell::values::IntValue<'ctx>,
        address_space: AddressSpace,
        name: &str,
    ) -> inkwell::values::PointerValue<'ctx> {
        self.builder.build_int_to_ptr(
            offset,
            self.field_type().ptr_type(address_space.into()),
            name,
        )
    }

    ///
    /// Returns a contract context value.
    ///
    pub fn access_context(
        &self,
        context_value: compiler_common::ContextValue,
    ) -> anyhow::Result<inkwell::values::BasicValueEnum<'ctx>> {
        let intrinsic = self.get_intrinsic_function(IntrinsicFunction::GetFromContext);
        let value = self
            .build_call(
                intrinsic,
                &[self.field_const(context_value.into()).as_basic_value_enum()],
                "context_get_call",
            )
            .expect("Contract context always returns a value");
        Ok(value)
    }

    ///
    /// Returns the EVM data reference.
    ///
    /// # Panics
    /// If the EVM data has not been initialized.
    ///
    pub fn evm(&self) -> &EVMData<'ctx> {
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
    pub fn evm_mut(&mut self) -> &mut EVMData<'ctx> {
        self.evm_data
            .as_mut()
            .expect("The EVM data must have been initialized")
    }
}
