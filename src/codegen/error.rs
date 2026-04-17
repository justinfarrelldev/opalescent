extern crate alloc;

use alloc::string::String;
use inkwell::builder::BuilderError;
use miette::SourceSpan;

/// A code generation error with an optional source span.
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
    pub span: Option<SourceSpan>,
}

impl CodegenError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    #[must_use]
    pub fn with_span(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }
}

impl core::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl core::error::Error for CodegenError {}

impl From<BuilderError> for CodegenError {
    fn from(value: BuilderError) -> Self {
        Self::new(alloc::format!("LLVM builder error: {value}"))
    }
}
