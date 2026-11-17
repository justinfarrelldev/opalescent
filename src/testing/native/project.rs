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
use alloc::collections::{BTreeMap, BTreeSet};
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
    #[error("ordinary application code cannot import test-only support module {0:?}")]
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
        let mut pending_tests = BTreeSet::new();
        let mut pending_applications = BTreeSet::new();
        let mut roots = Vec::new();
        for test_file in test_files {
            let path = canonical_path(test_file)?;
            pending_tests.insert(path.clone());
            roots.push(path);
        }
        let mut modules = BTreeMap::new();
        // Establish the complete explicit test-support closure first. A test
        // may then import an ordinary helper from one of those support modules
        // without making grammar selection depend on traversal/source order.
        while let Some(path) = pending_tests.pop_first() {
            if modules.contains_key(&path) {
                continue;
            }
            let module = load_source_module(&root, &path, ModuleMode::Test)?;
            for (dependency, mode) in module_load_dependencies(&path, &module.ast)? {
                if mode == ModuleMode::Test {
                    pending_tests.insert(dependency);
                } else {
                    pending_applications.insert(dependency);
                }
            }
            modules.insert(path, module);
        }
        pending_applications.retain(|path| !modules.contains_key(path));
        while let Some(path) = pending_applications.pop_first() {
            if modules.contains_key(&path) {
                continue;
            }
            let module = load_source_module(&root, &path, ModuleMode::Application)?;
            for (dependency, _) in module_dependencies(&path, &module.ast)? {
                if modules.get(&dependency).is_some_and(|dependency_module| {
                    matches!(dependency_module.ast, LoadedModuleAst::Test(_))
                }) {
                    return Err(NativeLoadError::ModeConflict(dependency));
                }
                if !modules.contains_key(&dependency) {
                    pending_applications.insert(dependency);
                }
            }
            modules.insert(path, module);
        }
        Ok(Self {
            root,
            modules,
            test_files: roots,
        })
    }
}

/// Read one source exactly once after its module role has been established.
fn load_source_module(
    root: &Path,
    path: &Path,
    mode: ModuleMode,
) -> Result<LoadedModule, NativeLoadError> {
    let source = std::fs::read_to_string(path).map_err(|error| NativeLoadError::Io {
        path: path.to_owned(),
        error,
    })?;
    let ast = parse_module(path, &source, mode)?;
    let identity = relative_module_identity(root, path)?;
    Ok(LoadedModule {
        path: path.to_owned(),
        identity,
        source,
        ast,
    })
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
pub(super) fn module_dependencies(
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

/// Double declarations reference source modules for static edge validation,
/// but these references are not application import edges or module cycles.
fn module_load_dependencies(
    path: &Path,
    ast: &LoadedModuleAst,
) -> Result<Vec<(PathBuf, ModuleMode)>, NativeLoadError> {
    let mut dependencies = module_dependencies(path, ast)?;
    if let LoadedModuleAst::Test(ref module) = *ast {
        for item in &module.items {
            if let TestItem::Double(ref double) = *item {
                for source in [&double.from, &double.as_used_by] {
                    let dependency = resolve_import_path(path, source).map_err(|error| {
                        NativeLoadError::Type {
                            path: path.to_owned(),
                            error: Box::new(error),
                        }
                    })?;
                    if !dependency.starts_with("__stdlib__") {
                        dependencies.push((dependency, ModuleMode::Application));
                    }
                }
            }
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
