//!
//! The LLVM context library.
//!

pub(crate) mod context;
pub(crate) mod dump_flag;
pub(crate) mod evm;

pub use self::context::address_space::AddressSpace;
pub use self::context::argument::Argument;
pub use self::context::code_type::CodeType;
pub use self::context::function::constructor::Constructor as ConstructorFunction;
pub use self::context::function::entry::Entry as EntryFunction;
pub use self::context::function::intrinsic::Intrinsic as IntrinsicFunction;
pub use self::context::function::r#return::Return as FunctionReturn;
pub use self::context::function::runtime::Runtime;
pub use self::context::function::selector::Selector as SelectorFunction;
pub use self::context::function::Function;
pub use self::context::optimizer::Optimizer;
pub use self::context::r#loop::Loop;
pub use self::context::Context;
pub use self::dump_flag::DumpFlag;
pub use self::evm::arithmetic;
pub use self::evm::bitwise;
pub use self::evm::calldata;
pub use self::evm::comparison;
pub use self::evm::context as contract_context;
pub use self::evm::contract;
pub use self::evm::create;
pub use self::evm::event;
pub use self::evm::hash;
pub use self::evm::math;
pub use self::evm::memory;
pub use self::evm::r#return;
pub use self::evm::return_data;
pub use self::evm::storage;

///
/// Implemented by items which are translated into LLVM IR.
///
#[allow(clippy::upper_case_acronyms)]
pub trait WriteLLVM<D>
where
    D: Dependency,
{
    ///
    /// Makes the required preparations in the LLVM IR.
    ///
    fn prepare(_context: &mut Context<D>) -> anyhow::Result<()> {
        Ok(())
    }

    ///
    /// Declares the entity in the LLVM IR.
    /// Is usually performed in order to use the item before defining it.
    ///
    fn declare(&mut self, _context: &mut Context<D>) -> anyhow::Result<()> {
        Ok(())
    }

    ///
    /// Translates the entity into LLVM IR.
    ///
    fn into_llvm(self, context: &mut Context<D>) -> anyhow::Result<()>;
}

///
/// Implemented by items managing project dependencies.
///
pub trait Dependency {
    ///
    /// Compiles a project dependency.
    ///
    fn compile(
        &mut self,
        name: &str,
        parent_name: &str,
        optimization_level_middle: inkwell::OptimizationLevel,
        optimization_level_back: inkwell::OptimizationLevel,
        dump_flags: Vec<DumpFlag>,
    ) -> anyhow::Result<String>;

    ///
    /// Resolves a library address.
    ///
    fn resolve_library(&self, path: &str) -> anyhow::Result<String>;
}
