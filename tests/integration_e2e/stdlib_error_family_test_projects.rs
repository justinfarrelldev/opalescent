#![cfg(feature = "integration")]

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CLI_WARNING_TEST_TIMEOUT: Duration = Duration::from_secs(30);
const REPLACEABLE_ERROR_LIST_CODE: &str =
    "opalescent::type_system::warning::replaceable_error_list";

struct Fixture {
    source: &'static str,
    family: &'static str,
    leaves: &'static str,
}

struct SafermWarning {
    source: &'static str,
    line: usize,
    label: &'static str,
    family: &'static str,
    leaves: &'static str,
}

const REMEDIATED_FIXTURES: &[Fixture] = &[
    Fixture {
        source: "test-projects/_absolute_path_sync/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/_fs_append_log/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/_fs_dir_inventory/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/_fs_read_text_lines/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/_fs_write_text_atomic/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/bytes-hex-roundtrip/src/main.op",
        family: "BytesError",
        leaves: "HexDecodeError, SliceRangeError",
    },
    Fixture {
        source: "test-projects/delete-downloads/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/delete-downloads-strict/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/fs-directory-operations/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/fs-markdown-roundtrip/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/fs-path-manipulation/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/game-of-life/src/main.op",
        family: "StringBuilderError",
        leaves: "BuilderFinishedError, AllocationFailureError",
    },
    Fixture {
        source: "test-projects/game-of-life/src/main.op",
        family: "OutputError",
        leaves: "WriteFailureError, FlushFailureError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/op-cat/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/print-text-flush-without-newline/src/main.op",
        family: "OutputError",
        leaves: "WriteFailureError, FlushFailureError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/process-api-smoke/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/process-api-smoke/src/main.op",
        family: "ProcessEnvError",
        leaves: "EnvironmentVariableNotFoundError, InvalidEnvironmentVariableNameError, InvalidUtf8Error",
    },
    Fixture {
        source: "test-projects/process-cwd/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/process-env/src/main.op",
        family: "ProcessEnvError",
        leaves: "EnvironmentVariableNotFoundError, InvalidEnvironmentVariableNameError, InvalidUtf8Error",
    },
    Fixture {
        source: "test-projects/process-paths/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/saferm/src/main_backup.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    Fixture {
        source: "test-projects/stdout-writer-interleaves-with-print-text/src/main.op",
        family: "OutputError",
        leaves: "WriteFailureError, FlushFailureError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/stdout-writer-write-flush/src/main.op",
        family: "OutputError",
        leaves: "WriteFailureError, FlushFailureError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/string-builder-push-finish/src/main.op",
        family: "StringBuilderError",
        leaves: "BuilderFinishedError, AllocationFailureError",
    },
    Fixture {
        source: "test-projects/string-builder-use-after-finish-errors/src/main.op",
        family: "StringBuilderError",
        leaves: "BuilderFinishedError, AllocationFailureError",
    },
    Fixture {
        source: "test-projects/string-ranges-stdlib/src/main.op",
        family: "StringRangeError",
        leaves: "StringNegativeCountError, StringRangeOutOfBoundsError, StringRangeOrderError",
    },
    Fixture {
        source: "test-projects/string-search-stdlib/src/main.op",
        family: "StringSearchError",
        leaves: "StringEmptySearchTextError, StringPatternNotFoundError",
    },
    Fixture {
        source: "test-projects/terminal-move-cursor-rejects-negative-column/src/main.op",
        family: "TerminalError",
        leaves: "TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/terminal-move-cursor-rejects-negative-row/src/main.op",
        family: "TerminalError",
        leaves: "TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/terminal-move-cursor-zero-based-ansi-bytes/src/main.op",
        family: "TerminalError",
        leaves: "TerminalWriteFailureError, InvalidCursorPositionError, SinkClosedError",
    },
    Fixture {
        source: "test-projects/windows-file-ops/src/main.op",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
];

const SAFERM_PRE_REMEDIATION_WARNINGS: &[SafermWarning] = &[
    SafermWarning {
        source: "src/main.op",
        line: 23,
        label: "entry main = f(args: string[]): void errors",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    SafermWarning {
        source: "src/trash.op",
        line: 28,
        label: "public let unique_destination_for = f(destination_root: FilesystemPath, name: string): FilesystemPath errors",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    SafermWarning {
        source: "src/trash.op",
        line: 43,
        label: "public let trash_entry_exists = f(dest: FilesystemPath, requested_name: string): boolean errors",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    SafermWarning {
        source: "src/trash.op",
        line: 70,
        label: "public let move_to_destination = f(arg: string, source_root: FilesystemPath, destination_root: FilesystemPath, force: boolean, verbose: boolean): void errors",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    SafermWarning {
        source: "src/trash.op",
        line: 95,
        label: "public let create_trash_path_if_not_exists = f(dest: FilesystemPath): void errors",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
    SafermWarning {
        source: "src/trash.op",
        line: 110,
        label: "public let get_trash_entries = f(dest: FilesystemPath): string[] errors",
        family: "FilesystemPathError",
        leaves: "InvalidPathError, PermissionDeniedError",
    },
];

const INTENTIONALLY_EXACT_CLAUSES: &[&str] = &[
    "test-projects/_fs_append_log/src/logger.op:6",
    "test-projects/_fs_write_text_atomic/src/atomic.op:6",
    "test-projects/fs-directory-operations/src/operations/create.op:6",
    "test-projects/fs-directory-operations/src/operations/create.op:13",
    "test-projects/fs-directory-operations/src/operations/list.op:6",
    "test-projects/fs-directory-operations/src/operations/remove.op:6",
    "test-projects/fs-directory-operations/src/operations/remove.op:13",
    "test-projects/fs-markdown-roundtrip/src/processing/serialize.op:6",
    "test-projects/game-of-life-full/src/main.op:11",
    "test-projects/game-of-life-full/src/render.op:16",
];

const ELIGIBLE_FAMILIES: &[(&str, &[&str])] = &[
    ("BytesError", &["HexDecodeError", "SliceRangeError"]),
    (
        "StringSearchError",
        &["StringEmptySearchTextError", "StringPatternNotFoundError"],
    ),
    (
        "StringRangeError",
        &[
            "StringNegativeCountError",
            "StringRangeOutOfBoundsError",
            "StringRangeOrderError",
        ],
    ),
    (
        "StringBuilderError",
        &["BuilderFinishedError", "AllocationFailureError"],
    ),
    (
        "OutputError",
        &["WriteFailureError", "FlushFailureError", "SinkClosedError"],
    ),
    (
        "TerminalError",
        &[
            "TerminalWriteFailureError",
            "InvalidCursorPositionError",
            "SinkClosedError",
        ],
    ),
    (
        "TimeError",
        &["InvalidDurationError", "InvalidFrameRateError"],
    ),
    (
        "ProcessPathError",
        &[
            "PermissionDeniedError",
            "InvalidPathError",
            "CurrentWorkingDirectoryUnavailableError",
            "CurrentExecutablePathUnavailableError",
            "FileNotFoundError",
            "IsNotADirectoryError",
        ],
    ),
    (
        "ProcessEnvError",
        &[
            "EnvironmentVariableNotFoundError",
            "InvalidEnvironmentVariableNameError",
            "InvalidUtf8Error",
        ],
    ),
    (
        "FilesystemPathError",
        &["InvalidPathError", "PermissionDeniedError"],
    ),
    (
        "FilesystemReadError",
        &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "ReadFailureError",
            "IsADirectoryError",
            "InvalidPathError",
            "InvalidUtf8Error",
            "OffsetOutOfRangeError",
        ],
    ),
    (
        "FilesystemWriteError",
        &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "WriteFailureError",
            "IsADirectoryError",
            "InvalidPathError",
            "FilesystemFullError",
            "OffsetOutOfRangeError",
        ],
    ),
    (
        "FilesystemCreateError",
        &[
            "FileAlreadyExistsError",
            "PermissionDeniedError",
            "CreateFailureError",
            "InvalidPathError",
            "FilesystemFullError",
        ],
    ),
    (
        "FilesystemDeleteError",
        &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "DeleteFailureError",
            "IsADirectoryError",
            "InvalidPathError",
        ],
    ),
    (
        "FilesystemDirectoryDeleteError",
        &[
            "DirectoryNotFoundError",
            "PermissionDeniedError",
            "DeleteFailureError",
            "DirectoryNotEmptyError",
            "IsNotADirectoryError",
            "InvalidPathError",
        ],
    ),
    (
        "FilesystemCopyMoveError",
        &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "CopyFailureError",
            "MoveFailureError",
            "IsADirectoryError",
            "FileAlreadyExistsError",
            "InvalidPathError",
            "FilesystemFullError",
        ],
    ),
    (
        "FilesystemMetadataError",
        &[
            "FileNotFoundError",
            "PermissionDeniedError",
            "MetadataUnavailableError",
            "InvalidPathError",
        ],
    ),
    (
        "FilesystemListError",
        &[
            "DirectoryNotFoundError",
            "PermissionDeniedError",
            "ReadFailureError",
            "IsNotADirectoryError",
            "InvalidPathError",
        ],
    ),
];

fn opalescent_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("opalescent")
}

fn assert_check(source: &Path, expected_warning: Option<&Fixture>) -> Result<(), String> {
    let mut command = Command::new(opalescent_binary_path());
    command.args(["check", source.to_string_lossy().as_ref()]);
    let output = run_command_output_with_timeout(
        &mut command,
        CLI_WARNING_TEST_TIMEOUT,
        &format!("opal check {}", source.display()),
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stdout.contains("check passed") {
        return Err(format!(
            "{} should check successfully, stdout: {stdout}, stderr: {stderr}",
            source.display()
        ));
    }

    match expected_warning {
        Some(fixture) => {
            let help = format!(
                "Replace `errors {}` with `errors {}`.",
                fixture.leaves, fixture.family
            );
            for expected in [
                REPLACEABLE_ERROR_LIST_CODE,
                fixture.family,
                fixture.leaves,
                help.as_str(),
            ] {
                if !stderr.contains(expected) {
                    return Err(format!(
                        "{} should render {expected:?}, stderr: {stderr}",
                        source.display()
                    ));
                }
            }
        }
        None if stderr.contains(REPLACEABLE_ERROR_LIST_CODE) => {
            return Err(format!(
                "{} should not render a replacement warning, stderr: {stderr}",
                source.display()
            ));
        }
        None => {}
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "{} should be created before copying the saferm fixture: {error}",
            destination.display()
        )
    })?;

    for entry in fs::read_dir(source)
        .map_err(|error| format!("{} should be readable: {error}", source.display()))?
    {
        let entry =
            entry.map_err(|error| format!("saferm fixture entry should be readable: {error}"))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| format!("{} type should be readable: {error}", source_path.display()))?
            .is_dir()
        {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "{} should copy to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn restore_saferm_pre_remediation_sources(project_dir: &Path) -> Result<(), String> {
    for source in ["src/main.op", "src/trash.op"] {
        let source_path = project_dir.join(source);
        let source_text = fs::read_to_string(&source_path)
            .map_err(|error| format!("{} should be readable: {error}", source_path.display()))?;
        let restored = source_text.replace(
            "FilesystemPathError",
            "PermissionDeniedError, InvalidPathError",
        );
        fs::write(&source_path, restored)
            .map_err(|error| format!("{} should be writable: {error}", source_path.display()))?;
    }
    Ok(())
}

fn assert_saferm_project_build(
    project_dir: &Path,
    expected_warnings: &[SafermWarning],
) -> Result<(), String> {
    let mut command = Command::new(opalescent_binary_path());
    command.arg("build").current_dir(project_dir);
    let output = run_command_output_with_timeout(
        &mut command,
        CLI_WARNING_TEST_TIMEOUT,
        &format!("saferm opal build in {}", project_dir.display()),
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "saferm build should succeed, stdout: {stdout}, stderr: {stderr}"
        ));
    }

    if expected_warnings.is_empty() {
        if stderr.contains(REPLACEABLE_ERROR_LIST_CODE) {
            return Err(format!(
                "saferm project build should not render a replacement warning, stderr: {stderr}"
            ));
        }
        return Ok(());
    }

    let warning_count = stderr.matches(REPLACEABLE_ERROR_LIST_CODE).count();
    if warning_count != expected_warnings.len() {
        return Err(format!(
            "saferm project build should render {} replacement warnings, found {warning_count}, stderr: {stderr}",
            expected_warnings.len()
        ));
    }

    let mut warning_blocks = stderr
        .split(REPLACEABLE_ERROR_LIST_CODE)
        .skip(1)
        .collect::<Vec<_>>();
    for fixture in expected_warnings {
        let source_location = format!("{}:{}", fixture.source, fixture.line);
        let help = format!(
            "Replace `errors {}` with `errors {}`.",
            fixture.leaves, fixture.family
        );
        let Some(block_index) = warning_blocks.iter().position(|block| {
            [
                source_location.as_str(),
                fixture.label,
                fixture.family,
                fixture.leaves,
                help.as_str(),
            ]
            .into_iter()
            .all(|expected| block.contains(expected))
        }) else {
            return Err(format!(
                "{} ({}) should have one matching replacement warning, remaining warnings: {warning_blocks:#?}",
                source_location, fixture.label
            ));
        };
        warning_blocks.remove(block_index);
    }
    Ok(())
}

fn restored_pre_remediation_source(fixture: &Fixture) -> Result<String, String> {
    let source = fs::read_to_string(fixture.source)
        .map_err(|error| format!("{} should be readable: {error}", fixture.source))?;
    if source.contains(fixture.family) {
        return Ok(source.replacen(fixture.family, fixture.leaves, 1));
    }
    Ok(source)
}

#[test]
fn stdlib_error_family_test_projects_pre_remediation_warnings() -> Result<(), String> {
    for fixture in REMEDIATED_FIXTURES {
        let original = restored_pre_remediation_source(fixture)?;
        let temp_dir = unique_probe_target_dir("stdlib-error-family-pre-remediation");
        prepare_dir(&temp_dir)
            .map_err(|error| format!("temporary directory should be created: {error}"))?;
        let source_path = temp_dir.join("main.op");
        fs::write(&source_path, original)
            .map_err(|error| format!("temporary source should be written: {error}"))?;
        let result = assert_check(&source_path, Some(fixture));
        cleanup_dir(&temp_dir)
            .map_err(|error| format!("temporary directory should be removed: {error}"))?;
        result?;
    }
    Ok(())
}

#[test]
fn stdlib_error_family_test_projects() -> Result<(), String> {
    for fixture in REMEDIATED_FIXTURES {
        assert_check(Path::new(fixture.source), None)?;
    }
    Ok(())
}

#[test]
fn stdlib_error_family_test_projects_saferm_project_pre_remediation_warning() -> Result<(), String>
{
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/saferm");
    let temp_dir = unique_probe_target_dir("saferm-pre-remediation");
    prepare_dir(&temp_dir)
        .map_err(|error| format!("temporary directory should be created: {error}"))?;
    let preparation = (|| {
        copy_dir_recursive(&fixture_dir, &temp_dir)?;
        restore_saferm_pre_remediation_sources(&temp_dir)
    })();
    let result = preparation
        .and_then(|()| assert_saferm_project_build(&temp_dir, SAFERM_PRE_REMEDIATION_WARNINGS));
    cleanup_dir(&temp_dir)
        .map_err(|error| format!("temporary directory should be removed: {error}"))?;
    result
}

#[test]
fn stdlib_error_family_test_projects_saferm_project() -> Result<(), String> {
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/saferm");
    assert_saferm_project_build(&project_dir, &[])
}

#[test]
fn stdlib_error_family_test_projects_inventory_classifies_all_clauses() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects");
    let mut source_paths = Vec::new();
    collect_op_sources(&root, &mut source_paths)?;

    let mut clause_count: usize = 0;
    for source_path in source_paths {
        let source = fs::read_to_string(&source_path)
            .map_err(|error| format!("{} should be readable: {error}", source_path.display()))?;
        for (line_index, line) in source.lines().enumerate() {
            let Some((_, clause)) = line.split_once(" errors ") else {
                continue;
            };
            let Some((names, _)) = clause.split_once(" =>") else {
                continue;
            };
            clause_count += 1_usize;
            let names: Vec<_> = names.split(',').map(str::trim).collect();
            let has_declared_family = names
                .iter()
                .any(|name| ELIGIBLE_FAMILIES.iter().any(|&(family, _)| *name == family));
            if has_declared_family || names.len() == 1 {
                continue;
            }
            let clause_path = source_path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .map_err(|error| {
                    format!(
                        "{} should be inside the repository: {error}",
                        source_path.display()
                    )
                })?;
            let clause_id = format!("{}:{}", clause_path.display(), line_index + 1);
            for &(family, leaves) in ELIGIBLE_FAMILIES {
                if leaves.iter().all(|leaf| names.contains(leaf))
                    && !INTENTIONALLY_EXACT_CLAUSES.contains(&clause_id.as_str())
                {
                    return Err(format!(
                        "{clause_id} retains complete warning-eligible {family} leaves without a family declaration: {names:?}"
                    ));
                }
            }
        }
    }

    if clause_count == 0_usize {
        return Err("test-projects should contain errors clauses".to_owned());
    }
    Ok(())
}

fn collect_op_sources(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("{} should be readable: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("{} entry should be readable: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_op_sources(&path, sources)?;
        } else if path.extension().is_some_and(|extension| extension == "op")
            && path
                .components()
                .any(|component| component.as_os_str() == "src")
        {
            sources.push(path);
        }
    }
    Ok(())
}
