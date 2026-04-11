use crate::runtime::errors::{RuntimeError, RuntimeResult};
use crate::runtime::memory::{OpalArray, RuntimeAllocator};

/// Allocate a runtime array through the configured allocator.
///
/// # Errors
///
/// Returns allocator-provided runtime errors when allocation fails.
pub fn allocate_array<Allocator, Element>(
    allocator: &Allocator,
    values: &[Element],
) -> RuntimeResult<OpalArray<Element>>
where
    Allocator: RuntimeAllocator,
    Element: Clone,
{
    allocator.allocate_array(values)
}

/// Return number of elements in runtime array.
#[must_use]
pub fn array_length<Element>(array: &OpalArray<Element>) -> usize {
    array.len()
}

/// Read an array element with runtime bounds checking.
///
/// # Errors
///
/// Returns [`RuntimeError::IndexOutOfBounds`] when `index` is invalid.
pub fn array_index<Element>(array: &OpalArray<Element>, index: usize) -> RuntimeResult<Element>
where
    Element: Clone,
{
    array.get(index).map_or_else(
        || {
            Err(RuntimeError::IndexOutOfBounds {
                index,
                length: array.len(),
            })
        },
        |value| Ok(value.clone()),
    )
}
