extern crate alloc;

use crate::codegen::error::CodegenError;
use inkwell::builder::Builder;
use inkwell::module::{Linkage, Module};
use inkwell::values::{BasicMetadataValueEnum, IntValue, PointerValue};
use inkwell::AddressSpace;

/// Emits RC runtime calls for strong and weak reference operations.
///
/// Memory model notes:
/// - Strong references use `opal_rc_*` and track payload liveness.
/// - Weak references use `opal_weak_*` and point at the RC header.
/// - `opal_weak_upgrade` returns payload `i8*` when alive, otherwise null.
pub struct RcEmitter<'builder, 'context> {
    /// LLVM instruction builder for emitting IR.
    builder: &'builder Builder<'context>,
    /// LLVM module for function declarations.
    module: &'builder Module<'context>,
}

impl<'builder, 'context> RcEmitter<'builder, 'context> {
    /// Creates a new RC emitter.
    #[must_use]
    pub const fn new(
        builder: &'builder Builder<'context>,
        module: &'builder Module<'context>,
    ) -> Self {
        Self { builder, module }
    }

    /// Emits an RC increment call.
    pub fn emit_inc(&self, ptr_value: PointerValue<'context>) -> Result<(), CodegenError> {
        let function = self.declare_or_get("opal_rc_inc");
        let casted = self.cast_to_i8_ptr(ptr_value, "rc.inc.cast")?;
        let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
        let _call = self.builder.build_call(function, &args, "rc.inc")?;
        Ok(())
    }

    /// Emits an RC decrement call.
    pub fn emit_dec(&self, ptr_value: PointerValue<'context>) -> Result<(), CodegenError> {
        let function = self.declare_or_get("opal_rc_dec");
        let casted = self.cast_to_i8_ptr(ptr_value, "rc.dec.cast")?;
        let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
        let _call = self.builder.build_call(function, &args, "rc.dec")?;
        Ok(())
    }

    /// Emits an RC iterative drop call.
    pub fn emit_drop(&self, ptr_value: PointerValue<'context>) -> Result<(), CodegenError> {
        let function = self.declare_or_get("opal_rc_drop_iterative");
        let casted = self.cast_to_i8_ptr(ptr_value, "rc.drop.cast")?;
        let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
        let _call = self.builder.build_call(function, &args, "rc.drop")?;
        Ok(())
    }

    /// Create a weak reference from a strong RC payload pointer.
    pub fn emit_weak_alloc(
        &self,
        strong_ptr: PointerValue<'context>,
    ) -> Result<PointerValue<'context>, CodegenError> {
        let function = self.declare_or_get_weak_alloc();
        let casted = self.cast_to_i8_ptr(strong_ptr, "weak.alloc.cast")?;
        let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
        let call_site = self.builder.build_call(function, &args, "weak.alloc")?;
        let returned = call_site.try_as_basic_value().basic().ok_or_else(|| {
            CodegenError::new(alloc::string::String::from(
                "opal_weak_alloc returned no value",
            ))
        })?;
        Ok(returned.into_pointer_value())
    }

    /// Attempt to upgrade a weak reference to a strong payload pointer.
    pub fn emit_weak_upgrade(
        &self,
        weak_ptr: PointerValue<'context>,
    ) -> Result<PointerValue<'context>, CodegenError> {
        let function = self.declare_or_get_weak_upgrade();
        let casted = self.cast_to_i8_ptr(weak_ptr, "weak.upgrade.cast")?;
        let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
        let call_site = self.builder.build_call(function, &args, "weak.upgrade")?;
        let returned = call_site.try_as_basic_value().basic().ok_or_else(|| {
            CodegenError::new(alloc::string::String::from(
                "opal_weak_upgrade returned no value",
            ))
        })?;
        Ok(returned.into_pointer_value())
    }

    /// Decrement and release a weak reference handle.
    pub fn emit_weak_dec(&self, weak_ptr: PointerValue<'context>) -> Result<(), CodegenError> {
        let function = self.declare_or_get_weak_dec();
        let casted = self.cast_to_i8_ptr(weak_ptr, "weak.dec.cast")?;
        let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
        let _call = self.builder.build_call(function, &args, "weak.dec")?;
        Ok(())
    }

    /// Allocate an RC-tracked payload via runtime `opal_rc_alloc`.
    ///
    /// `drop_fn_ptr` is passed as an opaque pointer argument (`i8*`), matching
    /// runtime C ABI expectations for function pointers.
    pub fn emit_alloc(
        &self,
        payload_size: IntValue<'context>,
        drop_fn_ptr: Option<PointerValue<'context>>,
    ) -> Result<PointerValue<'context>, CodegenError> {
        let function = self.declare_or_get_alloc();
        let i8_ptr = self
            .module
            .get_context()
            .i8_type()
            .ptr_type(AddressSpace::default());
        let drop_ptr = drop_fn_ptr.unwrap_or_else(|| i8_ptr.const_null());
        let args: [BasicMetadataValueEnum<'context>; 2] = [payload_size.into(), drop_ptr.into()];
        let call_site = self.builder.build_call(function, &args, "rc.alloc")?;
        let returned = call_site.try_as_basic_value().basic().ok_or_else(|| {
            CodegenError::new(alloc::string::String::from(
                "opal_rc_alloc returned no value",
            ))
        })?;
        Ok(returned.into_pointer_value())
    }

    /// Cast a pointer value to `i8*` for C ABI compatibility.
    fn cast_to_i8_ptr(
        &self,
        ptr_value: PointerValue<'context>,
        name: &str,
    ) -> Result<PointerValue<'context>, CodegenError> {
        let i8_ptr = self
            .module
            .get_context()
            .i8_type()
            .ptr_type(AddressSpace::default());
        self.builder
            .build_pointer_cast(ptr_value, i8_ptr, name)
            .map_err(CodegenError::from)
    }

    /// Declare or get an existing RC function by name.
    fn declare_or_get(&self, name: &str) -> inkwell::values::FunctionValue<'context> {
        if let Some(existing) = self.module.get_function(name) {
            return existing;
        }

        let ctx = self.module.get_context();
        let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
        let fn_type = ctx.void_type().fn_type(&[i8_ptr.into()], false);
        self.module.add_function(name, fn_type, Some(Linkage::External))
    }

    /// Declare or get the `opal_rc_alloc` function.
    fn declare_or_get_alloc(&self) -> inkwell::values::FunctionValue<'context> {
        if let Some(existing) = self.module.get_function("opal_rc_alloc") {
            return existing;
        }

        let ctx = self.module.get_context();
        let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
        let i64_type = ctx.i64_type();
        let fn_type = i8_ptr.fn_type(&[i64_type.into(), i8_ptr.into()], false);
        self.module
            .add_function("opal_rc_alloc", fn_type, Some(Linkage::External))
    }

    /// Declare or get the `opal_weak_alloc` function.
    fn declare_or_get_weak_alloc(&self) -> inkwell::values::FunctionValue<'context> {
        if let Some(existing) = self.module.get_function("opal_weak_alloc") {
            return existing;
        }

        let ctx = self.module.get_context();
        let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
        self.module
            .add_function("opal_weak_alloc", fn_type, Some(Linkage::External))
    }

    /// Declare or get the `opal_weak_upgrade` function.
    fn declare_or_get_weak_upgrade(&self) -> inkwell::values::FunctionValue<'context> {
        if let Some(existing) = self.module.get_function("opal_weak_upgrade") {
            return existing;
        }

        let ctx = self.module.get_context();
        let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
        let fn_type = i8_ptr.fn_type(&[i8_ptr.into()], false);
        self.module
            .add_function("opal_weak_upgrade", fn_type, Some(Linkage::External))
    }

    /// Declare or get the `opal_weak_dec` function.
    fn declare_or_get_weak_dec(&self) -> inkwell::values::FunctionValue<'context> {
        if let Some(existing) = self.module.get_function("opal_weak_dec") {
            return existing;
        }

        let ctx = self.module.get_context();
        let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
        let fn_type = ctx.void_type().fn_type(&[i8_ptr.into()], false);
        self.module
            .add_function("opal_weak_dec", fn_type, Some(Linkage::External))
    }
}
