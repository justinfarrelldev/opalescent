//! Compiler-emitted constant native lifecycle descriptors, matching the C ABI.
extern crate alloc;
use super::context::CodegenContext;
use super::error::CodegenError;
use crate::testing::native::identity::WorldIdentity;
use crate::token::Span;
use alloc::collections::BTreeSet;
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::types::{FunctionType, IntType, StructType};
use inkwell::values::{FunctionValue, GlobalValue, PointerValue, StructValue};

/// A compiled callback and its unchanged defining source extent.
#[derive(Clone)]
pub struct NativeCallbackDescriptor<'context> {
    /// Scalar C lifecycle wrapper, never a raw source-outcome function.
    pub function: FunctionValue<'context>,
    /// Defining source path for lifecycle application-error diagnostics.
    pub file: String,
    /// Original source extent, without source rewriting.
    pub span: Span,
}

/// One canonical World in deterministic initialization order.
pub struct NativeWorldDescriptor<'context> {
    /// Canonical provider identity, independent of local import spelling.
    pub identity: WorldIdentity,
    /// Required typed setup boundary.
    pub setup: NativeCallbackDescriptor<'context>,
    /// Explicit optional cleanup; absence remains legal.
    pub cleanup: Option<NativeCallbackDescriptor<'context>>,
    /// Ordinary typed value drop, not external resource cleanup.
    pub drop_value: Option<FunctionValue<'context>>,
}

/// Source-order hooks in an outermost-to-innermost scope sequence.
pub struct NativeHookScopeDescriptor<'context> {
    /// Source-order before-hooks.
    pub before: Vec<NativeCallbackDescriptor<'context>>,
    /// Source-order after-hooks; the lifecycle reverses invocation order.
    pub after: Vec<NativeCallbackDescriptor<'context>>,
}

/// An independently schedulable case referencing reusable compiled callbacks.
pub struct NativeCaseDescriptorPlan<'context> {
    /// Unique canonical Worlds in deterministic topological order.
    pub worlds: Vec<NativeWorldDescriptor<'context>>,
    /// Hook scopes, including those with no before-hooks.
    pub scopes: Vec<NativeHookScopeDescriptor<'context>>,
    /// Shared compiled body boundary.
    pub body: NativeCallbackDescriptor<'context>,
    /// Per-attempt typed example constructor, when parameterized.
    pub construct_example: Option<NativeCallbackDescriptor<'context>>,
    /// Ordinary example value memory drop.
    pub drop_example: Option<FunctionValue<'context>>,
}

/// Emit a C-compatible immutable descriptor without running any source callback.
///
/// # Errors
/// Rejects duplicate descriptor names, incompatible wrapper ABIs and unsized targets.
pub fn emit_case_descriptor<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    plan: &NativeCaseDescriptorPlan<'context>,
) -> Result<GlobalValue<'context>, CodegenError> {
    if codegen.module.get_global(name).is_some() || codegen.module.get_function(name).is_some() {
        return Err(CodegenError::new(
            "native case descriptor name is already defined",
        ));
    }
    let types = DescriptorTypes::new(codegen)?;
    let mut worlds = Vec::new();
    let mut identities = BTreeSet::new();
    for world in &plan.worlds {
        if !identities.insert(&world.identity) {
            return Err(CodegenError::new(
                "native case descriptor contains a duplicate canonical World provider",
            ));
        }
        let canonical = serde_json::to_string(&world.identity)
            .map_err(|error| CodegenError::new(error.to_string()))?;
        let identity = text(codegen, &canonical)?;
        worlds.push(
            types.world.const_named_struct(&[
                identity.into(),
                types.callback_value(codegen, Some(&world.setup))?.into(),
                types
                    .callback_value(codegen, world.cleanup.as_ref())?
                    .into(),
                types.drop_pointer(codegen, world.drop_value)?.into(),
            ]),
        );
    }
    let worlds = array_pointer(codegen, &format!("{name}.worlds"), types.world, &worlds);
    let mut scopes = Vec::new();
    for (index, scope) in plan.scopes.iter().enumerate() {
        let before = types.callback_array(
            codegen,
            &format!("{name}.scope.{index}.before"),
            &scope.before,
        )?;
        let after = types.callback_array(
            codegen,
            &format!("{name}.scope.{index}.after"),
            &scope.after,
        )?;
        scopes.push(types.scope.const_named_struct(&[
            before.into(),
            types.count(scope.before.len())?.into(),
            after.into(),
            types.count(scope.after.len())?.into(),
        ]));
    }
    let scopes = array_pointer(codegen, &format!("{name}.scopes"), types.scope, &scopes);
    let initializer = types.case.const_named_struct(&[
        worlds.into(),
        types.count(plan.worlds.len())?.into(),
        scopes.into(),
        types.count(plan.scopes.len())?.into(),
        types.callback_value(codegen, Some(&plan.body))?.into(),
        types
            .callback_value(codegen, plan.construct_example.as_ref())?
            .into(),
        types.drop_pointer(codegen, plan.drop_example)?.into(),
    ]);
    let global = codegen.module.add_global(types.case, None, name);
    global.set_initializer(&initializer);
    global.set_constant(true);
    Ok(global)
}

/// Physical layouts shared with `opal_testing_lifecycle.h`, including target `size_t`.
struct DescriptorTypes<'context> {
    /// A lifecycle callback pointer with scalar status and explicit outputs.
    invoke: FunctionType<'context>,
    /// Ordinary value-drop function pointer signature.
    drop: FunctionType<'context>,
    /// C `size_t`, used for array lengths.
    size: IntType<'context>,
    /// Callback function and original source extent.
    callback: StructType<'context>,
    /// Provider setup, declared cleanup and normal drop.
    world: StructType<'context>,
    /// Before/after callback arrays and counts.
    scope: StructType<'context>,
    /// The independently schedulable case descriptor.
    case: StructType<'context>,
}

impl<'context> DescriptorTypes<'context> {
    /// Derive types from the native target, never from host Rust struct layout.
    fn new(codegen: &CodegenContext<'context>) -> Result<Self, CodegenError> {
        let llvm = codegen.context;
        let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
        let slot = pointer.ptr_type(AddressSpace::default());
        let identity = llvm.i64_type();
        let machine = codegen
            .target_machine
            .as_ref()
            .ok_or_else(|| CodegenError::new("native descriptor emission requires target data"))?;
        let size = llvm.ptr_sized_int_type(&machine.get_target_data(), None);
        let invoke = llvm.i32_type().fn_type(
            &[
                slot.into(),
                pointer.into(),
                slot.into(),
                identity.ptr_type(AddressSpace::default()).into(),
                slot.into(),
            ],
            false,
        );
        let drop = llvm.void_type().fn_type(&[pointer.into()], false);
        let callback = llvm.struct_type(
            &[
                invoke.ptr_type(AddressSpace::default()).into(),
                pointer.into(),
                identity.into(),
                identity.into(),
            ],
            false,
        );
        let world = llvm.struct_type(
            &[
                pointer.into(),
                callback.into(),
                callback.into(),
                drop.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        let callbacks = callback.ptr_type(AddressSpace::default());
        let scope = llvm.struct_type(
            &[callbacks.into(), size.into(), callbacks.into(), size.into()],
            false,
        );
        let case = llvm.struct_type(
            &[
                world.ptr_type(AddressSpace::default()).into(),
                size.into(),
                scope.ptr_type(AddressSpace::default()).into(),
                size.into(),
                callback.into(),
                callback.into(),
                drop.ptr_type(AddressSpace::default()).into(),
            ],
            false,
        );
        Ok(Self {
            invoke,
            drop,
            size,
            callback,
            world,
            scope,
            case,
        })
    }

    /// Validate an explicitly C-compatible wrapper before embedding its address.
    fn callback_value(
        &self,
        codegen: &CodegenContext<'context>,
        callback: Option<&NativeCallbackDescriptor<'context>>,
    ) -> Result<StructValue<'context>, CodegenError> {
        let Some(callback) = callback else {
            return Ok(self.callback.const_zero());
        };
        if callback.function.get_type() != self.invoke
            || callback.function.get_call_conventions() != 0
            || codegen.native_function_outcome(callback.function).is_some()
        {
            return Err(CodegenError::new(
                "case descriptor callback must use the scalar lifecycle ABI",
            ));
        }
        let offset = u64::try_from(callback.span.start.offset)
            .map_err(|error| CodegenError::new(error.to_string()))?;
        let length = u64::try_from(callback.span.len())
            .map_err(|error| CodegenError::new(error.to_string()))?;
        Ok(self.callback.const_named_struct(&[
            callback
                .function
                .as_global_value()
                .as_pointer_value()
                .into(),
            text(codegen, &callback.file)?.into(),
            codegen.context.i64_type().const_int(offset, false).into(),
            codegen.context.i64_type().const_int(length, false).into(),
        ]))
    }

    /// Missing declared cleanup/drop stays null, with no synthesized obligation.
    fn drop_pointer(
        &self,
        codegen: &CodegenContext<'context>,
        function: Option<FunctionValue<'context>>,
    ) -> Result<PointerValue<'context>, CodegenError> {
        if let Some(function) = function {
            if function.get_type() != self.drop
                || function.get_call_conventions() != 0
                || codegen.native_function_outcome(function).is_some()
            {
                return Err(CodegenError::new(
                    "value drop descriptor requires the ordinary void pointer ABI",
                ));
            }
            Ok(function.as_global_value().as_pointer_value())
        } else {
            Ok(self.drop.ptr_type(AddressSpace::default()).const_null())
        }
    }

    /// Source order is preserved; the lifecycle owns reverse after-hook execution.
    fn callback_array(
        &self,
        codegen: &CodegenContext<'context>,
        name: &str,
        callbacks: &[NativeCallbackDescriptor<'context>],
    ) -> Result<PointerValue<'context>, CodegenError> {
        let values = callbacks
            .iter()
            .map(|callback| self.callback_value(codegen, Some(callback)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(array_pointer(codegen, name, self.callback, &values))
    }

    /// Reject overflow rather than truncate descriptor lengths on narrow targets.
    fn count(&self, count: usize) -> Result<inkwell::values::IntValue<'context>, CodegenError> {
        let value = u64::try_from(count).map_err(|error| CodegenError::new(error.to_string()))?;
        if self.size.get_bit_width() < 64 && value >> self.size.get_bit_width() != 0 {
            return Err(CodegenError::new(
                "native descriptor length exceeds target size_t",
            ));
        }
        Ok(self.size.const_int(value, false))
    }
}

/// Empty lists use null, not zero-length array addresses or dynamic maps.
fn array_pointer<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    element: StructType<'context>,
    values: &[StructValue<'context>],
) -> PointerValue<'context> {
    let pointer = element.ptr_type(AddressSpace::default());
    if values.is_empty() {
        return pointer.const_null();
    }
    let initializer = element.const_array(values);
    let global = codegen
        .module
        .add_global(initializer.get_type(), None, name);
    global.set_linkage(Linkage::Private);
    global.set_constant(true);
    global.set_initializer(&initializer);
    global.as_pointer_value().const_cast(pointer)
}

/// Metadata is emitted as static UTF-8 strings, never executed for discovery.
fn text<'context>(
    codegen: &CodegenContext<'context>,
    value: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    Ok(codegen
        .builder
        .build_global_string_ptr(value, "test.descriptor.text")?
        .as_pointer_value())
}
