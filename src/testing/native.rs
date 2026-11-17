//! Compiled native testing: resolution, artifacts and fresh-process execution.
//!
//! This is separate from the legacy Rust function-pointer testing API. Test
//! declarations are parsed by the normal lexer/parser in explicit module mode.
pub mod artifact;
#[cfg(test)]
mod artifact_tests;
pub mod cases;
pub mod checking;
/// Shared explicit declaration-import binding resolution.
mod declaration_imports;
pub mod doubles;
pub mod lowering;
#[cfg(test)]
mod lowering_tests;

#[cfg(test)]
mod double_resolution_tests;

#[cfg(test)]
mod checking_tests;
pub mod identity;
#[cfg(test)]
mod lifecycle_runtime_tests;
#[cfg(test)]
mod nominal_type_tests;
#[cfg(test)]
mod runtime_tests;

#[cfg(test)]
mod case_tests;
pub mod project;
pub mod subjects;
pub mod world_graph;
pub mod world_resolution;

#[cfg(test)]
mod subject_tests;

#[cfg(test)]
mod world_resolution_tests;

#[cfg(test)]
mod project_tests;

#[cfg(test)]
mod world_graph_tests;
