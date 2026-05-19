//! Memory layout information for types (required for hot reload ABI checking)

use super::types::CoreType;

/// Memory layout information for a type
///
/// Required for Phase 6 hot reload ABI compatibility checking.
/// This ensures that types maintain binary compatibility across reloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLayout {
    /// Size of the type in bytes
    pub size: usize,
    /// Alignment requirement in bytes
    pub align: usize,
}

impl CoreType {
    /// Get the memory layout for this type
    ///
    /// This is critical for Phase 6 hot reload ABI compatibility checking.
    /// Types with different memory layouts cannot be hot-swapped safely.
    ///
    /// # Returns
    ///
    /// The memory layout (size and alignment) for this type.
    pub const fn memory_layout(&self) -> MemoryLayout {
        match *self {
            Self::Int8 | Self::UInt8 | Self::Boolean => MemoryLayout { size: 1, align: 1 },
            Self::Int16 | Self::UInt16 => MemoryLayout { size: 2, align: 2 },
            Self::Int32 | Self::UInt32 | Self::Float32 => MemoryLayout { size: 4, align: 4 },
            Self::Int64 | Self::UInt64 | Self::Float64 => MemoryLayout { size: 8, align: 8 },
            Self::Unit | Self::Variable(_) => MemoryLayout { size: 0, align: 1 },
            // Pointers (strings, arrays, functions) are pointer-sized
            Self::String | Self::Array(_) | Self::Function { .. } | Self::Generic { .. } => {
                MemoryLayout {
                    size: core::mem::size_of::<usize>(),
                    align: core::mem::align_of::<usize>(),
                }
            }
        }
    }

    /// Returns the inner element type if this core type is an array.
    pub fn array_element_type(&self) -> Option<&Self> {
        match *self {
            Self::Array(ref element_type) => Some(element_type.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_type_sizes() {
        assert_eq!(CoreType::Int8.memory_layout().size, 1);
        assert_eq!(CoreType::Int16.memory_layout().size, 2);
        assert_eq!(CoreType::Int32.memory_layout().size, 4);
        assert_eq!(CoreType::Int64.memory_layout().size, 8);
    }

    #[test]
    fn test_unsigned_integer_type_sizes() {
        assert_eq!(CoreType::UInt8.memory_layout().size, 1);
        assert_eq!(CoreType::UInt16.memory_layout().size, 2);
        assert_eq!(CoreType::UInt32.memory_layout().size, 4);
        assert_eq!(CoreType::UInt64.memory_layout().size, 8);
    }

    #[test]
    fn test_float_type_sizes() {
        assert_eq!(CoreType::Float32.memory_layout().size, 4);
        assert_eq!(CoreType::Float64.memory_layout().size, 8);
    }

    #[test]
    fn test_boolean_size() {
        assert_eq!(CoreType::Boolean.memory_layout().size, 1);
    }

    #[test]
    fn test_integer_type_alignments() {
        assert_eq!(CoreType::Int8.memory_layout().align, 1);
        assert_eq!(CoreType::Int16.memory_layout().align, 2);
        assert_eq!(CoreType::Int32.memory_layout().align, 4);
        assert_eq!(CoreType::Int64.memory_layout().align, 8);
    }
}
