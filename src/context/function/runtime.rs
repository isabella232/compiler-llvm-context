//!
//! The LLVM runtime functions.
//!

use inkwell::types::BasicType;

use crate::context::address_space::AddressSpace;

///
/// The LLVM runtime functions.
///
#[derive(Debug)]
pub struct Runtime<'ctx> {
    /// The personality function, used for exception handling.
    pub personality: inkwell::values::FunctionValue<'ctx>,
    /// The exception throwing function.
    pub cxa_throw: inkwell::values::FunctionValue<'ctx>,
    /// The `__addmod` runtime function.
    pub addmod: inkwell::values::FunctionValue<'ctx>,
    /// The `__mulmod` runtime function.
    pub mulmod: inkwell::values::FunctionValue<'ctx>,
}

impl<'ctx> Runtime<'ctx> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        llvm: &'ctx inkwell::context::Context,
        module: &inkwell::module::Module<'ctx>,
    ) -> Self {
        let personality = module.add_function(
            compiler_common::LLVM_FUNCTION_PERSONALITY,
            llvm.i32_type().fn_type(&[], false),
            None,
        );

        let cxa_throw = module.add_function(
            compiler_common::LLVM_FUNCTION_CXA_THROW,
            llvm.void_type().fn_type(
                vec![
                    llvm.i8_type()
                        .ptr_type(AddressSpace::Stack.into())
                        .as_basic_type_enum();
                    3
                ]
                .as_slice(),
                false,
            ),
            Some(inkwell::module::Linkage::External),
        );
        cxa_throw.add_attribute(
            inkwell::attributes::AttributeLoc::Function,
            llvm.create_enum_attribute(inkwell::LLVMAttributeKindCode::NoProfile, 0),
        );

        let addmod = module.add_function(
            compiler_common::LLVM_FUNCTION_ADDMOD,
            llvm.custom_width_int_type(compiler_common::BITLENGTH_FIELD as u32)
                .fn_type(
                    vec![
                        llvm.custom_width_int_type(compiler_common::BITLENGTH_FIELD as u32)
                            .as_basic_type_enum();
                        3
                    ]
                    .as_slice(),
                    false,
                ),
            Some(inkwell::module::Linkage::External),
        );
        let mulmod = module.add_function(
            compiler_common::LLVM_FUNCTION_MULMOD,
            llvm.custom_width_int_type(compiler_common::BITLENGTH_FIELD as u32)
                .fn_type(
                    vec![
                        llvm.custom_width_int_type(compiler_common::BITLENGTH_FIELD as u32)
                            .as_basic_type_enum();
                        3
                    ]
                    .as_slice(),
                    false,
                ),
            Some(inkwell::module::Linkage::External),
        );

        Self {
            personality,
            cxa_throw,
            addmod,
            mulmod,
        }
    }
}
