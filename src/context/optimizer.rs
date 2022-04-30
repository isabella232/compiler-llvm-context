//!
//! The LLVM optimizing tools.
//!

///
/// The LLVM optimizing tools.
///
#[derive(Debug)]
pub struct Optimizer<'ctx> {
    /// The middle-end optimization level.
    level_middle: inkwell::OptimizationLevel,
    /// The back-end optimization level.
    level_back: inkwell::OptimizationLevel,
    /// The module optimization pass manager.
    pass_manager_module: inkwell::passes::PassManager<inkwell::module::Module<'ctx>>,
    /// The function optimization pass manager.
    pass_manager_function: inkwell::passes::PassManager<inkwell::values::FunctionValue<'ctx>>,
}

impl<'ctx> Optimizer<'ctx> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        module: &inkwell::module::Module<'ctx>,
        level_middle: inkwell::OptimizationLevel,
        level_back: inkwell::OptimizationLevel,
    ) -> Self {
        let internalize = matches!(level_middle, inkwell::OptimizationLevel::Aggressive);
        let run_inliner = matches!(level_middle, inkwell::OptimizationLevel::Aggressive);

        let pass_manager_builder = inkwell::passes::PassManagerBuilder::create();
        pass_manager_builder.set_optimization_level(level_middle);
        pass_manager_builder.set_disable_unroll_loops(matches!(
            level_middle,
            inkwell::OptimizationLevel::Aggressive
        ));

        let pass_manager_module = inkwell::passes::PassManager::create(());
        pass_manager_builder.populate_lto_pass_manager(
            &pass_manager_module,
            internalize,
            run_inliner,
        );
        pass_manager_builder.populate_module_pass_manager(&pass_manager_module);

        let pass_manager_function = inkwell::passes::PassManager::create(module);
        pass_manager_builder.populate_function_pass_manager(&pass_manager_function);

        Self {
            level_middle,
            level_back,
            pass_manager_module,
            pass_manager_function,
        }
    }

    ///
    /// Returns the middle-end optimization level.
    ///
    pub fn level_middle(&self) -> inkwell::OptimizationLevel {
        self.level_middle
    }

    ///
    /// Returns the back-end optimization level.
    ///
    pub fn level_back(&self) -> inkwell::OptimizationLevel {
        self.level_back
    }

    ///
    /// Runs the optimizations on `module`.
    ///
    /// Only returns `true` if any of the passes modified the module.
    ///
    pub fn run_on_module(&self, module: &inkwell::module::Module<'ctx>) -> bool {
        self.pass_manager_module.run_on(module)
    }

    ///
    /// Runs the optimizations on `function`.
    ///
    /// Only returns `true` if any of the passes modified the function.
    ///
    pub fn run_on_function(&self, function: inkwell::values::FunctionValue<'ctx>) -> bool {
        self.pass_manager_function.run_on(&function)
    }
}
