//!
//! The LLVM optimizing tools.
//!

///
/// The LLVM optimizing tools.
///
#[derive(Debug)]
pub struct Optimizer<'ctx> {
    /// The middle-end optimization level.
    level_middle_end: inkwell::OptimizationLevel,
    /// The back-end optimization level.
    level_back_end: inkwell::OptimizationLevel,
    /// Whether to run the inliner.
    run_inliner: bool,
    /// The module optimization pass manager.
    pass_manager_module: Option<inkwell::passes::PassManager<inkwell::module::Module<'ctx>>>,
    /// The function optimization pass manager.
    pass_manager_function:
        Option<inkwell::passes::PassManager<inkwell::values::FunctionValue<'ctx>>>,
}

impl<'ctx> Optimizer<'ctx> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(
        level_middle_end: inkwell::OptimizationLevel,
        level_back_end: inkwell::OptimizationLevel,
        run_inliner: bool,
    ) -> Self {
        Self {
            level_middle_end,
            level_back_end,
            run_inliner,
            pass_manager_module: None,
            pass_manager_function: None,
        }
    }

    ///
    /// Sets the module which is to be optimized.
    ///
    pub fn set_module(&mut self, module: &inkwell::module::Module<'ctx>) {
        let pass_manager_builder = inkwell::passes::PassManagerBuilder::create();
        pass_manager_builder.set_optimization_level(self.level_middle_end);
        pass_manager_builder.set_disable_unroll_loops(matches!(
            self.level_middle_end,
            inkwell::OptimizationLevel::Aggressive
        ));

        let pass_manager_module = inkwell::passes::PassManager::create(());
        pass_manager_builder.populate_lto_pass_manager(
            &pass_manager_module,
            true,
            self.run_inliner,
        );
        pass_manager_builder.populate_module_pass_manager(&pass_manager_module);

        let pass_manager_function = inkwell::passes::PassManager::create(module);
        pass_manager_builder.populate_function_pass_manager(&pass_manager_function);

        self.pass_manager_module = Some(pass_manager_module);
        self.pass_manager_function = Some(pass_manager_function);
    }

    ///
    /// Returns the middle-end optimization level.
    ///
    pub fn level_middle_end(&self) -> inkwell::OptimizationLevel {
        self.level_middle_end
    }

    ///
    /// Returns the back-end optimization level.
    ///
    pub fn level_back_end(&self) -> inkwell::OptimizationLevel {
        self.level_back_end
    }

    ///
    /// Runs the optimizations on `module`.
    ///
    /// Only returns `true` if any of the passes modified the module.
    ///
    pub fn run_on_module(&self, module: &inkwell::module::Module<'ctx>) -> bool {
        self.pass_manager_module
            .as_ref()
            .expect("The module has not been set")
            .run_on(module)
    }

    ///
    /// Runs the optimizations on `function`.
    ///
    /// Only returns `true` if any of the passes modified the function.
    ///
    pub fn run_on_function(&self, function: inkwell::values::FunctionValue<'ctx>) -> bool {
        self.pass_manager_function
            .as_ref()
            .expect("The module has not been set")
            .run_on(&function)
    }
}
