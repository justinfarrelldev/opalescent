//! Source-preserving discovery of test modules and their ordinary dependencies.
extern crate alloc;
use super::identity::DeclarationIdentity;
use crate::ast::testing::{TestItem, TestModule};
use crate::ast::{Decl, Program};
use crate::errors::reporter::CompilationErrorReport;
use crate::lexer::{Lexer, ModuleMode};
use crate::module_loader::{is_types_file, resolve_import_path, validate_module_file_role};
use crate::parser::Parser;
use crate::type_system::errors::TypeError;
use alloc::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

/// Parsed grammar selected explicitly for a reachable module.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadedModuleAst {
    /// Test root or support module reached via an explicit World/double import.
    Test(TestModule),
    /// Ordinary dependency parsed without conversational framework syntax.
    Application(Program),
}

/// Source and structured AST retained for subsequent resolution and diagnostics.
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// Canonical filesystem path used for I/O and dependency resolution.
    pub path: PathBuf,
    /// Project-relative module identity with normalized separators.
    pub identity: String,
    /// Original source bytes, never regex-rewritten or tab-expanded.
    pub source: String,
    /// Explicit grammar selection and parsed source tree.
    pub ast: LoadedModuleAst,
}

impl LoadedModule {
    /// Construct a canonical identity for one declaration in this defining module.
    #[must_use]
    pub fn declaration_identity(&self, name: &str) -> DeclarationIdentity {
        DeclarationIdentity {
            module: self.identity.clone(),
            name: name.to_owned(),
        }
    }
}

/// Compilation/discovery failure, distinct from any runtime test assertion.
#[derive(Debug, Error)]
pub enum NativeLoadError {
    /// Required source could not be read or canonicalized.
    #[error("cannot load test source {path:?}: {error}")]
    Io {
        /// Source filesystem path.
        path: PathBuf,
        /// Original I/O error.
        #[source]
        error: std::io::Error,
    },
    /// Original lexical/parser diagnostics, including original source spans.
    #[error("test compilation failed in {path:?}")]
    Frontend {
        /// Module producing diagnostics.
        path: PathBuf,
        /// Original source for diagnostic rendering.
        source_text: String,
        /// Normal compiler error collection, not a stringified test outcome.
        report: Box<CompilationErrorReport>,
    },
    /// Existing import or file-role error.
    #[error("test compilation failed in {path:?}: {error}")]
    Type {
        /// Module producing the failure.
        path: PathBuf,
        /// Ordinary compiler diagnostic.
        error: Box<TypeError>,
    },
    /// One module must not be parsed in conflicting production/test roles.
    #[error("test-only module {0:?} must be reached through an explicit World/double import")]
    ModeConflict(PathBuf),
    /// Generated module identities cannot silently collapse non-UTF-8 paths.
    #[error("module path has no portable UTF-8 identity: {0:?}")]
    InvalidIdentity(PathBuf),
}

/// Reachable source graph. Loading never executes application code or setup.
#[derive(Debug, Clone)]
pub struct NativeTestProject {
    /// Canonical project root.
    pub root: PathBuf,
    /// Loaded modules, ordered by canonical filesystem path.
    pub modules: BTreeMap<PathBuf, LoadedModule>,
    /// Explicitly selected test roots, separate from reusable support modules.
    pub test_files: Vec<PathBuf>,
}

impl NativeTestProject {
    /// Load selected roots and all dependencies with explicit module modes.
    ///
    /// # Errors
    /// Returns original frontend, import, file-role and filesystem diagnostics.
    pub fn load(root: &Path, test_files: &[PathBuf]) -> Result<Self, NativeLoadError> {
        let root = canonical_path(root)?;
        let mut modes = BTreeMap::new();
        let mut pending = BTreeMap::new();
        let mut roots = Vec::new();
        for test_file in test_files {
            let path = canonical_path(test_file)?;
            modes.insert(path.clone(), ModuleMode::Test);
            pending.insert(path.clone(), ModuleMode::Test);
            roots.push(path);
        }
        let mut modules = BTreeMap::new();
        while let Some((path, mode)) = pending.pop_first() {
            let source = std::fs::read_to_string(&path).map_err(|error| NativeLoadError::Io {
                path: path.clone(),
                error,
            })?;
            let ast = parse_module(&path, &source, mode)?;
            let dependencies = module_dependencies(&path, &ast)?;
            for (dependency, dependency_mode) in dependencies {
                if let Some(previous_mode) = modes.get(&dependency) {
                    if *previous_mode != dependency_mode {
                        return Err(NativeLoadError::ModeConflict(dependency));
                    }
                } else {
                    modes.insert(dependency.clone(), dependency_mode);
                    pending.insert(dependency, dependency_mode);
                }
            }
            let identity = relative_module_identity(&root, &path)?;
            modules.insert(
                path.clone(),
                LoadedModule {
                    path,
                    identity,
                    source,
                    ast,
                },
            );
        }
        Ok(Self {
            root,
            modules,
            test_files: roots,
        })
    }
}

/// Canonicalize at the I/O boundary, preserving the original error path.
fn canonical_path(path: &Path) -> Result<PathBuf, NativeLoadError> {
    path.canonicalize().map_err(|error| NativeLoadError::Io {
        path: path.to_owned(),
        error,
    })
}

/// Parse using original source bytes and existing compiler error collections.
fn parse_module(
    path: &Path,
    source: &str,
    mode: ModuleMode,
) -> Result<LoadedModuleAst, NativeLoadError> {
    let lexer = if mode == ModuleMode::Test {
        Lexer::new_test_module(source)
    } else {
        Lexer::new(source)
    };
    let (tokens, lex_errors) = lexer.tokenize();
    let mut report = CompilationErrorReport::new();
    report.extend_lex_errors(lex_errors.errors);
    let ast = if report.is_empty() {
        if mode == ModuleMode::Test {
            let (module, errors) = Parser::new(tokens).parse_test_module();
            report.extend_parse_errors(errors.errors);
            module.map(LoadedModuleAst::Test)
        } else {
            let (program, errors) = Parser::new(tokens).parse();
            report.extend_parse_errors(errors.errors);
            program.map(LoadedModuleAst::Application)
        }
    } else {
        None
    };
    let Some(ast) = ast.filter(|_| report.is_empty()) else {
        return Err(NativeLoadError::Frontend {
            path: path.to_owned(),
            source_text: source.to_owned(),
            report: Box::new(report),
        });
    };
    let program = match ast {
        LoadedModuleAst::Application(ref program) => program.clone(),
        LoadedModuleAst::Test(ref module) => {
            if is_types_file(path)
                && module
                    .items
                    .iter()
                    .any(|item| !matches!(*item, TestItem::Application(_)))
            {
                return Err(NativeLoadError::Type {
                    path: path.to_owned(),
                    error: Box::new(TypeError::InvalidOperation {
                        operation:
                            "Worlds, doubles and test declarations cannot appear in .types.op files"
                                .to_owned(),
                        type_name: "test declaration".to_owned(),
                        span: TypeError::span_from_span(module.span),
                    }),
                });
            }
            Program {
                declarations: module
                    .items
                    .iter()
                    .filter_map(|item| match *item {
                        TestItem::Application(ref declaration) => Some(*declaration.clone()),
                        _ => None,
                    })
                    .collect(),
                span: module.span,
                id: crate::ast::NodeId(0),
            }
        }
    };
    validate_module_file_role(path, &program).map_err(|error| NativeLoadError::Type {
        path: path.to_owned(),
        error: Box::new(error),
    })?;
    Ok(ast)
}

/// Extract direct dependencies without invoking any source-level callback.
fn module_dependencies(
    path: &Path,
    ast: &LoadedModuleAst,
) -> Result<Vec<(PathBuf, ModuleMode)>, NativeLoadError> {
    let mut sources = Vec::new();
    match *ast {
        LoadedModuleAst::Application(ref program) => {
            for declaration in &program.declarations {
                if let Decl::Import { ref source, .. } = *declaration {
                    sources.push((source.as_str(), ModuleMode::Application));
                }
            }
        }
        LoadedModuleAst::Test(ref module) => {
            for item in &module.items {
                match *item {
                    TestItem::SupportImport(ref import) => {
                        sources.push((import.source.as_str(), ModuleMode::Test));
                    }
                    TestItem::Application(ref declaration) => {
                        if let Decl::Import { ref source, .. } = **declaration {
                            sources.push((source.as_str(), ModuleMode::Application));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let mut dependencies = Vec::new();
    for (source, mode) in sources {
        let dependency =
            resolve_import_path(path, source).map_err(|error| NativeLoadError::Type {
                path: path.to_owned(),
                error: Box::new(error),
            })?;
        if !dependency.starts_with("__stdlib__") {
            dependencies.push((dependency, mode));
        }
    }
    Ok(dependencies)
}

/// Canonical relative names also support explicit dependencies outside the root.
fn relative_module_identity(root: &Path, path: &Path) -> Result<String, NativeLoadError> {
    let root_components: Vec<_> = root.components().collect();
    let path_components: Vec<_> = path.components().collect();
    let common = root_components
        .iter()
        .zip(&path_components)
        .take_while(|&(left, right)| left == right)
        .count();
    let mut relative = PathBuf::new();
    for _ in root_components.iter().skip(common) {
        relative.push("..");
    }
    for component in path_components.iter().skip(common) {
        if matches!(*component, Component::Prefix(_) | Component::RootDir) {
            return Err(NativeLoadError::InvalidIdentity(path.to_owned()));
        }
        relative.push(component.as_os_str());
    }
    relative
        .to_str()
        .map(|name| name.replace('\\', "/"))
        .ok_or_else(|| NativeLoadError::InvalidIdentity(path.to_owned()))
}
