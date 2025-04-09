// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

pub mod ast;
use crate::io::SourceResolver;
use ast::{Program, StmtKind};
use mut_visit::MutVisitor;
use qsc_data_structures::span::Span;
use qsc_frontend::compile::SourceMap;
use qsc_frontend::error::WithSource;
use scan::ParserContext;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(test)]
pub(crate) mod tests;

pub mod completion;
mod error;
pub use error::Error;
mod expr;
mod mut_visit;
mod prgm;
mod prim;
mod scan;
mod stmt;

struct Offsetter(pub(super) u32);

impl MutVisitor for Offsetter {
    fn visit_span(&mut self, span: &mut Span) {
        span.lo += self.0;
        span.hi += self.0;
    }
}

pub struct QasmParseResult {
    pub source: QasmSource,
    pub source_map: SourceMap,
}

impl QasmParseResult {
    #[must_use]
    pub fn new(source: QasmSource) -> QasmParseResult {
        let source_map = create_source_map(&source);
        let mut source = source;
        update_offsets(&source_map, &mut source);
        QasmParseResult { source, source_map }
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.source.has_errors()
    }

    pub fn all_errors(&self) -> Vec<WithSource<crate::Error>> {
        let mut self_errors = self.errors();
        let include_errors = self
            .source
            .includes()
            .iter()
            .flat_map(QasmSource::all_errors)
            .map(|e| self.map_error(e))
            .collect::<Vec<_>>();

        self_errors.extend(include_errors);
        self_errors
    }

    #[must_use]
    pub fn errors(&self) -> Vec<WithSource<crate::Error>> {
        self.source
            .errors()
            .iter()
            .map(|e| self.map_error(e.clone()))
            .collect::<Vec<_>>()
    }

    fn map_error(&self, error: Error) -> WithSource<crate::Error> {
        WithSource::from_map(
            &self.source_map,
            crate::Error(crate::ErrorKind::Parser(error)),
        )
    }
}

/// all spans and errors spans are relative to the start of the file
/// We need to update the spans based on the offset of the file in the source map.
/// We have to do this after a full parse as we don't know what files will be loaded
/// until we have parsed all the includes.
fn update_offsets(source_map: &SourceMap, source: &mut QasmSource) {
    let source_file = source_map.find_by_name(&source.path().display().to_string());
    let offset = source_file.map_or(0, |source| source.offset);
    // Update the errors' offset
    source
        .errors
        .iter_mut()
        .for_each(|e| *e = e.clone().with_offset(offset));
    // Update the program's spans with the offset
    let mut offsetter = Offsetter(offset);
    offsetter.visit_program(&mut source.program);

    // Recursively update the includes, their programs, and errors
    for include in source.includes_mut() {
        update_offsets(source_map, include);
    }
}

/// Parse a QASM file and return the parse result.
/// This function will resolve includes using the provided resolver.
/// If an include file cannot be resolved, an error will be returned.
/// If a file is included recursively, a stack overflow occurs.
pub fn parse_source<S, P, R>(source: S, path: P, resolver: &mut R) -> QasmParseResult
where
    S: AsRef<str>,
    P: AsRef<Path>,
    R: SourceResolver,
{
    let res = parse_qasm_source(source, path, resolver);
    QasmParseResult::new(res)
}

/// Creates a Q# source map from a QASM parse output. The `QasmSource`
/// has all of the recursive includes resolved with their own source
/// and parse results.
fn create_source_map(source: &QasmSource) -> SourceMap {
    let mut files: Vec<(Arc<str>, Arc<str>)> = Vec::new();
    collect_source_files(source, &mut files);
    SourceMap::new(files, None)
}

/// Recursively collect all source files from the includes
fn collect_source_files(source: &QasmSource, files: &mut Vec<(Arc<str>, Arc<str>)>) {
    files.push((
        Arc::from(source.path().to_string_lossy().to_string()),
        Arc::from(source.source()),
    ));
    // Collect all source files from the includes, this
    // begins the recursive process of collecting all source files.
    for include in source.includes() {
        collect_source_files(include, files);
    }
}

/// Represents a QASM source file that has been parsed.
#[derive(Clone, Debug)]
pub struct QasmSource {
    /// The path to the source file. This is used for error reporting.
    /// This path is just a name, it does not have to exist on disk.
    path: PathBuf,
    /// The source code of the file.
    source: Arc<str>,
    /// The parsed AST of the source file or any parse errors.
    program: Program,
    /// Any parse errors that occurred.
    errors: Vec<Error>,
    /// Any included files that were resolved.
    /// Note that this is a recursive structure.
    included: Vec<QasmSource>,
}

impl QasmSource {
    pub fn new<T: AsRef<str>, P: AsRef<Path>>(
        source: T,
        file_path: P,
        program: Program,
        errors: Vec<Error>,
        included: Vec<QasmSource>,
    ) -> QasmSource {
        QasmSource {
            path: file_path.as_ref().to_owned(),
            source: source.as_ref().into(),
            program,
            errors,
            included,
        }
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        if !self.errors().is_empty() {
            return true;
        }
        self.includes().iter().any(QasmSource::has_errors)
    }

    #[must_use]
    pub fn all_errors(&self) -> Vec<crate::parser::Error> {
        let mut self_errors = self.errors();
        let include_errors = self.includes().iter().flat_map(QasmSource::all_errors);
        self_errors.extend(include_errors);
        self_errors
    }

    #[must_use]
    pub fn includes(&self) -> &Vec<QasmSource> {
        self.included.as_ref()
    }

    #[must_use]
    pub fn includes_mut(&mut self) -> &mut Vec<QasmSource> {
        self.included.as_mut()
    }

    #[must_use]
    pub fn program(&self) -> &Program {
        &self.program
    }

    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    #[must_use]
    pub fn errors(&self) -> Vec<crate::parser::Error> {
        self.errors.clone()
    }

    #[must_use]
    pub fn source(&self) -> &str {
        self.source.as_ref()
    }
}

/// Parse a QASM file and return the parse result using the provided resolver.
/// Returns `Err` if the resolver cannot resolve the file.
/// Returns `Ok` otherwise. Any parse errors will be included in the result.
///
/// This function is the start of a recursive process that will resolve all
/// includes in the QASM file. Any includes are parsed as if their contents
/// were defined where the include statement is.
fn parse_qasm_file<P, R>(path: P, resolver: &mut R) -> QasmSource
where
    P: AsRef<Path>,
    R: SourceResolver,
{
    match resolver.resolve(&path) {
        Ok((path, source)) => {
            let parse_result = parse_qasm_source(source, path, resolver);

            // Once we finish parsing the source, we pop the file from the
            // resolver. This is needed to keep track of multiple includes
            // and cyclic includes.
            resolver.ctx().pop_current_file();

            parse_result
        }
        Err(e) => {
            let error = crate::parser::error::ErrorKind::IO(e);
            let error = crate::parser::Error(error, None);
            QasmSource {
                path: path.as_ref().to_owned(),
                source: Default::default(),
                program: Program {
                    span: Span::default(),
                    statements: vec![].into_boxed_slice(),
                    version: None,
                },
                errors: vec![error],
                included: vec![],
            }
        }
    }
}

fn parse_qasm_source<S, P, R>(source: S, path: P, resolver: &mut R) -> QasmSource
where
    S: AsRef<str>,
    P: AsRef<Path>,
    R: SourceResolver,
{
    let (program, errors, includes) = parse_source_and_includes(source.as_ref(), resolver);
    QasmSource::new(source, path, program, errors, includes)
}

fn parse_source_and_includes<P: AsRef<str>, R>(
    source: P,
    resolver: &mut R,
) -> (Program, Vec<Error>, Vec<QasmSource>)
where
    R: SourceResolver,
{
    let (program, errors) = parse(source.as_ref());
    let included = parse_includes(&program, resolver);
    (program, errors, included)
}

fn parse_includes<R>(program: &Program, resolver: &mut R) -> Vec<QasmSource>
where
    R: SourceResolver,
{
    let mut includes = vec![];
    for stmt in &program.statements {
        if let StmtKind::Include(include) = stmt.kind.as_ref() {
            let file_path = &include.filename;
            // Skip the standard gates include file.
            // Handling of this file is done by the compiler.
            if file_path.to_lowercase() == "stdgates.inc"
                || file_path.to_lowercase() == "qiskit_stdgates.inc"
            {
                continue;
            }
            let source = parse_qasm_file(file_path, resolver);
            includes.push(source);
        }
    }

    includes
}

pub(crate) type Result<T> = std::result::Result<T, crate::parser::error::Error>;

pub(crate) trait Parser<T>: FnMut(&mut ParserContext) -> Result<T> {}

impl<T, F: FnMut(&mut ParserContext) -> Result<T>> Parser<T> for F {}

#[must_use]
pub fn parse(input: &str) -> (Program, Vec<Error>) {
    let mut scanner = ParserContext::new(input);
    let program = prgm::parse(&mut scanner);
    (program, scanner.into_errors())
}
