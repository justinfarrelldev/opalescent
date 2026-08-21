use super::TypeChecker;

impl TypeChecker {
    /// Permit imports from module interfaces marked test-only.
    pub const fn enable_test_only_imports(&mut self) {
        self.allow_test_only_imports = true;
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
