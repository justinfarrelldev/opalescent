//! Compiled native testing: resolution, artifacts and fresh-process execution.
//!
//! This is separate from the legacy Rust function-pointer testing API. Test
//! declarations are parsed by the normal lexer/parser in explicit module mode.
pub mod artifact;
pub mod artifact_cache;
#[cfg(test)]
mod artifact_cache_tests;
#[cfg(test)]
mod artifact_tests;
pub mod cases;
pub mod checking;
pub mod command;
#[cfg(test)]
mod command_tests;
/// Shared explicit declaration-import binding resolution.
mod declaration_imports;
pub mod doubles;
pub mod lowering;
#[cfg(test)]
mod lowering_tests;
pub mod scheduler;
#[cfg(test)]
mod scheduler_tests;
#[cfg(test)]
mod support_module_tests;
#[cfg(test)]
mod terminal_runtime_tests;

#[cfg(test)]
mod double_resolution_tests;

#[cfg(test)]
mod checking_tests;
#[cfg(test)]
mod double_runtime_tests;
pub mod identity;
#[cfg(test)]
mod lifecycle_runtime_tests;
#[cfg(test)]
mod nominal_type_tests;
#[cfg(test)]
mod path_value_runtime_tests;
#[cfg(test)]
mod runtime_tests;
#[cfg(test)]
mod snapshot_runtime_tests;

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
