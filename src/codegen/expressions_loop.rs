#![allow(
    clippy::all,
    clippy::missing_const_for_fn,
    reason = "internal codegen implementation module"
)]
use crate::codegen::expressions::{CodegenEnv, LoopContext};
use inkwell::values::PointerValue;

impl<'context> CodegenEnv<'context> {
    /// Push a loop frame for nested break/continue lowering.
    pub fn push_loop(&mut self, loop_context: LoopContext<'context>) {
        self.loop_stack.push(loop_context);
    }

    /// Pop the innermost active loop frame.
    pub fn pop_loop(&mut self) -> Option<LoopContext<'context>> {
        self.loop_stack.pop()
    }

    /// Borrow the innermost loop frame if one exists.
    #[must_use]
    pub fn current_loop(&self) -> Option<&LoopContext<'context>> {
        self.loop_stack.last()
    }

    /// Run closure with loop stack cleared, then restore snapshot.
    pub fn with_loop_isolated<T>(&mut self, callback: impl FnOnce(&mut Self) -> T) -> T {
        let saved_stack = core::mem::take(&mut self.loop_stack);
        let result = callback(self);
        self.loop_stack = saved_stack;
        result
    }

    /// Push the active guard error slot for a guard else-scope.
    pub fn push_active_guard_error_slot(&mut self, error_slot: PointerValue<'context>) {
        self.active_guard_error_slots.push(error_slot);
    }

    /// Pop the innermost active guard error slot.
    pub fn pop_active_guard_error_slot(&mut self) -> Option<PointerValue<'context>> {
        self.active_guard_error_slots.pop()
    }

    /// Borrow the innermost active guard error slot if one exists.
    #[must_use]
    pub fn current_guard_error_slot(&self) -> Option<PointerValue<'context>> {
        self.active_guard_error_slots.last().copied()
    }
}
