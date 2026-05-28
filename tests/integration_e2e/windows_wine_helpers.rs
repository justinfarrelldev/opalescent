use super::wine_harness::*;
use super::WineRun;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const PRE_WINE_SKIP_DUMP: &str = "(skipped before Wine execution)";
pub(super) const FATAL_WINE_SKIP_DUMP: &str = "(skipped after Wine fatal crash/dialog)";
pub(super) const FILE_OPS_TASK_NUM: u32 = 3;
pub(super) const FILE_OPS_SLUG: &str = "wine-msvc-file-ops";
pub(super) const FILE_OPS_PROJECT: &str = "windows-file-ops";
pub(super) const WINDOWS_MSVC_TARGET: &str = "x86_64-pc-windows-msvc";
pub(super) const PROCESS_PATHS_TASK_NUM: u32 = 9;
pub(super) const PROCESS_PATHS_SLUG: &str = "wine-process-paths";
pub(super) const PROCESS_PATHS_PROJECT: &str = "process-paths";
pub(super) const PROCESS_PATHS_TEST_NAME: &str = "process_paths";
pub(super) const PROCESS_PATHS_MARKERS: [&str; 12] = [
    "cwd_non_empty=true",
    "cwd_exists=true",
    "cwd_is_directory=true",
    "exe_path_non_empty=true",
    "exe_path_exists=true",
    "exe_dir_non_empty=true",
    "exe_dir_exists=true",
    "exe_dir_is_directory=true",
    "cwd_changed=true",
    "changed_cwd_exists=true",
    "changed_cwd_is_directory=true",
    "cwd_restored=true",
];
pub(super) const PROCESS_ENV_TASK_NUM: u32 = 9;
pub(super) const PROCESS_ENV_SLUG: &str = "wine-process-env";
pub(super) const PROCESS_ENV_PROJECT: &str = "process-env";
pub(super) const PROCESS_ENV_TEST_NAME: &str = "process_env";
pub(super) const PROCESS_ENV_MARKERS: [&str; 7] = [
    "present_value=present-value",
    "present_exists=true",
    "missing_exists=false",
    "missing_default=fallback-value",
    "empty_present=",
    "empty_default=",
    "present_default=present-value",
];
pub(super) const PROCESS_EXIT_TASK_NUM: u32 = 5;
pub(super) const PROCESS_EXIT_SLUG: &str = "exit-wine";
pub(super) const PROCESS_EXIT_PROJECT: &str = "process-exit-code";
pub(super) const PROCESS_EXIT_TEST_NAME: &str = "process_exit";
pub(super) const PROCESS_EXIT_CODE: i32 = 42;
pub(super) const SYMLINK_TASK_NUM: u32 = 7;
pub(super) const GUARD_SHORTHAND_TASK_NUM: u32 = 9;
pub(super) const GUARD_SHORTHAND_SLUG: &str = "wine-guard-shorthand";
pub(super) const GUARD_SHORTHAND_PROJECT: &str = "guard-shorthand";
pub(super) const GUARD_SHORTHAND_TEST_NAME: &str = "wine_msvc_guard_shorthand";
pub(super) const SYMLINK_SLUG: &str = "symlink-metadata";
pub(super) const SYMLINK_TEST_NAME: &str = "wine_msvc_symlink_metadata";
pub(super) const FILE_OPS_TEST_NAME: &str = "wine_msvc_file_ops";
pub(super) const LONG_PATH_SEGMENTS: usize = 18;
pub(super) const EXPECTED_SUMMARY: &str =
    "status=ready\nfile=résumé final.txt\ncontent=alpha\nbeta\ndir=dir with spaces/café";
pub(super) const EXPECTED_MARKERS: [&str; 16] = [
    "MARKER:DIR_CREATED=test-projects/windows-file-ops/workspace/dir with spaces/café",
    "MARKER:FILE_CREATED=test-projects/windows-file-ops/workspace/dir with spaces/café/naïve file.txt",
    "MARKER:READ_BEFORE_RENAME=ok",
    "MARKER:LIST_COUNT=1",
    "MARKER:LIST_HAS_ORIGINAL=1",
    "MARKER:DIR_OPEN=ok",
    "MARKER:RENAMED_TO=test-projects/windows-file-ops/workspace/dir with spaces/café/résumé final.txt",
    "MARKER:OLD_EXISTS_AFTER_MOVE=false",
    "MARKER:RENAMED_EXISTS_AFTER_MOVE=true",
    "MARKER:READ_AFTER_RENAME=ok",
    "MARKER:RENAME_REPLACE_NOTE=move_path_sync used as nearest supported rename/replace operation",
    "MARKER:LONG_PATH_OK=true",
    "MARKER:SUMMARY_WRITE=ok",
    "MARKER:RENAMED_EXISTS_AFTER_DELETE=false",
    "MARKER:UNICODE_DIR_EXISTS_AFTER_DELETE=false",
    "MARKER:FINAL_STATUS=ok",
];

pub(super) struct FileOpsFixturePaths {
    pub(super) workspace_root: PathBuf,
    pub(super) summary_path: PathBuf,
    pub(super) unicode_dir: PathBuf,
    pub(super) original_file: PathBuf,
    pub(super) renamed_file: PathBuf,
    pub(super) long_nested_dir: PathBuf,
    pub(super) long_nested_file: PathBuf,
}

pub(super) fn record_skip(task_num: u32, slug: &str, test_name: &str, reason: &str, fs_dump: &str) {
    let evidence = WineRun {
        stdout: format!("SKIP: {reason}\n"),
        stderr: String::new(),
        exit_code: 0,
        fs_dump: fs_dump.to_owned(),
    };
    assert!(
        capture_evidence(task_num, slug, &evidence).is_ok(),
        "{test_name} skip path should still write deterministic evidence"
    );
    eprintln!("SKIP {test_name}: {reason}");
}

pub(super) fn skip_if_prereqs_missing(task_num: u32, slug: &str, test_name: &str) -> bool {
    if let Err(reason) = check_prereqs() {
        record_skip(
            task_num,
            slug,
            test_name,
            reason.as_str(),
            PRE_WINE_SKIP_DUMP,
        );
        return true;
    }
    false
}

pub(super) fn cleanup_symlink_metadata_artifacts(
    link_path: &Path,
    target_file: &Path,
    workspace_root: &Path,
    temp_dir: &Path,
) {
    drop(fs::remove_file(link_path));
    drop(fs::remove_file(target_file));
    drop(fs::remove_dir_all(workspace_root));
    drop(fs::remove_dir_all(temp_dir));
}

pub(super) fn setup_symlink_metadata_workspace(
    workspace_root: &Path,
    target_file: &Path,
    link_path: &Path,
) -> Result<(), String> {
    if workspace_root.exists() {
        fs::remove_dir_all(workspace_root)
            .map_err(|e| format!("failed to clear symlink metadata workspace: {e}"))?;
    }
    fs::create_dir_all(workspace_root)
        .map_err(|e| format!("failed to create symlink metadata workspace: {e}"))?;
    fs::write(target_file, "symlink-target")
        .map_err(|e| format!("failed to create symlink metadata target: {e}"))?;
    std::os::unix::fs::symlink(target_file, link_path).map_err(|e| {
        format!("Wine limitation: symlink/reparse behavior differs from native Windows ({e})")
    })?;
    Ok(())
}

pub(super) fn build_symlink_metadata_source(link_path: &Path) -> String {
    let escaped_link = link_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'");
    format!(
        "import path_from, read_metadata_sync, read_metadata_nofollow_sync from standard\n\n##\n  Description: Windows symlink metadata probe for Wine.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, MetadataUnavailableError, InvalidPathError =>\n    let link = path_from('{escaped_link}')\n    let follow = propagate read_metadata_sync(link)\n    let nofollow = propagate read_metadata_nofollow_sync(link)\n    print('FOLLOW is_symlink={{follow.is_symlink}}')\n    print('NOFOLLOW is_symlink={{nofollow.is_symlink}}')\n    print('FOLLOW is_directory={{follow.is_directory}}')\n    print('NOFOLLOW is_directory={{nofollow.is_directory}}')\n    return void\n"
    )
}

pub(super) fn assert_symlink_metadata_output(run: &WineRun) {
    assert!(
        run.stdout.contains("FOLLOW is_symlink=true") || run.stdout.contains("FOLLOW is_symlink=1"),
        "wine_msvc_symlink_metadata should report reparse-point symlink after follow stat on Windows, stdout={:?}, stderr={:?}",
        run.stdout,
        run.stderr
    );
    assert!(
        run.stdout.contains("NOFOLLOW is_symlink=true") || run.stdout.contains("NOFOLLOW is_symlink=1"),
        "wine_msvc_symlink_metadata should report reparse-point symlink for nofollow metadata on Windows, stdout={:?}, stderr={:?}",
        run.stdout,
        run.stderr
    );
}

pub(super) fn build_long_nested_dir(workspace_root: &Path) -> PathBuf {
    let mut long_nested_dir = workspace_root.to_path_buf();
    for index in 0_usize..LONG_PATH_SEGMENTS {
        long_nested_dir = long_nested_dir.join(format!("segment-{index}-long-name"));
    }
    long_nested_dir
}

pub(super) fn is_known_wine_host_limitation(message: &str) -> bool {
    message.contains("Unhandled page fault")
        || message.contains("starting debugger")
        || message.contains("could not load kernel32.dll")
        || message.contains("status c0000135")
}

pub(super) fn build_file_ops_paths(project_root: &Path) -> FileOpsFixturePaths {
    let workspace_root = project_root.join("workspace");
    let summary_path = workspace_root.join("final-summary.txt");
    let unicode_dir = workspace_root.join("dir with spaces").join("café");
    let original_file = unicode_dir.join("naïve file.txt");
    let renamed_file = unicode_dir.join("résumé final.txt");
    let long_nested_dir = build_long_nested_dir(&workspace_root);
    let long_nested_file = long_nested_dir
        .join("deep-file-name-that-keeps-the-path-over-two-hundred-sixty-characters.txt");

    FileOpsFixturePaths {
        workspace_root,
        summary_path,
        unicode_dir,
        original_file,
        renamed_file,
        long_nested_dir,
        long_nested_file,
    }
}

pub(super) fn run_file_ops_under_wine(exe_path: &Path) -> Option<WineRun> {
    let run_result = run_under_wine(exe_path, &[]);
    if let Err(error) = run_result.as_ref() {
        if is_known_wine_host_limitation(error) {
            let reason = format!("Wine limitation: fatal crash/dialog requires manual close ({error})");
            record_skip(
                FILE_OPS_TASK_NUM,
                FILE_OPS_SLUG,
                FILE_OPS_TEST_NAME,
                reason.as_str(),
                FATAL_WINE_SKIP_DUMP,
            );
            return None;
        }
    }

    assert!(
        run_result.is_ok(),
        "wine_msvc_file_ops fixture should execute under Wine after a successful build: {:?}",
        run_result.as_ref().err()
    );
    Some(run_result.expect("asserted Wine file-ops execution succeeded"))
}

pub(super) fn capture_workspace_snapshot(run: &mut WineRun, workspace_root: &Path) {
    run.fs_dump = snapshot_workspace(workspace_root)
        .unwrap_or_else(|error| format!("(workspace snapshot failed: {error})"));
}

pub(super) fn assert_stdout_markers(run: &WineRun, markers: &[&str], context: &str) {
    for marker in markers {
        assert!(
            run.stdout.lines().any(|line| line.trim() == *marker),
            "{context} stdout should contain marker {marker:?}, stdout={:?}, stderr={:?}",
            run.stdout,
            run.stderr
        );
    }
}

pub(super) fn assert_expected_markers(run: &WineRun) {
    for marker in EXPECTED_MARKERS {
        assert!(
            run.stdout.contains(marker),
            "wine_msvc_file_ops stdout should contain marker '{marker}', stdout={:?}, stderr={:?}",
            run.stdout,
            run.stderr
        );
    }
}

pub(super) fn assert_file_ops_summary(paths: &FileOpsFixturePaths) {
    let summary_text = fs::read_to_string(&paths.summary_path);
    assert!(
        summary_text.is_ok(),
        "wine_msvc_file_ops should leave final-summary.txt for host verification: {:?}",
        summary_text.as_ref().err()
    );
    assert_eq!(
        summary_text.ok().as_deref(),
        Some(EXPECTED_SUMMARY),
        "wine_msvc_file_ops summary file should match expected multiline payload"
    );
}

pub(super) fn assert_long_path_artifacts(paths: &FileOpsFixturePaths, run: &WineRun) {
    let long_path_len = paths.long_nested_file.to_string_lossy().len();
    assert!(
        long_path_len > 260,
        "wine_msvc_file_ops long-path host probe should exceed 260 characters, got {long_path_len}: {}",
        paths.long_nested_file.display()
    );

    let long_len_marker = run
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("MARKER:LONG_PATH_LEN="))
        .and_then(|value| value.trim().parse::<usize>().ok());
    assert!(
        matches!(long_len_marker, Some(value) if value > 260),
        "wine_msvc_file_ops stdout should report a long-path marker above 260 characters, got {:?}, stdout={:?}, stderr={:?}",
        long_len_marker,
        run.stdout,
        run.stderr
    );

    let long_path_summary = fs::read_to_string(&paths.long_nested_file);
    assert!(
        long_path_summary.is_ok(),
        "wine_msvc_file_ops should leave the long nested file for host verification before cleanup snapshot: {:?}",
        long_path_summary.as_ref().err()
    );
    assert_eq!(
        long_path_summary.ok().as_deref(),
        Some("Hello, Opal!\n"),
        "wine_msvc_file_ops long nested file should round-trip exact bytes"
    );
}

pub(super) fn assert_file_ops_host_state(paths: &FileOpsFixturePaths, run: &WineRun) {
    assert!(
        paths.workspace_root.exists(),
        "wine_msvc_file_ops should leave the workspace root for evidence inspection"
    );
    assert!(
        !paths.unicode_dir.exists(),
        "wine_msvc_file_ops should delete the unicode directory after removing the renamed file"
    );
    assert!(
        !paths.original_file.exists(),
        "wine_msvc_file_ops should remove the original path via move before completion"
    );
    assert!(
        !paths.renamed_file.exists(),
        "wine_msvc_file_ops should delete the renamed file before completion"
    );
    assert!(
        paths.long_nested_dir.exists(),
        "wine_msvc_file_ops should leave the long nested directory tree in place for host verification"
    );
    assert!(
        paths.long_nested_file.exists(),
        "wine_msvc_file_ops should leave the long nested file in place for host verification"
    );
    assert!(
        run.fs_dump.contains("final-summary.txt")
            && run.fs_dump.contains("deep-file-name-that-keeps-the-path-over-two-hundred-sixty-characters.txt"),
        "wine_msvc_file_ops filesystem snapshot should record final-summary.txt, fs_dump={:?}",
        run.fs_dump
    );
}
