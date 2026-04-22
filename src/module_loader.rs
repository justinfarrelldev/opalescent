//! Module loading utilities for import path resolution and dependency discovery.

extern crate alloc;

use crate::ast::{Decl, ImportItem, Program};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::Span;
use crate::type_system::errors::TypeError;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Resolve an import source string to a concrete module path.
///
/// Supported forms:
/// - `./path` -> `<from_dir>/path.op`
/// - `./path.types` -> `<from_dir>/path.types.op`
/// - `standard` / `math` -> `__stdlib__/<name>` sentinel path
/// - `@scope/name` -> `TypeError::PackageImportNotSupported`
///
/// # Errors
/// Returns [`TypeError`] when package imports are used or the resolved file does not exist.
pub fn resolve_import_path(from_file: &Path, import_source: &str) -> Result<PathBuf, TypeError> {
    resolve_import_path_with_span(
        from_file,
        import_source,
        Span::single(crate::token::Position::start()),
    )
}

/// Internal helper that resolves import paths using a caller-provided source span.
fn resolve_import_path_with_span(
    from_file: &Path,
    import_source: &str,
    span: Span,
) -> Result<PathBuf, TypeError> {
    if matches!(import_source, "standard" | "math") {
        return Ok(PathBuf::from(format!("__stdlib__/{import_source}")));
    }

    if import_source.starts_with('@') {
        return Err(TypeError::PackageImportNotSupported {
            path: import_source.to_owned(),
            span: TypeError::span_from_span(span),
        });
    }

    let base_dir = from_file.parent().unwrap_or_else(|| Path::new("."));
    let mut resolved = if Path::new(import_source)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("types"))
    {
        let stem = import_source.trim_end_matches(".types");
        base_dir.join(format!("{stem}.types.op"))
    } else {
        let mut path = base_dir.join(import_source);
        if path.extension().is_none() {
            path.set_extension("op");
        }
        path
    };

    if !resolved.exists() {
        return Err(TypeError::ModuleNotFound {
            path: resolved.display().to_string(),
            span: TypeError::span_from_span(span),
        });
    }

    if let Ok(canonicalized) = resolved.canonicalize() {
        resolved = canonicalized;
    }

    Ok(resolved)
}

/// Checks if a file path represents a types file (ends with `.types.op`).
///
/// # Arguments
/// * `path` - The file path to check
///
/// # Returns
/// `true` if the path's filename ends with `.types.op`, `false` otherwise.
///
/// # Examples
/// ```
/// use std::path::Path;
/// use opalescent::module_loader::is_types_file;
///
/// assert!(is_types_file(Path::new("models.types.op")));
/// assert!(is_types_file(Path::new("dir/sub/models.types.op")));
/// assert!(!is_types_file(Path::new("models.op")));
/// assert!(!is_types_file(Path::new("models.types")));
/// ```
pub fn is_types_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|s| s.ends_with(".types.op"))
}

/// Validate that declarations in `program` match the role implied by `path`.
///
/// Rules:
/// - `.types.op` files may only contain `type` declarations and imports.
/// - non-`.types.op` files may not contain `type` declarations.
/// - stdlib sentinel module paths (`__stdlib__/...`) are exempt.
pub fn validate_module_file_role(path: &Path, program: &Program) -> Result<(), TypeError> {
    if path.to_string_lossy().starts_with("__stdlib__/") {
        return Ok(());
    }

    let file_path = path.display().to_string();

    if is_types_file(path) {
        #[expect(
            clippy::match_ref_pats,
            reason = "Pattern matching on borrowed declarations"
        )]
        for declaration in &program.declarations {
            match declaration {
                &(Decl::Type { .. } | Decl::Import { .. }) => {}
                &Decl::Let {
                    ref binding, span, ..
                } => {
                    return Err(TypeError::NonTypeDeclarationInTypesFile {
                        decl_kind: "let".to_owned(),
                        decl_name: binding.name.clone(),
                        file_path,
                        span: TypeError::span_from_span(span),
                    });
                }
                &Decl::Function {
                    ref name,
                    is_entry,
                    span,
                    ..
                } => {
                    let decl_kind = if is_entry { "entry" } else { "function" };
                    return Err(TypeError::NonTypeDeclarationInTypesFile {
                        decl_kind: decl_kind.to_owned(),
                        decl_name: name.clone(),
                        file_path,
                        span: TypeError::span_from_span(span),
                    });
                }
                &Decl::Comment { span, .. } => {
                    return Err(TypeError::NonTypeDeclarationInTypesFile {
                        decl_kind: "comment".to_owned(),
                        decl_name: "<comment>".to_owned(),
                        file_path,
                        span: TypeError::span_from_span(span),
                    });
                }
            }
        }

        return Ok(());
    }

    for declaration in &program.declarations {
        if let &Decl::Type { ref name, span, .. } = declaration {
            return Err(TypeError::TypeDeclarationOutsideTypesFile {
                type_name: name.clone(),
                file_path,
                span: TypeError::span_from_span(span),
            });
        }
    }

    Ok(())
}

/// Parsed module data used for dependency graph traversal.
#[derive(Debug, Clone)]
pub struct ParsedModule {
    pub path: PathBuf,
    pub source: String,
    pub ast: Program,
    pub imports: Vec<ImportInfo>,
}

/// Resolved import edge metadata.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source_path: String,
    pub resolved_path: PathBuf,
    pub is_type_import: bool,
    pub span: Span,
}

/// File-based module loader with source caching.
#[derive(Debug, Clone)]
pub struct ModuleLoader {
    /// Absolute project root used to resolve relative module paths.
    project_root: PathBuf,
    /// In-memory source text cache keyed by normalized absolute file path.
    source_cache: HashMap<PathBuf, String>,
}

impl ModuleLoader {
    /// Create a new module loader rooted at `project_root`.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            source_cache: HashMap::new(),
        }
    }

    /// Read module source from disk and cache by normalized path.
    ///
    /// # Errors
    /// Returns IO errors from filesystem reads.
    pub fn get_module_source(&mut self, path: &Path) -> Result<String, std::io::Error> {
        let normalized_path = self.normalize_path(path);
        if let Some(cached) = self.source_cache.get(&normalized_path) {
            return Ok(cached.clone());
        }

        let source = std::fs::read_to_string(&normalized_path)?;
        self.source_cache.insert(normalized_path, source.clone());
        Ok(source)
    }

    /// Discover all file-based modules reachable from `entry_path`.
    ///
    /// Returns topological order where dependencies come first and entry is last.
    ///
    /// # Errors
    /// Returns [`TypeError`] for unresolved imports, parse failures, or dependency cycles.
    pub fn discover_all_modules(&mut self, entry_path: &Path) -> Result<Vec<PathBuf>, TypeError> {
        let entry = self.normalize_path(entry_path);
        if !entry.exists() {
            return Err(TypeError::ModuleNotFound {
                path: entry.display().to_string(),
                span: TypeError::unknown_span(),
            });
        }

        let mut parsed_cache: BTreeMap<PathBuf, ParsedModule> = BTreeMap::new();
        let mut visited: BTreeSet<PathBuf> = BTreeSet::new();
        let mut visiting: BTreeSet<PathBuf> = BTreeSet::new();
        let mut visiting_stack: Vec<PathBuf> = Vec::new();
        let mut ordered: Vec<PathBuf> = Vec::new();

        self.discover_module_dfs(
            &entry,
            &mut parsed_cache,
            &mut visited,
            &mut visiting,
            &mut visiting_stack,
            &mut ordered,
        )?;

        Ok(ordered)
    }

    /// Traverse module imports depth-first and emit topologically sorted paths.
    ///
    /// # Errors
    /// Returns [`TypeError`] for cycle detection failures or nested module parse/resolve failures.
    fn discover_module_dfs(
        &mut self,
        current: &Path,
        parsed_cache: &mut BTreeMap<PathBuf, ParsedModule>,
        visited: &mut BTreeSet<PathBuf>,
        visiting: &mut BTreeSet<PathBuf>,
        visiting_stack: &mut Vec<PathBuf>,
        ordered: &mut Vec<PathBuf>,
    ) -> Result<(), TypeError> {
        let normalized = self.normalize_path(current);

        if visited.contains(&normalized) {
            return Ok(());
        }

        if visiting.contains(&normalized) {
            let cycle_start = visiting_stack
                .iter()
                .position(|path| path == &normalized)
                .unwrap_or_default();
            let mut cycle: Vec<String> = visiting_stack[cycle_start..]
                .iter()
                .map(|path| path.display().to_string())
                .collect();
            cycle.push(normalized.display().to_string());

            return Err(TypeError::CircularDependency {
                cycle,
                span: TypeError::unknown_span(),
            });
        }

        visiting.insert(normalized.clone());
        visiting_stack.push(normalized.clone());

        let parsed = if let Some(cached) = parsed_cache.get(&normalized) {
            cached.clone()
        } else {
            let parsed = self.parse_module(&normalized)?;
            parsed_cache.insert(normalized.clone(), parsed.clone());
            parsed
        };

        for import in &parsed.imports {
            if Self::is_stdlib_sentinel(&import.resolved_path) {
                continue;
            }
            self.discover_module_dfs(
                &import.resolved_path,
                parsed_cache,
                visited,
                visiting,
                visiting_stack,
                ordered,
            )?;
        }

        visiting.remove(&normalized);
        visiting_stack.pop();
        visited.insert(normalized.clone());
        ordered.push(normalized);
        Ok(())
    }

    /// Parse one module file and collect its import declarations.
    ///
    /// # Errors
    /// Returns [`TypeError`] when the module cannot be loaded, lexed, parsed, or imports fail resolution.
    fn parse_module(&mut self, path: &Path) -> Result<ParsedModule, TypeError> {
        let normalized = self.normalize_path(path);
        let source =
            self.get_module_source(&normalized)
                .map_err(|_io_err| TypeError::ModuleNotFound {
                    path: normalized.display().to_string(),
                    span: TypeError::unknown_span(),
                })?;
        let normalized_source = source.replace('\t', "    ");

        let lexer = Lexer::new(&normalized_source);
        let (tokens, lex_errors) = lexer.tokenize();
        if !lex_errors.errors.is_empty() {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "failed to lex module '{}': {} lexical error(s)",
                    normalized.display(),
                    lex_errors.errors.len()
                ),
                span: TypeError::unknown_span(),
            });
        }

        let parser = Parser::new(tokens);
        let (program, parse_errors) = parser.parse();
        if !parse_errors.errors.is_empty() {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "failed to parse module '{}': {} parse error(s)",
                    normalized.display(),
                    parse_errors.errors.len()
                ),
                span: TypeError::unknown_span(),
            });
        }

        let Some(ast) = program else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "parser returned no AST for module '{}'",
                    normalized.display()
                ),
                span: TypeError::unknown_span(),
            });
        };

        let mut imports = Vec::new();
        for declaration in &ast.declarations {
            if let &Decl::Import {
                ref items,
                source: ref import_source,
                span,
                ..
            } = declaration
            {
                let resolved_path =
                    resolve_import_path_with_span(&normalized, import_source, span)?;
                let is_type_import = items
                    .iter()
                    .all(|item| matches!(item, &ImportItem::Type { .. }));

                imports.push(ImportInfo {
                    source_path: import_source.clone(),
                    resolved_path,
                    is_type_import,
                    span,
                });
            }
        }

        Ok(ParsedModule {
            path: normalized,
            source: normalized_source,
            ast,
            imports,
        })
    }

    /// Normalize a path to an absolute canonical module file path when possible.
    fn normalize_path(&self, path: &Path) -> PathBuf {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };

        candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.clone())
    }

    /// Return true when a path is a stdlib sentinel (`__stdlib__/...`) rather than a filesystem file.
    fn is_stdlib_sentinel(path: &Path) -> bool {
        path.starts_with(Path::new("__stdlib__"))
    }
}

#[cfg(test)]
mod tests {
    use super::{ModuleLoader, resolve_import_path, validate_module_file_role};
    use crate::type_system::errors::TypeError;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("opalescent_{prefix}_{pid}_{nanos}"));
        std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
        dir
    }

    #[test]
    fn resolve_import_path_relative_op_module() {
        let base = make_temp_dir("resolve_relative");
        let from_file = base.join("main.op");
        let target = base.join("utils.op");

        std::fs::write(&from_file, "entry main = f(): void => { return void }\n")
            .expect("main file should be writable");
        std::fs::write(&target, "let util = f(): void => { return void }\n")
            .expect("target module should be writable");

        let resolved = resolve_import_path(&from_file, "./utils").expect("path should resolve");
        assert_eq!(
            resolved,
            target.canonicalize().expect("path should canonicalize")
        );

        std::fs::remove_dir_all(&base).expect("temp dir should be removable");
    }

    #[test]
    fn resolve_import_path_types_module_suffix() {
        let base = make_temp_dir("resolve_types");
        let from_file = base.join("main.op");
        let target = base.join("models.types.op");

        std::fs::write(&from_file, "entry main = f(): void => { return void }\n")
            .expect("main file should be writable");
        std::fs::write(&target, "type User: name: string\n")
            .expect("types module should be writable");

        let resolved =
            resolve_import_path(&from_file, "./models.types").expect("path should resolve");
        assert_eq!(
            resolved,
            target.canonicalize().expect("path should canonicalize")
        );

        std::fs::remove_dir_all(&base).expect("temp dir should be removable");
    }

    #[test]
    fn resolve_import_path_stdlib_sentinel() {
        let from_file = PathBuf::from("/tmp/main.op");
        let resolved = resolve_import_path(&from_file, "standard").expect("stdlib resolves");
        assert_eq!(resolved, PathBuf::from("__stdlib__/standard"));
    }

    #[test]
    fn resolve_import_path_package_import_not_supported() {
        let from_file = PathBuf::from("/tmp/main.op");
        let result = resolve_import_path(&from_file, "@scope/pkg");

        let error = result.expect_err("package imports should fail");
        assert!(matches!(
            error,
            TypeError::PackageImportNotSupported { ref path, .. } if path == "@scope/pkg"
        ));
    }

    #[test]
    fn discover_all_modules_returns_dependency_order() {
        let base = make_temp_dir("discover_order");
        let entry = base.join("main.op");
        let util = base.join("util.op");
        let helper = base.join("helper.op");

        std::fs::write(
            &entry,
            "import util from ./util\nimport print from standard\nentry main = f(): void => { return void }\n",
        )
        .expect("entry should be writable");
        std::fs::write(
            &util,
            "import helper from ./helper\nlet util = f(): void => { return void }\n",
        )
        .expect("util should be writable");
        std::fs::write(&helper, "let helper = f(): void => { return void }\n")
            .expect("helper should be writable");

        let mut loader = ModuleLoader::new(base.clone());
        let discovered = loader
            .discover_all_modules(&entry)
            .expect("module discovery should succeed");

        assert_eq!(
            discovered,
            vec![
                helper.canonicalize().expect("helper canonical path"),
                util.canonicalize().expect("util canonical path"),
                entry.canonicalize().expect("entry canonical path")
            ]
        );

        std::fs::remove_dir_all(&base).expect("temp dir should be removable");
    }

    #[test]
    fn discover_all_modules_detects_cycles() {
        let base = make_temp_dir("discover_cycle");
        let a = base.join("a.op");
        let b = base.join("b.op");

        std::fs::write(
            &a,
            "import b from ./b\nlet a = f(): void => { return void }\n",
        )
        .expect("a should be writable");
        std::fs::write(
            &b,
            "import a from ./a\nlet b = f(): void => { return void }\n",
        )
        .expect("b should be writable");

        let mut loader = ModuleLoader::new(base.clone());
        let result = loader.discover_all_modules(&a);
        let error = result.expect_err("cycle should be reported");
        assert!(matches!(error, TypeError::CircularDependency { .. }));

        std::fs::remove_dir_all(&base).expect("temp dir should be removable");
    }

    #[test]
    fn is_types_file_returns_true_for_types_op_files() {
        use super::is_types_file;
        use std::path::Path;

        assert!(is_types_file(Path::new("foo.types.op")));
        assert!(is_types_file(Path::new("dir/sub/models.types.op")));
    }

    #[test]
    fn is_types_file_returns_false_for_regular_op_files() {
        use super::is_types_file;
        use std::path::Path;

        assert!(!is_types_file(Path::new("foo.op")));
    }

    #[test]
    fn is_types_file_returns_false_for_types_without_op_extension() {
        use super::is_types_file;
        use std::path::Path;

        assert!(!is_types_file(Path::new("foo.types")));
    }

    // RED phase tests for validate_module_file_role
    // These tests call validate_module_file_role which does NOT yet exist.
    // They are expected to fail to compile (RED state).

    #[test]
    fn validate_op_file_allows_entry_and_let() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source =
            "entry main = f(): void => { return void }\nlet util = f(): void => { return void }\n";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.op"), &program);
        assert!(
            result.is_ok(),
            "regular .op file with entry and let should be valid"
        );
    }

    #[test]
    fn validate_op_file_rejects_type_decl() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "type User: name: string\n";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.op"), &program);
        assert!(
            result.is_err(),
            "regular .op file with type decl should fail"
        );

        if let Err(TypeError::TypeDeclarationOutsideTypesFile { type_name, .. }) = result {
            assert_eq!(type_name, "User");
        } else {
            unreachable!("Expected TypeDeclarationOutsideTypesFile error");
        }
    }

    #[test]
    fn validate_types_op_file_allows_type_decls() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "type User: name: string\ntype Admin: user: User\n";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.types.op"), &program);
        assert!(
            result.is_ok(),
            ".types.op file with only type decls should be valid"
        );
    }

    #[test]
    fn validate_types_op_file_rejects_let_decl() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "let x = 42\n";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.types.op"), &program);
        assert!(result.is_err(), ".types.op file with let decl should fail");

        if let Err(TypeError::NonTypeDeclarationInTypesFile { decl_kind, .. }) = result {
            assert_eq!(decl_kind, "let");
        } else {
            unreachable!("Expected NonTypeDeclarationInTypesFile error");
        }
    }

    #[test]
    fn validate_types_op_file_rejects_entry_decl() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "entry main = f(): void => { return void }\n";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.types.op"), &program);
        assert!(
            result.is_err(),
            ".types.op file with entry decl should fail"
        );

        if let Err(TypeError::NonTypeDeclarationInTypesFile { decl_kind, .. }) = result {
            assert_eq!(decl_kind, "entry");
        } else {
            unreachable!("Expected NonTypeDeclarationInTypesFile error");
        }
    }

    #[test]
    fn validate_types_op_file_rejects_function_let() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "let add = f(a: int32, b: int32): int32 => { return a + b }\n";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.types.op"), &program);
        assert!(
            result.is_err(),
            ".types.op file with function-typed let should fail"
        );

        if let Err(TypeError::NonTypeDeclarationInTypesFile { decl_kind, .. }) = result {
            assert_eq!(decl_kind, "let");
        } else {
            unreachable!("Expected NonTypeDeclarationInTypesFile error");
        }
    }

    #[test]
    fn validate_empty_op_file_ok() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.op"), &program);
        assert!(result.is_ok(), "empty .op file should be valid");
    }

    #[test]
    fn validate_empty_types_op_file_ok() {
        use crate::lexer::Lexer;
        use crate::parser::Parser;
        use std::path::Path;

        let source = "";
        let lexer = Lexer::new(source);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, _) = parser.parse();
        let program = program_opt.expect("program should parse");

        let result = validate_module_file_role(Path::new("test.types.op"), &program);
        assert!(result.is_ok(), "empty .types.op file should be valid");
    }
}
