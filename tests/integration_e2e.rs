#![cfg(feature = "integration")]

use opalescent::compiler::{
    compile_program, compile_to_module, emit_object_file, link_object_file,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn prepare_dir(path: &Path) -> Result<PathBuf, std::io::Error> {
    if path.exists() {
        fs::remove_dir_all(path)?;
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
mod tests {
    use super::*;

    #[test]
    fn smoke_void_program_compiles_links_and_runs() {
        let temp_dir = Path::new("test-projects/_smoke/target");
        let prepare = prepare_dir(temp_dir);
        assert!(prepare.is_ok(), "smoke temp directory should be created");

        let source = "entry main = f(): void => { return void }";
        let binary_result = compile_program(source, temp_dir);
        assert!(
            binary_result.is_ok(),
            "smoke source should compile to a runnable binary"
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = Command::new(&binary_path).output();
        assert!(
            output_result.is_ok(),
            "compiled smoke binary should execute"
        );
        let Ok(run_output) = output_result else {
            return;
        };

        assert!(
            run_output.status.success(),
            "compiled smoke binary should exit successfully"
        );
        assert!(
            run_output.stdout.is_empty(),
            "compiled smoke binary should not print anything"
        );

        let cleanup = cleanup_dir(temp_dir);
        assert!(cleanup.is_ok(), "smoke temp directory should be removed");
    }

    #[test]
    fn emit_object_file_creates_valid_object() {
        let temp_dir = Path::new("test-projects/_emit/target");
        let prepare = prepare_dir(temp_dir);
        assert!(prepare.is_ok(), "emit temp directory should be created");

        let context = inkwell::context::Context::create();
        let source = "entry main = f(): void => { return void }";
        let module_result = compile_to_module(&context, source);
        assert!(
            module_result.is_ok(),
            "source should compile into an LLVM module for object emission"
        );
        let Ok(module) = module_result else {
            return;
        };

        let object_path = temp_dir.join("program.o");
        let emit_result = emit_object_file(&module, &object_path);
        assert!(emit_result.is_ok(), "object emission should succeed");

        assert!(
            object_path.exists(),
            "object file should exist after emission"
        );

        let metadata_result = fs::metadata(&object_path);
        assert!(
            metadata_result.is_ok(),
            "object metadata should be readable"
        );
        let Ok(metadata) = metadata_result else {
            return;
        };
        assert!(
            metadata.len() > 0,
            "object file should be non-empty after emission"
        );

        let cleanup = cleanup_dir(temp_dir);
        assert!(cleanup.is_ok(), "emit temp directory should be removed");
    }

    #[test]
    fn link_produces_executable() {
        let temp_dir = Path::new("test-projects/_link/target");
        let prepare = prepare_dir(temp_dir);
        assert!(prepare.is_ok(), "link temp directory should be created");

        let context = inkwell::context::Context::create();
        let source = "entry main = f(): void => { return void }";
        let module_result = compile_to_module(&context, source);
        assert!(
            module_result.is_ok(),
            "source should compile into an LLVM module for linking"
        );
        let Ok(module) = module_result else {
            return;
        };

        let object_path = temp_dir.join("program.o");
        let binary_path = temp_dir.join("program");

        let emit_result = emit_object_file(&module, &object_path);
        assert!(
            emit_result.is_ok(),
            "object emission should succeed before linking"
        );

        let link_result = link_object_file(&object_path, &binary_path, &[]);
        assert!(
            link_result.is_ok(),
            "link step should produce an executable"
        );
        let Ok(linked_binary) = link_result else {
            return;
        };

        assert!(
            linked_binary.exists(),
            "linked binary should exist at requested output path"
        );

        #[cfg(unix)]
        {
            let metadata_result = fs::metadata(&linked_binary);
            assert!(
                metadata_result.is_ok(),
                "linked binary metadata should be readable on unix"
            );
            let Ok(metadata) = metadata_result else {
                return;
            };
            let mode = metadata.permissions().mode();
            assert!(
                mode & 0o111 != 0,
                "linked output should have executable bits on unix"
            );
        }

        let cleanup = cleanup_dir(temp_dir);
        assert!(cleanup.is_ok(), "link temp directory should be removed");
    }
}
