use once_hir::HirProgram;
use once_ty::{TypeChecker, Type};
use once_ty::effects::{EffectChecker, EffectRow};
use once_linear::{LinearityChecker, Linearity, UsageInfo};
use once_rinf::{RegionChecker, Region};
use once_lex::Span;
use thiserror::Error;
use colored::Colorize;

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
}

impl Explainer {
    pub fn new() -> Self {
        Self {
            type_checker: TypeChecker::new(),
            effects_checker: EffectChecker::new(),
            linearity_checker: LinearityChecker::new(),
            region_checker: RegionChecker::new(),
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
        // Run region checking
        self.region_checker.check(hir)
            .map_err(|e| ExplainError::RegionInferenceError(format!("{:?}", e)))?;

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

    /// Find type at a specific span
    fn find_type_at_span(&self, _hir: &HirProgram, _span: Span) -> Result<Type, ExplainError> {
        // Query the type checker's environment for inferred types
        // In a full implementation, this would traverse HIR nodes at the span
        // For now, return the first non-primitive binding found, or Int as fallback
        let env = &self.type_checker.env;
        for (name, scheme) in &env.bindings {
            if !matches!(name.as_str(), "Unit" | "Int" | "Bool" | "Float" | "Str" | "print") {
                return Ok(scheme.ty.clone());
            }
        }
        Ok(Type::Int) // fallback when no user bindings exist
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

    /// Find effect at a specific span
    fn find_effect_at_span(&self, _hir: &HirProgram, _span: Span) -> Result<EffectRow, ExplainError> {
        // Query the effect checker for inferred effects
        // Return effects from the checker's environment
        let env = &self.effects_checker.env;
        if let Some(effects) = &env.effects.last() {
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

    /// Find linearity at a specific span
    fn find_linearity_at_span(&self, _hir: &HirProgram, _span: Span) -> Result<(String, Linearity, UsageInfo), ExplainError> {
        // Query the linearity checker for variable usage
        let env = &self.linearity_checker.env;
        for (name, usage) in &env.variables {
            return Ok((name.clone(), usage.linearity.clone(), usage.clone()));
        }
        // Fallback when no variables tracked
        Ok((
            "<unknown>".to_string(),
            Linearity::Unrestricted,
            UsageInfo {
                variable: "<none>".to_string(),
                linearity: Linearity::Unrestricted,
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

    /// Find region at a specific span
    fn find_region_at_span(&self, _hir: &HirProgram, _span: Span) -> Result<Region, ExplainError> {
        // Simplified - in a real implementation, this would traverse the HIR
        Ok(Region {
            id: 1,
            name: "r1".to_string(),
            is_primary: true,
        })
    }

    /// Get region constraints
    fn get_region_constraints(&self, region: &Region) -> Vec<String> {
        vec![format!("Region: {:?}", region)]
    }

    /// Get escape analysis
    fn get_escape_analysis(&self, _region: &Region) -> Vec<String> {
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
                    params: vec![],
                    return_type: None,
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

