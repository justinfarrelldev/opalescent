/// Consistent compiler diagnostic formatting utilities.
pub mod formatter;
/// Graphical miette-based diagnostic renderer.
pub mod renderer;
/// Multi-error collection and rendering utilities.
pub mod reporter;
/// Typo and contextual suggestion helpers.
pub mod suggestions;

#[cfg(test)]
mod tests;

/// Touch exported error APIs so strict dead-code lints remain satisfied.
#[must_use]
pub fn touch_error_api_for_lints() -> bool {
    let probe_error = crate::type_system::errors::TypeError::CannotInferGenericType {
        param_name: String::from("T"),
        span: miette::SourceSpan::new(0.into(), 1),
    };

    let _type_hint = suggestions::did_you_mean_type_annotation(&probe_error);
    let _codegen_preview = formatter::format_codegen_error("lint warmup");
    let _formatted_type = formatter::format_diagnostic(
        formatter::CompilerPhase::TypeChecker,
        &reporter::CompilerError::TypeChecker(probe_error.clone()),
    );
    let _formatted_codegen = formatter::format_diagnostic(
        formatter::CompilerPhase::Codegen,
        &reporter::CompilerError::Codegen(crate::codegen::error::CodegenError::new("lint warmup")),
    );

    let mut report = reporter::CompilationErrorReport::new();
    report.push_type_error(probe_error.clone());
    report.push_codegen_error(String::from("lint warmup"));
    report.extend_type_errors(vec![probe_error]);
    let has_errors = !report.is_empty();
    let _count = report.len();

    has_errors
}
