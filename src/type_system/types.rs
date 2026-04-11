//! Core type representations for the Opalescent type system
//!
//! This module defines the fundamental types used throughout the language,
//! including primitives, composites, and type variables for inference.

extern crate alloc;

use alloc::{boxed::Box, fmt, string::String, vec::Vec};

/// Represents type variables used in type inference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    /// Unique identifier for this type variable
    pub id: usize,
    /// Human-readable name for debugging
    pub name: String,
}

/// Generic type parameter metadata stored on function types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericTypeParameter {
    /// Declared generic parameter name (for diagnostics/display), e.g. `T`.
    pub name: String,
    /// Internal type variable associated with this generic parameter.
    pub type_var: TypeVar,
    /// Constraint types that the parameter must satisfy.
    pub constraints: Vec<CoreType>,
}

impl TypeVar {
    /// Create a new type variable with the given id and name for debugging context.
    pub const fn new(id: usize, name: String) -> Self {
        Self { id, name }
    }
}

/// Represents the core types supported by the Opalescent language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreType {
    /// 8-bit signed integer
    Int8,
    /// 16-bit signed integer
    Int16,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer
    Int64,
    /// 8-bit unsigned integer
    UInt8,
    /// 16-bit unsigned integer
    UInt16,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit unsigned integer
    UInt64,
    /// 32-bit floating point
    Float32,
    /// 64-bit floating point
    Float64,
    /// Unicode string
    String,
    /// Boolean type
    Boolean,
    /// Unit type (empty value)
    Unit,
    /// Type variable for inference
    Variable(TypeVar),
    /// Array type with element type
    Array(Box<CoreType>),
    /// Function type with parameter types and return type
    Function {
        /// Generic parameters declared by the function signature.
        generic_params: Vec<GenericTypeParameter>,
        /// Parameter types
        parameters: Vec<CoreType>,
        /// Return types in declaration order.
        ///
        /// Backward compatibility: single-return functions use a vector with one element.
        return_types: Vec<CoreType>,
        /// Error types this function may produce
        ///
        /// Architectural note: we use `Vec<CoreType>` (not a set) to preserve
        /// deterministic iteration order for diagnostics and ABI hashing. The
        /// type system treats this as a set semantically in future phases.
        error_types: Vec<CoreType>,
    },
    /// Generic type with name and type arguments
    Generic {
        /// Name of the generic type
        name: String,
        /// Type arguments
        type_args: Vec<CoreType>,
    },
}

impl fmt::Display for CoreType {
    /// Format `CoreType` for user-friendly error messages
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Int8 => write!(f, "int8"),
            Self::Int16 => write!(f, "int16"),
            Self::Int32 => write!(f, "int32"),
            Self::Int64 => write!(f, "int64"),
            Self::UInt8 => write!(f, "uint8"),
            Self::UInt16 => write!(f, "uint16"),
            Self::UInt32 => write!(f, "uint32"),
            Self::UInt64 => write!(f, "uint64"),
            Self::Float32 => write!(f, "float32"),
            Self::Float64 => write!(f, "float64"),
            Self::String => write!(f, "string"),
            Self::Boolean => write!(f, "boolean"),
            Self::Unit => write!(f, "unit"),
            Self::Variable(ref var) => write!(f, "{}", var.name.as_str()),
            Self::Array(ref element_type) => write!(f, "[{element_type}]"),
            Self::Function {
                ref generic_params,
                ref parameters,
                ref return_types,
                ref error_types,
            } => {
                if !generic_params.is_empty() {
                    write!(f, "<")?;
                    for (index, generic_param) in generic_params.iter().enumerate() {
                        if index > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", generic_param.name)?;
                        if !generic_param.constraints.is_empty() {
                            write!(f, ": ")?;
                            for (constraint_index, constraint) in
                                generic_param.constraints.iter().enumerate()
                            {
                                if constraint_index > 0 {
                                    write!(f, " + ")?;
                                }
                                write!(f, "{constraint}")?;
                            }
                        }
                    }
                    write!(f, "> ")?;
                }
                write!(f, "(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> ")?;
                for (i, return_type) in return_types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{return_type}")?;
                }
                if !error_types.is_empty() {
                    write!(f, " errors ")?;
                    for (i, e) in error_types.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{e}")?;
                    }
                }
                Ok(())
            }
            Self::Generic {
                ref name,
                ref type_args,
            } => {
                write!(f, "{name}")?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
        }
    }
}

// NOTE: Keep core types focused and lightweight; accessor helpers that borrow inner
// fields should be added with care to avoid triggering clippy::ref_patterns under
// our strict linting profile. Prefer direct pattern matching at call sites.

/// Classification of numeric types used for cast and operation validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericKind {
    /// Signed integer family (int8, int16, int32, int64).
    SignedInt,
    /// Unsigned integer family (uint8, uint16, uint32, uint64).
    UnsignedInt,
    /// Floating point family (float32, float64).
    Float,
}
