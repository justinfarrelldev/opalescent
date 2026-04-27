# T33 Rerunnability Policy (`fs_*` suite)

## Scope
This policy applies to the 20 fs project fixtures used by the fs integration wave:

- `_fs_path_from`
- `_fs_normalize_path`
- `_fs_join_path_components`
- `_fs_path_file_extension`
- `_fs_path_file_name`
- `_fs_path_parent_directory`
- `_absolute_path_sync`
- `_fs_read_text_happy`
- `_fs_read_text_invalid_utf8`
- `_fs_read_contents_happy`
- `_fs_read_contents_is_dir`
- `_fs_read_contents_not_found`
- `_fs_read_lines_lf`
- `_fs_read_lines_crlf`
- `_fs_read_lines_mixed`
- `_fs_read_offset_happy`
- `_fs_read_offset_oob`
- `fs-directory-operations`
- `fs-path-manipulation`
- `fs-markdown-roundtrip`

## Clean-state definition
A fixture project is considered clean when all of the following are true:

1. All committed files in the project tree are byte-identical before and after a full `fs_` test run.
2. Generated `target/` and `workspace/` contents do not persist after test completion.
3. Each project includes `.gitignore` entries for exactly the two rerun-working dirs:
   - `target/`
   - `workspace/`

## Verification mechanism
`tests/integration_e2e/fs_rerunnability.rs` enforces rerunnability by:

1. Taking a SHA-256 manifest snapshot over all files in the 20 project directories.
2. Running `cargo test --features integration fs_ -- --skip fs_rerunnability --test-threads=1` as a subprocess.
3. Taking a second snapshot.
4. Asserting pre/post manifest equality.
5. Failing if any required project is missing `.gitignore` or the required entries.

## Operator checks
Required shell checks for T33 evidence:

- `cargo test --features integration fs_rerunnability`
- `cargo test --features integration fs_` (run twice)
- `git status --porcelain test-projects/`

Note: `git status --porcelain test-projects/` reflects full branch state under `test-projects/`; use it to detect cleanup leaks relative to your current working baseline.
