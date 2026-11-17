//! Explicit function ABI metadata for test-build source lowering.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::native_outcome::NativeOutcomeType;
use inkwell::module::Linkage;
use inkwell::types::{BasicTypeEnum, FunctionType};
use inkwell::values::{BasicValueEnum, FunctionValue, InstructionValue};

/// Original source ABI and its independent internal test outcome.
#[derive(Clone, Copy)]
pub(super) struct NativeFunctionAbi<'context> {
    /// Full original signature, including ordinary application errors.
    original: FunctionType<'context>,
    /// Metadata-selected test-only return channel.
    outcome: NativeOutcomeType<'context>,
}

impl<'context> CodegenContext<'context> {
    /// Declare one source function in test ABI without changing normal functions.
    ///
    /// # Errors
    /// Rejects conflicting declarations and unsupported variadic call paths.
    pub fn declare_native_test_function(
        &self,
        name: &str,
        original: FunctionType<'context>,
        linkage: Option<Linkage>,
    ) -> Result<FunctionValue<'context>, CodegenError> {
        if original.is_var_arg() {
            return Err(CodegenError::new(
                "variadic foreign calls cannot use the native test outcome ABI",
            ));
        }
        if let Some(function) = self.module.get_function(name) {
            if self
                .native_functions
                .borrow()
                .get(name)
                .is_some_and(|metadata| metadata.original == original)
            {
                return Ok(function);
            }
            return Err(CodegenError::new(
                "native test function conflicts with an existing ABI declaration",
            ));
        }
        let outcome = NativeOutcomeType::new(self.context, original.get_return_type());
        let parameters = original.get_param_types();
        let function = self.module.add_function(
            name,
            outcome.llvm_type().fn_type(&parameters, false),
            linkage,
        );
        self.native_functions
            .borrow_mut()
            .insert(name.to_owned(), NativeFunctionAbi { original, outcome });
        Ok(function)
    }

    /// Look up only explicitly declared test-build function outcome metadata.
    #[must_use]
    pub fn native_function_outcome(
        &self,
        function: FunctionValue<'context>,
    ) -> Option<NativeOutcomeType<'context>> {
        self.native_functions
            .borrow()
            .get(function.get_name().to_string_lossy().as_ref())
            .map(|metadata| metadata.outcome)
    }

    /// Inspect the original result without mistaking an outcome for an error ABI.
    #[must_use]
    pub fn application_return_type(
        &self,
        function: FunctionValue<'context>,
    ) -> Option<BasicTypeEnum<'context>> {
        self.native_functions
            .borrow()
            .get(function.get_name().to_string_lossy().as_ref())
            .map_or_else(
                || function.get_type().get_return_type(),
                |metadata| metadata.original.get_return_type(),
            )
    }

    /// Emit an ordinary source return, preserving test/application separation.
    /// Existing source scope exits must have run before calling this routine.
    ///
    /// # Errors
    /// Rejects a missing insertion function or an incorrect application result.
    pub fn emit_application_return(
        &self,
        value: Option<BasicValueEnum<'context>>,
    ) -> Result<InstructionValue<'context>, CodegenError> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| {
                CodegenError::new("application return requires an enclosing function")
            })?;
        if let Some(outcome) = self.native_function_outcome(function) {
            let completed = outcome.build_completed(self, value)?;
            return Ok(self.builder.build_return(Some(&completed))?);
        }
        match value {
            Some(value) => Ok(self.builder.build_return(Some(&value))?),
            None => Ok(self.builder.build_return(None)?),
        }
    }
}
