extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::FunctionValue;

#[must_use]
#[doc = "Build stable symbol names for concrete generic function specializations."]
pub fn monomorphized_function_name(base_name: &str, type_args: &[CoreType]) -> String {
    let mut name = String::from(base_name);
    for type_arg in type_args {
        name.push_str("__");
        name.push_str(&core_type_monomorphization_key(type_arg));
    }
    name
}

#[doc = "Create or retrieve the monomorphized function declaration for a concrete call site."]
pub fn ensure_monomorphized_function_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    generic_function: FunctionValue<'context>,
    base_name: &str,
    type_args: &[CoreType],
) -> FunctionValue<'context> {
    let specialization_name = monomorphized_function_name(base_name, type_args);
    let key = (
        base_name.to_owned(),
        type_args
            .iter()
            .map(core_type_monomorphization_key)
            .collect::<Vec<_>>(),
    );

    if let Some(existing) = env.emitted_specializations.get(&key) {
        return *existing;
    }

    if let Some(existing) = codegen_context.module.get_function(&specialization_name) {
        env.emitted_specializations.insert(key, existing);
        return existing;
    }

    let specialization = codegen_context.module.add_function(
        &specialization_name,
        generic_function.get_type(),
        None,
    );
    env.emitted_specializations.insert(key, specialization);
    specialization
}

#[doc = "Render a core type into a compact suffix token used in specialization names."]
fn core_type_monomorphization_key(core_type: &CoreType) -> String {
    match *core_type {
        CoreType::Int8 => String::from("int8"),
        CoreType::Int16 => String::from("int16"),
        CoreType::Int32 => String::from("int32"),
        CoreType::Int64 => String::from("int64"),
        CoreType::UInt8 => String::from("uint8"),
        CoreType::UInt16 => String::from("uint16"),
        CoreType::UInt32 => String::from("uint32"),
        CoreType::UInt64 => String::from("uint64"),
        CoreType::Float32 => String::from("float32"),
        CoreType::Float64 => String::from("float64"),
        CoreType::String => String::from("string"),
        CoreType::Boolean => String::from("boolean"),
        CoreType::Unit => String::from("unit"),
        CoreType::Array(ref element) => {
            format!("array_{}", core_type_monomorphization_key(element.as_ref()))
        }
        CoreType::Variable(ref variable) => format!("var_{}", variable.name),
        CoreType::Function { .. } => String::from("function"),
        CoreType::Generic {
            ref name,
            ref type_args,
        } => {
            if type_args.is_empty() {
                return name.clone();
            }
            let mut rendered = name.clone();
            for type_arg in type_args {
                rendered.push('_');
                rendered.push_str(&core_type_monomorphization_key(type_arg));
            }
            rendered
        }
    }
}
