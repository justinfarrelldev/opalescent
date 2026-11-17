//! Private test-build result ABI, independent of ordinary application errors.
//!
//! Completed carries the *unchanged* original result (including its ordinary
//! error ABI, when applicable). TestAborted carries only a failure identity;
//! its application slot is undefined and must never be read. Call lowering must
//! branch on the abort tag before extracting an application result. This ABI is
//! selected by compiler artifact metadata, never guessed from a struct shape.
use super::context::CodegenContext;
use super::error::CodegenError;
use inkwell::basic_block::BasicBlock;
use inkwell::context::Context;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValueEnum, IntValue, StructValue};

/// Typed layout of one test-build function's internal result channel.
#[derive(Clone, Copy)]
pub struct NativeOutcomeType<'context> {
    /// Abort tag, failure identity, and optional unchanged application result.
    llvm_type: StructType<'context>,
    /// Original LLVM return type; void has no fabricated result field.
    application_type: Option<BasicTypeEnum<'context>>,
}

/// Explicit control-flow edges after a native test-build call.
///
/// The abort edge is intentionally unfinished so existing lexical cleanup and
/// RC emission can run before propagating its originating failure identity.
pub struct NativeCallBranches<'context> {
    /// Caller must emit its ordinary scope exits and internal abort return here.
    pub aborted: BasicBlock<'context>,
    /// Builder is positioned here; application results may now be extracted.
    pub completed: BasicBlock<'context>,
    /// Original failure ID, defined on and usable from the abort edge only.
    pub failure_identity: IntValue<'context>,
}

impl<'context> NativeOutcomeType<'context> {
    /// Define a test-only return shape without modifying production ABI helpers.
    #[must_use]
    pub fn new(
        context: &'context Context,
        application_type: Option<BasicTypeEnum<'context>>,
    ) -> Self {
        let mut fields = vec![context.bool_type().into(), context.i64_type().into()];
        fields.extend(application_type);
        Self {
            llvm_type: context.struct_type(&fields, false),
            application_type,
        }
    }

    /// Return the exact LLVM type used for native function declarations.
    #[must_use]
    pub const fn llvm_type(self) -> StructType<'context> {
        self.llvm_type
    }

    /// Complete normally, preserving any existing application error aggregate.
    ///
    /// # Errors
    /// Rejects a value whose shape differs from the original function result.
    pub fn build_completed(
        self,
        codegen: &CodegenContext<'context>,
        application: Option<BasicValueEnum<'context>>,
    ) -> Result<StructValue<'context>, CodegenError> {
        if application.map(|value| value.get_type()) != self.application_type {
            return Err(CodegenError::new(
                "native completed result does not match the original application ABI",
            ));
        }
        self.build(
            codegen,
            false,
            codegen.context.i64_type().const_zero(),
            application,
        )
    }

    /// Abort with an existing nonzero failure identity and no application value.
    /// The inactive application slot stays `undef`, including for non-fallible
    /// numeric, pointer, aggregate and resource-producing functions.
    ///
    /// # Errors
    /// Rejects an incorrectly typed identity or the reserved constant zero ID.
    pub fn build_aborted(
        self,
        codegen: &CodegenContext<'context>,
        failure: IntValue<'context>,
    ) -> Result<StructValue<'context>, CodegenError> {
        if failure.get_type() != codegen.context.i64_type()
            || failure.get_zero_extended_constant() == Some(0)
        {
            return Err(CodegenError::new(
                "native abort requires a nonzero i64 failure identity",
            ));
        }
        self.build(codegen, true, failure, None)
    }

    /// Inspect the internal tag before any application error/return handling.
    ///
    /// # Errors
    /// Rejects a value with another function's result layout.
    pub fn aborted_flag(
        self,
        codegen: &CodegenContext<'context>,
        result: StructValue<'context>,
    ) -> Result<IntValue<'context>, CodegenError> {
        Ok(self.extract(codegen, result, 0)?.into_int_value())
    }

    /// Preserve the originating failure identity while unwinding another call.
    ///
    /// # Errors
    /// Rejects a value with another function's result layout.
    pub fn failure_identity(
        self,
        codegen: &CodegenContext<'context>,
        result: StructValue<'context>,
    ) -> Result<IntValue<'context>, CodegenError> {
        Ok(self.extract(codegen, result, 1)?.into_int_value())
    }

    /// Extract the unchanged application result on the **completed edge only**.
    /// Callers must have branched on [`Self::aborted_flag`] first. A void result
    /// returns `None`; no zero, null or empty source value is manufactured.
    ///
    /// # Errors
    /// Rejects a value with another function's result layout.
    pub fn completed_application_value(
        self,
        codegen: &CodegenContext<'context>,
        result: StructValue<'context>,
    ) -> Result<Option<BasicValueEnum<'context>>, CodegenError> {
        self.validate_layout(result)?;
        self.application_type
            .map(|_| self.extract(codegen, result, 2))
            .transpose()
    }

    /// Split a native call before ordinary `guard`, `propagate` or result use.
    /// Leaves the builder on the completed edge and exposes an unfinished abort
    /// edge for the caller's existing lexical cleanup machinery.
    ///
    /// # Errors
    /// Rejects mismatched layouts or a missing/terminated insertion block.
    pub fn branch_after_call(
        self,
        codegen: &CodegenContext<'context>,
        result: StructValue<'context>,
    ) -> Result<NativeCallBranches<'context>, CodegenError> {
        let block = codegen.builder.get_insert_block().ok_or_else(|| {
            CodegenError::new("native outcome branch requires an insertion block")
        })?;
        if block.get_terminator().is_some() {
            return Err(CodegenError::new(
                "native outcome branch cannot follow a terminated block",
            ));
        }
        let function = block.get_parent().ok_or_else(|| {
            CodegenError::new("native outcome branch requires an enclosing function")
        })?;
        let tag = self.aborted_flag(codegen, result)?;
        let aborted = codegen.context.append_basic_block(function, "test.aborted");
        let completed = codegen
            .context
            .append_basic_block(function, "test.completed");
        codegen
            .builder
            .build_conditional_branch(tag, aborted, completed)?;
        codegen.builder.position_at_end(aborted);
        let failure_identity = self.failure_identity(codegen, result)?;
        codegen.builder.position_at_end(completed);
        Ok(NativeCallBranches {
            aborted,
            completed,
            failure_identity,
        })
    }

    /// Populate only fields that belong to the selected outcome variant.
    fn build(
        self,
        codegen: &CodegenContext<'context>,
        aborted: bool,
        failure: IntValue<'context>,
        application: Option<BasicValueEnum<'context>>,
    ) -> Result<StructValue<'context>, CodegenError> {
        let mut result = self.llvm_type.get_undef();
        result = codegen
            .builder
            .build_insert_value(
                result,
                codegen
                    .context
                    .bool_type()
                    .const_int(u64::from(aborted), false),
                0,
                "test_outcome.tag",
            )?
            .into_struct_value();
        result = codegen
            .builder
            .build_insert_value(result, failure, 1, "test_outcome.failure")?
            .into_struct_value();
        if let Some(value) = application {
            result = codegen
                .builder
                .build_insert_value(result, value, 2, "test_outcome.application")?
                .into_struct_value();
        }
        Ok(result)
    }

    /// Check layout before applying known field indices/types.
    fn validate_layout(self, result: StructValue<'context>) -> Result<(), CodegenError> {
        if result.get_type() == self.llvm_type {
            Ok(())
        } else {
            Err(CodegenError::new(
                "native outcome layout does not match its declared function ABI",
            ))
        }
    }

    /// Read one metadata-selected field without guessing aggregate conventions.
    fn extract(
        self,
        codegen: &CodegenContext<'context>,
        result: StructValue<'context>,
        field: u32,
    ) -> Result<BasicValueEnum<'context>, CodegenError> {
        self.validate_layout(result)?;
        Ok(codegen
            .builder
            .build_extract_value(result, field, "test_outcome.field")?)
    }
}
