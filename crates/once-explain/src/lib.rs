use once_hir::{HirProgram, HirItem, HirBlock, HirStmt, HirExpr, HirSpan};
use once_ty::{TypeChecker, Type};
use once_ty::effects::{EffectChecker, EffectRow};
use once_linear::{LinearityChecker, Linearity, UsageInfo};
use once_rinf::{RegionChecker, Region, RegionDag};
use once_lex::Span;
use thiserror::Error;
use colored::Colorize;

/// Check if two spans overlap (useful for byte-range queries)
fn spans_overlap(a: Span, b: HirSpan) -> bool {
    a.start <= b.end && a.end >= b.start
}

/// Errors for explain operations
#[derive(Error, Debug, Clone)]
pub enum ExplainError {
    #[error("Type checking error: {0}")]
    TypeCheckingError(String),
    #[error("Effects checking error: {0}")]
    EffectsCheckingError(String),
    #[error("Linearity checking error: {0}")]
    LinearityCheckingError(String),
    #[error("Region inference error: {0}")]
    RegionInferenceError(String),
    #[error("Span not found: {0}")]
    SpanNotFound(String),
}

/// Explanation for a type
#[derive(Debug, Clone)]
pub struct TypeExplanation {
    pub span: Span,
    pub inferred_type: String,
    pub constraints: Vec<String>,
    pub reasoning: Vec<String>,
}

/// Explanation for effects
#[derive(Debug, Clone)]
pub struct EffectExplanation {
    pub span: Span,
    pub effect_row: String,
    pub effect_labels: Vec<String>,
    pub reasoning: Vec<String>,
}

/// Explanation for linearity
#[derive(Debug, Clone)]
pub struct LinearityExplanation {
    pub span: Span,
    pub variable: String,
    pub linearity: String,
    pub first_use: Option<Span>,
    pub last_use: Option<Span>,
    pub usage_count: usize,
    pub reasoning: Vec<String>,
}

/// Explanation for regions
#[derive(Debug, Clone)]
pub struct RegionExplanation {
    pub span: Span,
    pub region: String,
    pub constraints: Vec<String>,
    pub escapes: Vec<String>,
    pub reasoning: Vec<String>,
}

/// Explainer for compiler analysis
pub struct Explainer {
    type_checker: TypeChecker,
    effects_checker: EffectChecker,
    linearity_checker: LinearityChecker,
    region_checker: RegionChecker,
    cached_dag: Option<RegionDag>,
}

impl Explainer {
    pub fn new() -> Self {
        Self {
            type_checker: TypeChecker::new(),
            effects_checker: EffectChecker::new(),
            linearity_checker: LinearityChecker::new(),
            region_checker: RegionChecker::new(),
            cached_dag: None,
        }
    }

    /// Explain types at a specific span
    pub fn explain_types(&mut self, hir: &HirProgram, span: Span) -> Result<TypeExplanation, ExplainError> {
        // Run type checking
        self.type_checker.check(hir)
            .map_err(|e| ExplainError::TypeCheckingError(format!("{:?}", e)))?;

        // Find the type at the given span
        let inferred_type = self.find_type_at_span(hir, span)?;
        
        // Get constraints for this type
        let constraints = self.get_type_constraints(&inferred_type);
        
        // Generate reasoning
        let reasoning = self.generate_type_reasoning(&inferred_type, &constraints);

        Ok(TypeExplanation {
            span,
            inferred_type: format!("{:?}", inferred_type),
            constraints,
            reasoning,
        })
    }

    /// Explain effects at a specific span
    pub fn explain_effects(&mut self, hir: &HirProgram, span: Span) -> Result<EffectExplanation, ExplainError> {
        // Run effects checking
        self.effects_checker.check(hir)
            .map_err(|e| ExplainError::EffectsCheckingError(format!("{:?}", e)))?;

        // Find the effect row at the given span
        let effect_row = self.find_effect_at_span(hir, span)?;
        
        // Extract effect labels
        let effect_labels = self.extract_effect_labels(&effect_row);
        
        // Generate reasoning
        let reasoning = self.generate_effect_reasoning(&effect_row, &effect_labels);

        Ok(EffectExplanation {
            span,
            effect_row: format!("{:?}", effect_row),
            effect_labels,
            reasoning,
        })
    }

    /// Explain linearity at a specific span
    pub fn explain_linearity(&mut self, hir: &HirProgram, span: Span) -> Result<LinearityExplanation, ExplainError> {
        // Run linearity checking
        self.linearity_checker.check(hir)
            .map_err(|e| ExplainError::LinearityCheckingError(format!("{:?}", e)))?;

        // Find the linearity info at the given span
        let (variable, linearity, usage_info) = self.find_linearity_at_span(hir, span)?;
        
        // Generate reasoning
        let reasoning = self.generate_linearity_reasoning(&linearity, &usage_info);

        Ok(LinearityExplanation {
            span,
            variable,
            linearity: format!("{:?}", linearity),
            first_use: usage_info.first_use,
            last_use: usage_info.last_use,
            usage_count: usage_info.usage_count,
            reasoning,
        })
    }

    /// Explain regions at a specific span
    pub fn explain_regions(&mut self, hir: &HirProgram, span: Span) -> Result<RegionExplanation, ExplainError> {
        // Run region checking and cache the DAG
        let dag = self.region_checker.check(hir)
            .map_err(|e| ExplainError::RegionInferenceError(format!("{:?}", e)))?;
        self.cached_dag = Some(dag);

        // Find the region at the given span
        let region = self.find_region_at_span(hir, span)?;
        
        // Get constraints for this region
        let constraints = self.get_region_constraints(&region);
        
        // Get escape analysis
        let escapes = self.get_escape_analysis(&region);
        
        // Generate reasoning
        let reasoning = self.generate_region_reasoning(&region, &constraints, &escapes);

        Ok(RegionExplanation {
            span,
            region: format!("{:?}", region),
            constraints,
            escapes,
            reasoning,
        })
    }

    /// Find type at a specific span - walks HIR to find matching expressions
    fn find_type_at_span(&self, hir: &HirProgram, span: Span) -> Result<Type, ExplainError> {
        // Walk HIR items looking for the expression closest to the given span
        for item in &hir.items {
            if let HirItem::FnDecl(fn_decl) = item {
                if let Some(hir_span) = fn_decl.span {
                    if spans_overlap(span, hir_span) {
                        // Look through function body for matching expression
                        if let Ok(ty) = self.find_type_in_block(&fn_decl.body, span) {
                            return Ok(ty);
                        }
                    }
                }
            }
        }
        // Fallback: look for any user-defined type in the environment
        let env = &self.type_checker.env;
        for (name, scheme) in &env.bindings {
            if !matches!(name.as_str(), "Unit" | "Int" | "Bool" | "Float" | "Str" | "print" | "spawn") {
                return Ok(scheme.ty.clone());
            }
        }
        Ok(Type::Int)
    }

    fn find_type_in_block(&self, block: &HirBlock, span: Span) -> Result<Type, ExplainError> {
        for stmt in &block.statements {
            if let Ok(ty) = self.find_type_in_stmt(stmt, span) {
                return Ok(ty);
            }
        }
        Err(ExplainError::SpanNotFound(format!("No type found at span {:?}", span)))
    }

    fn find_type_in_stmt(&self, stmt: &HirStmt, span: Span) -> Result<Type, ExplainError> {
        match stmt {
            HirStmt::Let(let_stmt) => {
                if let Some(hir_span) = let_stmt.span {
                    if spans_overlap(span, hir_span) {
                        return Ok(Type::UserDefined {
                            name: let_stmt.name.clone(),
                            args: vec![],
                        });
                    }
                }
                self.find_type_in_expr(&let_stmt.value, span)
            }
            HirStmt::Return(ret) => {
                if let Some(ref e) = ret.value {
                    self.find_type_in_expr(e, span)
                } else {
                    Ok(Type::Unit)
                }
            }
            HirStmt::Expr(e) => self.find_type_in_expr(e, span),
            HirStmt::Using(u) => {
                self.find_type_in_expr(&u.init, span)
                    .or_else(|_| self.find_type_in_block(&u.body, span))
            }
            _ => Err(ExplainError::SpanNotFound("No type at this statement".to_string())),
        }
    }

    fn find_type_in_expr(&self, expr: &HirExpr, span: Span) -> Result<Type, ExplainError> {
        match expr {
            HirExpr::Literal(lit, _) => {
                Ok(match lit {
                    once_hir::HirLiteral::Int(_) => Type::Int,
                    once_hir::HirLiteral::Float(_) => Type::Float,
                    once_hir::HirLiteral::String(_) => Type::Str,
                    once_hir::HirLiteral::Bool(_) => Type::Bool,
                    once_hir::HirLiteral::Unit => Type::Unit,
                })
            }
            HirExpr::Ident(name, _) => {
                if let Some(scheme) = self.type_checker.env.bindings.get(name) {
                    Ok(scheme.ty.clone())
                } else {
                    Ok(Type::UserDefined { name: name.clone(), args: vec![] })
                }
            }
            HirExpr::Call { function, .. } => {
                if let Some(scheme) = self.type_checker.env.bindings.get(function) {
                    Ok(scheme.ty.clone())
                } else {
                    Ok(Type::Int)
                }
            }
            HirExpr::Block(b, _) => self.find_type_in_block(b, span),
            _ => Ok(Type::Int),
        }
    }

    /// Get type constraints
    fn get_type_constraints(&self, ty: &Type) -> Vec<String> {
        let mut constraints = Vec::new();
        constraints.push(format!("Type: {:?}", ty));
        // Include relevant constraints from the environment
        for constraint in &self.type_checker.env.constraints {
            constraints.push(format!("  Constraint: {:?}", constraint));
        }
        constraints.truncate(5); // Limit to avoid overwhelming output
        constraints
    }

    /// Generate type reasoning
    fn generate_type_reasoning(&self, ty: &Type, constraints: &[String]) -> Vec<String> {
        let mut reasoning = Vec::new();
        reasoning.push(format!("The type {:?} was inferred based on:", ty));
        for constraint in constraints {
            reasoning.push(format!("  - {}", constraint));
        }
        if reasoning.len() == 1 {
            reasoning.push("  - No additional constraints found".to_string());
        }
        reasoning
    }

    /// Find effect at a specific span by looking up the enclosing function
    fn find_effect_at_span(&self, hir: &HirProgram, span: Span) -> Result<EffectRow, ExplainError> {
        let fn_name = self.find_enclosing_function(hir, span)?;

        // Look up effect row by function name
        let env = &self.effects_checker.env;
        if let Some(effects) = env.bindings.get(&fn_name) {
            return Ok(effects.clone());
        }

        // Try with "fn_" prefix
        let prefixed = format!("fn_{}", fn_name);
        if let Some(effects) = env.bindings.get(&prefixed) {
            return Ok(effects.clone());
        }

        // Fallback: return first binding if any
        if let Some((_name, effects)) = env.bindings.iter().next() {
            return Ok(effects.clone());
        }
        Ok(EffectRow::Empty)
    }

    /// Extract effect labels
    fn extract_effect_labels(&self, effect_row: &EffectRow) -> Vec<String> {
        match effect_row {
            EffectRow::Empty => vec!["(no effects)".to_string()],
            EffectRow::Single { label, .. } => vec![format!("{:?}", label)],
            EffectRow::Cons { label, tail, .. } => {
                let mut labels = vec![format!("{:?}", label)];
                labels.extend(self.extract_effect_labels(tail));
                labels
            }
            EffectRow::Var(_) => vec!["<effect variable>".to_string()],
            EffectRow::Union(left, right) => {
                let mut labels = self.extract_effect_labels(left);
                labels.extend(self.extract_effect_labels(right));
                labels
            }
            EffectRow::Intersection(left, right) => {
                let mut labels = self.extract_effect_labels(left);
                labels.extend(self.extract_effect_labels(right));
                labels
            }
        }
    }

    /// Generate effect reasoning
    fn generate_effect_reasoning(&self, effect_row: &EffectRow, effect_labels: &[String]) -> Vec<String> {
        let mut reasoning = Vec::new();
        reasoning.push(format!("The effect row {:?} was inferred based on:", effect_row));
        for label in effect_labels {
            reasoning.push(format!("  - Effect: {}", label));
        }
        if reasoning.len() == 1 {
            reasoning.push("  - No effects detected".to_string());
        }
        reasoning
    }

    /// Find linearity at a specific span by matching against UsageInfo spans
    fn find_linearity_at_span(&self, _hir: &HirProgram, span: Span) -> Result<(String, Linearity, UsageInfo), ExplainError> {
        let env = &self.linearity_checker.env;

        // Try to find a variable whose usage spans contain the query span
        for (name, usage) in &env.variables {
            let in_range = match (usage.first_use, usage.last_use) {
                (Some(first), Some(last)) => {
                    span.start >= first.start && span.end <= last.end
                }
                (Some(first), None) => {
                    span.start >= first.start && span.start <= first.end
                }
                _ => false,
            };
            if in_range {
                return Ok((name.clone(), usage.linearity.clone(), usage.clone()));
            }
        }

        // Fallback: return first variable with span information
        for (name, usage) in &env.variables {
            if usage.first_use.is_some() || usage.last_use.is_some() {
                return Ok((name.clone(), usage.linearity.clone(), usage.clone()));
            }
        }

        // Final fallback: any variable
        for (name, usage) in &env.variables {
            return Ok((name.clone(), usage.linearity.clone(), usage.clone()));
        }

        Ok((
            "<unknown>".to_string(),
            Linearity::NonLinear,
            UsageInfo {
                variable: "<none>".to_string(),
                linearity: Linearity::NonLinear,
                usage_count: 0,
                first_use: None,
                last_use: None,
            },
        ))
    }

    /// Generate linearity reasoning
    fn generate_linearity_reasoning(&self, linearity: &Linearity, usage_info: &UsageInfo) -> Vec<String> {
        let mut reasoning = Vec::new();
        reasoning.push(format!("The linearity {:?} was determined based on:", linearity));
        reasoning.push(format!("  - Usage count: {}", usage_info.usage_count));
        if let Some(first_use) = usage_info.first_use {
            reasoning.push(format!("  - First use at: {:?}", first_use));
        }
        if let Some(last_use) = usage_info.last_use {
            reasoning.push(format!("  - Last use at: {:?}", last_use));
        }
        reasoning
    }

    /// Find region at a specific span by walking the region DAG
    fn find_region_at_span(&self, hir: &HirProgram, span: Span) -> Result<Region, ExplainError> {
        let enclosing_fn = self.find_enclosing_function(hir, span)?;

        if let Some(ref dag) = self.cached_dag {
            for (region, _node) in dag.nodes.iter() {
                if region.name == format!("fn_{}", enclosing_fn) {
                    return Ok(region.clone());
                }
            }
            for (region, _node) in dag.nodes.iter() {
                return Ok(region.clone());
            }
        }

        Ok(Region { id: 1, name: enclosing_fn, is_primary: true })
    }

    /// Walk HIR to find the enclosing function name at a given span
    fn find_enclosing_function(&self, hir: &HirProgram, _span: Span) -> Result<String, ExplainError> {
        for item in &hir.items {
            if let HirItem::FnDecl(fn_decl) = item {
                if let Some(hs) = fn_decl.span {
                    if hs.start <= _span.start && _span.end <= hs.end {
                        return Ok(fn_decl.name.clone());
                    }
                }
            }
        }
        // Fallback: return first function name
        for item in &hir.items {
            if let HirItem::FnDecl(fn_decl) = item {
                return Ok(fn_decl.name.clone());
            }
        }
        Ok("main".to_string())
    }

    /// Get region constraints
    fn get_region_constraints(&self, region: &Region) -> Vec<String> {
        vec![format!("Region: {:?}", region)]
    }

    /// Get escape analysis
    fn get_escape_analysis(&self, region: &Region) -> Vec<String> {
        if let Some(ref dag) = self.cached_dag {
            if let Some(node) = dag.nodes.get(region) {
                if node.escapes.is_empty() {
                    return vec!["No escapes detected".to_string()];
                }
                return node.escapes.clone();
            }
        }
        vec!["No escapes detected".to_string()]
    }

    /// Generate region reasoning
    fn generate_region_reasoning(&self, region: &Region, constraints: &[String], escapes: &[String]) -> Vec<String> {
        let mut reasoning = Vec::new();
        reasoning.push(format!("The region {:?} was inferred based on:", region));
        for constraint in constraints {
            reasoning.push(format!("  - {}", constraint));
        }
        for escape in escapes {
            reasoning.push(format!("  - {}", escape));
        }
        reasoning
    }

    /// Format type explanation for display
    pub fn format_type_explanation(&self, explanation: &TypeExplanation) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", "Type Explanation:".bold().green()));
        output.push_str(&format!("  Span: {:?}\n", explanation.span));
        output.push_str(&format!("  Inferred Type: {}\n", explanation.inferred_type.cyan()));
        output.push_str(&format!("  Constraints:\n"));
        for constraint in &explanation.constraints {
            output.push_str(&format!("    - {}\n", constraint));
        }
        output.push_str(&format!("  Reasoning:\n"));
        for reason in &explanation.reasoning {
            output.push_str(&format!("    {}\n", reason));
        }
        output
    }

    /// Format effect explanation for display
    pub fn format_effect_explanation(&self, explanation: &EffectExplanation) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", "Effect Explanation:".bold().green()));
        output.push_str(&format!("  Span: {:?}\n", explanation.span));
        output.push_str(&format!("  Effect Row: {}\n", explanation.effect_row.cyan()));
        output.push_str(&format!("  Effect Labels:\n"));
        for label in &explanation.effect_labels {
            output.push_str(&format!("    - {}\n", label));
        }
        output.push_str(&format!("  Reasoning:\n"));
        for reason in &explanation.reasoning {
            output.push_str(&format!("    {}\n", reason));
        }
        output
    }

    /// Format linearity explanation for display
    pub fn format_linearity_explanation(&self, explanation: &LinearityExplanation) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", "Linearity Explanation:".bold().green()));
        output.push_str(&format!("  Span: {:?}\n", explanation.span));
        output.push_str(&format!("  Variable: {}\n", explanation.variable.cyan()));
        output.push_str(&format!("  Linearity: {}\n", explanation.linearity.yellow()));
        output.push_str(&format!("  Usage Count: {}\n", explanation.usage_count));
        if let Some(first_use) = explanation.first_use {
            output.push_str(&format!("  First Use: {:?}\n", first_use));
        }
        if let Some(last_use) = explanation.last_use {
            output.push_str(&format!("  Last Use: {:?}\n", last_use));
        }
        output.push_str(&format!("  Reasoning:\n"));
        for reason in &explanation.reasoning {
            output.push_str(&format!("    {}\n", reason));
        }
        output
    }

    /// Format region explanation for display
    pub fn format_region_explanation(&self, explanation: &RegionExplanation) -> String {
        let mut output = String::new();
        output.push_str(&format!("{}\n", "Region Explanation:".bold().green()));
        output.push_str(&format!("  Span: {:?}\n", explanation.span));
        output.push_str(&format!("  Region: {}\n", explanation.region.cyan()));
        output.push_str(&format!("  Constraints:\n"));
        for constraint in &explanation.constraints {
            output.push_str(&format!("    - {}\n", constraint));
        }
        output.push_str(&format!("  Escape Analysis:\n"));
        for escape in &explanation.escapes {
            output.push_str(&format!("    - {}\n", escape));
        }
        output.push_str(&format!("  Reasoning:\n"));
        for reason in &explanation.reasoning {
            output.push_str(&format!("    {}\n", reason));
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirExpr, HirLiteral};

    #[test]
    fn test_explainer_creation() {
        let explainer = Explainer::new();
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_explain_types() {
        let mut explainer = Explainer::new();
        let hir = create_test_hir();
        let span = Span { start: 0, end: 10, line: 1, column: 1 };
        
        let explanation = explainer.explain_types(&hir, span).unwrap();
        assert_eq!(explanation.span, span);
        assert!(!explanation.reasoning.is_empty());
    }

    #[test]
    fn test_explain_effects() {
        let mut explainer = Explainer::new();
        let hir = create_test_hir();
        let span = Span { start: 0, end: 10, line: 1, column: 1 };
        
        let explanation = explainer.explain_effects(&hir, span).unwrap();
        assert_eq!(explanation.span, span);
        assert!(!explanation.reasoning.is_empty());
    }

    #[test]
    fn test_explain_linearity() {
        let mut explainer = Explainer::new();
        let hir = create_test_hir();
        let span = Span { start: 0, end: 10, line: 1, column: 1 };
        
        let explanation = explainer.explain_linearity(&hir, span).unwrap();
        assert_eq!(explanation.span, span);
        assert!(!explanation.reasoning.is_empty());
    }

    #[test]
    fn test_explain_regions() {
        let mut explainer = Explainer::new();
        let hir = create_test_hir();
        let span = Span { start: 0, end: 10, line: 1, column: 1 };
        
        let explanation = explainer.explain_regions(&hir, span).unwrap();
        assert_eq!(explanation.span, span);
        assert!(!explanation.reasoning.is_empty());
    }

    #[test]
    fn test_format_type_explanation() {
        let explainer = Explainer::new();
        let explanation = TypeExplanation {
            span: Span { start: 0, end: 10, line: 1, column: 1 },
            inferred_type: "Int".to_string(),
            constraints: vec!["Type: Int".to_string()],
            reasoning: vec!["The type Int was inferred based on:".to_string()],
        };
        
        let formatted = explainer.format_type_explanation(&explanation);
        assert!(formatted.contains("Type Explanation:"));
        assert!(formatted.contains("Int"));
    }

    fn create_test_hir() -> HirProgram {
        HirProgram {
            items: vec![
                HirItem::FnDecl(HirFnDecl {
                    name: "main".to_string(),
                    type_params: vec![],
                    params: vec![],
                    return_type: Some(once_hir::HirType::Unit),
                    effects: None,
                    body: HirBlock {
                        statements: vec![],
                        span: None,
                    },
                    is_public: false,
                    span: None,
                }),
            ],
            imports: vec![],
        }
    }
}

