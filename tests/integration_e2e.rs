#![cfg(feature = "integration")]

use opalescent::build_system::targets::TargetTriple;
use opalescent::compiler::{
    CompileError, compile_program, compile_to_module, emit_object_file, link_object_file,
};
use opalescent::errors::reporter::CompilerError;
use opalescent::type_system::errors::TypeError;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn prepare_dir(path: &Path) -> Result<PathBuf, std::io::Error> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::create_dir_all(path)?;
    Ok(path.to_path_buf())
}

fn cleanup_dir(path: &Path) -> Result<(), std::io::Error> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "integration_e2e/tests.rs"]
mod tests;
