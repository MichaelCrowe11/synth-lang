/*!
 * SYNTH Diagnostic and Error Reporting System
 * Provides comprehensive error handling and user-friendly diagnostics
 */

use std::collections::HashMap;
use std::fmt;
use codespan_reporting::diagnostic::{Diagnostic as CodespanDiagnostic, Label, Severity};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};

/// Source file information for diagnostic reporting
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: usize,
    pub name: String,
    pub content: String,
}

/// Span information for locating errors in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn single(pos: usize) -> Self {
        Self { start: pos, end: pos + 1 }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Help,
}

impl From<DiagnosticLevel> for Severity {
    fn from(level: DiagnosticLevel) -> Self {
        match level {
            DiagnosticLevel::Error => Severity::Error,
            DiagnosticLevel::Warning => Severity::Warning,
            DiagnosticLevel::Info => Severity::Note,
            DiagnosticLevel::Help => Severity::Help,
        }
    }
}

/// A diagnostic message with location information
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub file_id: usize,
    pub span: Option<Span>,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub help: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticLabel {
    pub span: Span,
    pub message: String,
    pub level: DiagnosticLevel,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            file_id: 0,
            span: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            code: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            file_id: 0,
            span: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            code: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Info,
            message: message.into(),
            file_id: 0,
            span: None,
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            code: None,
        }
    }

    pub fn with_span(mut self, file_id: usize, span: Span) -> Self {
        self.file_id = file_id;
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(DiagnosticLabel {
            span,
            message: message.into(),
            level: self.level,
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

/// Main diagnostic engine for collecting and reporting errors
pub struct DiagnosticEngine {
    diagnostics: Vec<Diagnostic>,
    files: SimpleFiles<String, String>,
    file_map: HashMap<usize, SourceFile>,
    error_count: usize,
    warning_count: usize,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
            files: SimpleFiles::new(),
            file_map: HashMap::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Add a source file and return its ID
    pub fn add_file(&mut self, name: String, content: String) -> usize {
        let file_id = self.files.add(name.clone(), content.clone());
        self.file_map.insert(file_id, SourceFile {
            id: file_id,
            name,
            content,
        });
        file_id
    }

    /// Add a diagnostic to the collection
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        match diagnostic.level {
            DiagnosticLevel::Error => self.error_count += 1,
            DiagnosticLevel::Warning => self.warning_count += 1,
            _ => {}
        }
        self.diagnostics.push(diagnostic);
    }

    /// Emit an error diagnostic
    pub fn error(&mut self, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(self, Diagnostic::error(message))
    }

    /// Emit a warning diagnostic
    pub fn warning(&mut self, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(self, Diagnostic::warning(message))
    }

    /// Emit an info diagnostic
    pub fn info(&mut self, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(self, Diagnostic::info(message))
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Take all diagnostics, clearing the internal collection
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        let diagnostics = std::mem::take(&mut self.diagnostics);
        self.error_count = 0;
        self.warning_count = 0;
        diagnostics
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Emit all diagnostics to stderr
    pub fn emit_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let writer = StandardStream::stderr(ColorChoice::Auto);
        let config = codespan_reporting::term::Config::default();

        for diagnostic in &self.diagnostics {
            let codespan_diagnostic = self.to_codespan_diagnostic(diagnostic)?;
            term::emit(&mut writer.lock(), &config, &self.files, &codespan_diagnostic)?;
        }

        Ok(())
    }

    /// Convert internal diagnostic to codespan diagnostic
    fn to_codespan_diagnostic(&self, diagnostic: &Diagnostic) -> Result<CodespanDiagnostic<usize>, Box<dyn std::error::Error>> {
        let mut codespan_diagnostic = CodespanDiagnostic::new(diagnostic.level.into());
        
        if let Some(code) = &diagnostic.code {
            codespan_diagnostic = codespan_diagnostic.with_code(code);
        }
        
        codespan_diagnostic = codespan_diagnostic.with_message(&diagnostic.message);

        // Add primary label if span is present
        if let Some(span) = diagnostic.span {
            codespan_diagnostic = codespan_diagnostic.with_labels(vec![
                Label::primary(diagnostic.file_id, span.start..span.end)
            ]);
        }

        // Add secondary labels
        for label in &diagnostic.labels {
            codespan_diagnostic = codespan_diagnostic.with_labels(vec![
                Label::secondary(diagnostic.file_id, label.span.start..label.span.end)
                    .with_message(&label.message)
            ]);
        }

        // Add notes
        for note in &diagnostic.notes {
            codespan_diagnostic = codespan_diagnostic.with_notes(vec![note.clone()]);
        }

        // Add help
        if let Some(help) = &diagnostic.help {
            codespan_diagnostic = codespan_diagnostic.with_notes(vec![format!("help: {}", help)]);
        }

        Ok(codespan_diagnostic)
    }
    
    /// Report a diagnostic message
    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.add_diagnostic(diagnostic);
    }
    
    /// Collect all diagnostics for compilation result
    pub fn collect_diagnostics(&mut self) -> Vec<crate::Diagnostic> {
        self.diagnostics.iter().map(|d| crate::Diagnostic {
            level: match d.level {
                DiagnosticLevel::Error => crate::DiagnosticLevel::Error,
                DiagnosticLevel::Warning => crate::DiagnosticLevel::Warning,
                DiagnosticLevel::Info => crate::DiagnosticLevel::Info,
                DiagnosticLevel::Help => crate::DiagnosticLevel::Hint,
            },
            message: d.message.clone(),
            location: d.span.map(|span| crate::SourceLocation {
                file_id: d.file_id,
                span: span.start..span.end,
            }),
        }).collect()
    }
}

/// Builder pattern for constructing diagnostics
pub struct DiagnosticBuilder<'a> {
    engine: &'a mut DiagnosticEngine,
    diagnostic: Diagnostic,
}

impl<'a> DiagnosticBuilder<'a> {
    fn new(engine: &'a mut DiagnosticEngine, diagnostic: Diagnostic) -> Self {
        Self { engine, diagnostic }
    }

    pub fn with_span(mut self, file_id: usize, span: Span) -> Self {
        self.diagnostic = self.diagnostic.with_span(file_id, span);
        self
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.diagnostic = self.diagnostic.with_label(span, message);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.diagnostic = self.diagnostic.with_note(note);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.diagnostic = self.diagnostic.with_help(help);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.diagnostic = self.diagnostic.with_code(code);
        self
    }

    pub fn emit(self) {
        self.engine.add_diagnostic(self.diagnostic);
    }
}

/// Common diagnostic codes for SYNTH
pub mod codes {
    pub const PARSE_ERROR: &str = "E0001";
    pub const UNDEFINED_VARIABLE: &str = "E0002";
    pub const TYPE_MISMATCH: &str = "E0003";
    pub const INVALID_OPERATOR: &str = "E0004";
    pub const AI_API_ERROR: &str = "E0101";
    pub const SEMANTIC_SIMILARITY_ERROR: &str = "E0102";
    pub const UNCERTAINTY_ERROR: &str = "E0103";
    pub const TEMPLATE_PARSE_ERROR: &str = "E0201";
    pub const CROSS_DOMAIN_ERROR: &str = "E0301";
}

/// Common diagnostic messages
pub mod messages {
    pub fn unexpected_token(expected: &str, found: &str) -> String {
        format!("expected {}, found {}", expected, found)
    }

    pub fn undefined_variable(name: &str) -> String {
        format!("cannot find variable `{}` in this scope", name)
    }

    pub fn type_mismatch(expected: &str, found: &str) -> String {
        format!("type mismatch: expected `{}`, found `{}`", expected, found)
    }

    pub fn ai_api_error(provider: &str, error: &str) -> String {
        format!("AI API error ({}): {}", provider, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diagnostic = Diagnostic::error("test error")
            .with_span(0, Span::new(10, 15))
            .with_label(Span::new(12, 14), "here")
            .with_note("this is a note")
            .with_help("try this instead")
            .with_code(codes::PARSE_ERROR);

        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.message, "test error");
        assert_eq!(diagnostic.span, Some(Span::new(10, 15)));
        assert_eq!(diagnostic.labels.len(), 1);
        assert_eq!(diagnostic.notes.len(), 1);
        assert_eq!(diagnostic.help, Some("try this instead".to_string()));
        assert_eq!(diagnostic.code, Some(codes::PARSE_ERROR.to_string()));
    }

    #[test]
    fn test_diagnostic_engine() {
        let mut engine = DiagnosticEngine::new();
        let file_id = engine.add_file("test.synth".to_string(), "let x = 5;".to_string());

        engine.error("test error")
            .with_span(file_id, Span::new(4, 5))
            .with_code(codes::PARSE_ERROR)
            .emit();

        assert_eq!(engine.error_count(), 1);
        assert_eq!(engine.warning_count(), 0);
        assert!(engine.has_errors());

        let diagnostics = engine.take_diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(engine.error_count(), 0);
    }
}