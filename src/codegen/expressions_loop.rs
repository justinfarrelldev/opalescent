#![allow(
    clippy::all,
    clippy::missing_const_for_fn,
    reason = "internal codegen implementation module"
)]
use crate::codegen::expressions::{CodegenEnv, LoopContext};
use inkwell::values::IntValue;

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

    /// Record a runtime length extracted from the current array-producing expression.
    pub fn set_pending_array_length(&mut self, length: Option<IntValue<'context>>) {
        self.pending_array_length = length;
    }

    /// Consume any runtime array length left by the immediately preceding expression lowering.
    pub fn take_pending_array_length(&mut self) -> Option<IntValue<'context>> {
        self.pending_array_length.take()
    }
}
