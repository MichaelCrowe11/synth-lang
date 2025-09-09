/*!
 * SYNTH Compiler Library
 * 
 * The SYNTH compiler transforms SYNTH source code into executable code
 * for various targets including native, WASM, quantum, and AI accelerators.
 */

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod semantic;
pub mod codegen;
pub mod diagnostics;
pub mod hir;
pub mod mir;
pub mod ty;

#[cfg(feature = "ai")]
pub mod ai;

#[cfg(feature = "quantum")]
pub mod quantum;

#[cfg(feature = "zkp")]
pub mod zkp;

use std::path::Path;
use anyhow::Result;
use codespan_reporting::files::SimpleFiles;
use crate::diagnostics::DiagnosticEngine;

pub struct Compiler {
    files: SimpleFiles<String, String>,
    diagnostics: DiagnosticEngine,
    target: CompilationTarget,
    features: CompilerFeatures,
}

#[derive(Debug, Clone)]
pub enum CompilationTarget {
    Native,
    WebAssembly,
    Quantum,
    AIAccelerator,
    Embedded(EmbeddedTarget),
    Runtime, // For REPL/JIT
}

#[derive(Debug, Clone)]
pub enum EmbeddedTarget {
    ArmCortexM,
    RiscV,
    Avr,
    Xtensa,
}

#[derive(Debug, Default)]
pub struct CompilerFeatures {
    pub ai_enabled: bool,
    pub quantum_enabled: bool,
    pub semantic_enabled: bool,
    pub zkp_enabled: bool,
    pub optimizations: bool,
}

#[derive(Debug)]
pub struct CompilationResult {
    pub bytecode: Vec<u8>,
    pub metadata: CompilationMetadata,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct CompilationMetadata {
    pub target: CompilationTarget,
    pub features_used: Vec<String>,
    pub entry_points: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub location: Option<SourceLocation>,
}

#[derive(Debug)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug)]
pub struct SourceLocation {
    pub file_id: usize,
    pub span: std::ops::Range<usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            files: SimpleFiles::new(),
            diagnostics: DiagnosticEngine::new(),
            target: CompilationTarget::Native,
            features: CompilerFeatures::default(),
        }
    }

    pub fn set_target(&mut self, target: &str) -> Result<()> {
        self.target = match target {
            "native" => CompilationTarget::Native,
            "wasm" | "wasm32" => CompilationTarget::WebAssembly,
            "quantum" => CompilationTarget::Quantum,
            "ai" => CompilationTarget::AIAccelerator,
            "runtime" => CompilationTarget::Runtime,
            "arm-cortex-m" => CompilationTarget::Embedded(EmbeddedTarget::ArmCortexM),
            "riscv" => CompilationTarget::Embedded(EmbeddedTarget::RiscV),
            _ => anyhow::bail!("Unsupported target: {}", target),
        };
        Ok(())
    }

    pub fn enable_ai_features(&mut self) {
        self.features.ai_enabled = true;
    }

    pub fn enable_quantum_features(&mut self) {
        self.features.quantum_enabled = true;
    }

    pub fn enable_semantic_features(&mut self) {
        self.features.semantic_enabled = true;
    }

    pub fn enable_zkp_features(&mut self) {
        self.features.zkp_enabled = true;
    }

    pub fn enable_optimizations(&mut self) {
        self.features.optimizations = true;
    }

    pub async fn compile_file<P: AsRef<Path>>(&mut self, path: P) -> Result<CompilationResult> {
        let path = path.as_ref();
        let source = tokio::fs::read_to_string(path).await?;
        let file_id = self.files.add(path.display().to_string(), source.clone());
        
        self.compile_source(file_id, &source).await
    }

    pub async fn compile_source(&mut self, file_id: usize, source: &str) -> Result<CompilationResult> {
        // Phase 1: Lexical analysis
        let tokens = self.tokenize(file_id, source)?;
        
        // Phase 2: Parsing
        let ast = self.parse(tokens)?;
        
        // Phase 3: Semantic analysis (AST -> HIR)
        let hir = self.analyze_semantics(ast).await?;
        
        // Phase 4: Type checking
        let typed_hir = self.type_check(hir)?;
        
        // Phase 5: Lower to MIR
        let mir = self.lower_to_mir(typed_hir)?;
        
        // Phase 6: Optimization
        let optimized_mir = self.optimize(mir)?;
        
        // Phase 7: Code generation (for now, still use AST for backward compatibility)
        // In a full implementation, we'd generate from MIR
        let bytecode = self.generate_code_from_mir(optimized_mir).await?;
        
        // Collect diagnostics
        let diagnostics = self.diagnostics.collect_diagnostics();
        
        // Create compilation result
        let metadata = CompilationMetadata {
            target: self.target.clone(),
            features_used: self.get_used_features(),
            entry_points: vec!["main".to_string()],
            dependencies: vec![],
        };
        
        Ok(CompilationResult {
            bytecode,
            metadata,
            diagnostics,
        })
    }
    
    /// Alternative compilation method for quick compilation (bypassing full pipeline)
    pub async fn compile_source_quick(&mut self, file_id: usize, source: &str) -> Result<CompilationResult> {
        // Lexical analysis
        let tokens = self.tokenize(file_id, source)?;
        
        // Parsing
        let ast = self.parse(tokens)?;
        
        // Direct code generation (skip semantic analysis for speed)
        let bytecode = self.generate_code(ast).await?;
        
        // Create compilation result
        let metadata = CompilationMetadata {
            target: self.target.clone(),
            features_used: self.get_used_features(),
            entry_points: vec!["main".to_string()],
            dependencies: vec![],
        };
        
        Ok(CompilationResult {
            bytecode,
            metadata,
            diagnostics: vec![],
        })
    }

    fn tokenize(&mut self, file_id: usize, source: &str) -> Result<Vec<token::Token>> {
        lexer::tokenize(source, file_id)
    }

    fn parse(&mut self, tokens: Vec<token::Token>) -> Result<ast::Program> {
        parser::Parser::parse(tokens)
    }

    async fn analyze_semantics(&mut self, ast: ast::Program) -> Result<hir::Program> {
        let mut analyzer = semantic::SemanticAnalyzer::new(&mut self.diagnostics);
        
        #[cfg(feature = "ai")]
        if self.features.ai_enabled {
            analyzer.enable_ai_analysis();
        }
        
        #[cfg(feature = "semantic")]
        if self.features.semantic_enabled {
            analyzer.enable_semantic_analysis();
        }
        
        analyzer.analyze(ast).await
    }

    fn type_check(&mut self, hir: hir::Program) -> Result<hir::Program> {
        let mut type_checker = ty::TypeChecker::new(&mut self.diagnostics);
        type_checker.check(hir)
    }

    fn lower_to_mir(&mut self, hir: hir::Program) -> Result<mir::Program> {
        let mut lowering = mir::Lowering::new();
        lowering.lower(hir)
    }

    fn optimize(&mut self, mir: mir::Program) -> Result<mir::Program> {
        let mut optimizer = mir::Optimizer::new();
        
        if self.features.ai_enabled {
            optimizer.enable_ai_optimizations();
        }
        
        optimizer.optimize(mir)
    }

    async fn generate_code(&mut self, program: ast::Program) -> Result<Vec<u8>> {
        let target = match self.target {
            CompilationTarget::Native => codegen::CompilationTarget::Native,
            CompilationTarget::WebAssembly => codegen::CompilationTarget::WebAssembly,
            CompilationTarget::Quantum => codegen::CompilationTarget::Quantum,
            CompilationTarget::Runtime => codegen::CompilationTarget::JavaScript,
            _ => codegen::CompilationTarget::JavaScript,
        };
        
        let mut codegen = codegen::CodeGenerator::new(target);
        
        #[cfg(feature = "quantum")]
        if self.features.quantum_enabled {
            codegen.enable_quantum_support();
        }
        
        #[cfg(feature = "zkp")]
        if self.features.zkp_enabled {
            codegen.enable_zkp_support();
        }
        
        codegen.generate(&program).await
    }
    
    async fn generate_code_from_mir(&mut self, mir: mir::Program) -> Result<Vec<u8>> {
        // For now, we'll generate JavaScript from MIR
        // In a full implementation, this would handle all targets
        let js_code = self.generate_js_from_mir(mir)?;
        Ok(js_code.into_bytes())
    }
    
    fn generate_js_from_mir(&self, mir: mir::Program) -> Result<String> {
        // Simple MIR to JavaScript generation
        // This is a placeholder - a real implementation would be more sophisticated
        let mut output = String::new();
        
        // Generate functions
        for func in mir.functions {
            output.push_str(&format!("function {}() {{\n", func.name));
            
            // Generate basic blocks
            for (_, block) in func.blocks {
                for inst in block.instructions {
                    match inst {
                        mir::Instruction::Assign { dest: _, value: _ } => {
                            output.push_str("  // Assignment\n");
                        }
                        mir::Instruction::Call { .. } => {
                            output.push_str("  // Function call\n");
                        }
                        mir::Instruction::AiOp { .. } => {
                            output.push_str("  // AI operation\n");
                        }
                        _ => {}
                    }
                }
            }
            
            output.push_str("}\n\n");
        }
        
        // Add AI runtime if needed
        if mir.metadata.ai_operations_used {
            output.push_str(r#"
// SYNTH AI Runtime
const synthAI = {
    async generate(prompt) {
        console.log('AI Generate:', prompt);
        return `AI Response: ${prompt}`;
    },
    async embed(text) {
        console.log('Embed:', text);
        return new Array(768).fill(0).map(() => Math.random());
    }
};
"#);
        }
        
        Ok(output)
    }

    fn get_used_features(&self) -> Vec<String> {
        let mut features = Vec::new();
        
        if self.features.ai_enabled {
            features.push("ai".to_string());
        }
        if self.features.quantum_enabled {
            features.push("quantum".to_string());
        }
        if self.features.semantic_enabled {
            features.push("semantic".to_string());
        }
        if self.features.zkp_enabled {
            features.push("zkp".to_string());
        }
        
        features
    }
}

impl CompilationResult {
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        std::fs::write(path, &self.bytecode)?;
        Ok(())
    }
    
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter()
            .any(|d| matches!(d.level, DiagnosticLevel::Error))
    }
    
    pub fn print_diagnostics(&self) {
        for diagnostic in &self.diagnostics {
            match diagnostic.level {
                DiagnosticLevel::Error => eprintln!("Error: {}", diagnostic.message),
                DiagnosticLevel::Warning => eprintln!("Warning: {}", diagnostic.message),
                DiagnosticLevel::Info => println!("Info: {}", diagnostic.message),
                DiagnosticLevel::Hint => println!("Hint: {}", diagnostic.message),
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}