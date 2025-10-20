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
    ///
    /// # Note
    ///
    /// Currently returns placeholder values. These should be updated to match
    /// the actual LLVM backend layout in Phase 5.
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
}
