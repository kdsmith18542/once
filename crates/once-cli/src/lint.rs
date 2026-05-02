//! Linter for Once source code.

use once_hir::{HirProgram, HirItem};
use std::collections::HashSet;

pub struct LintWarning {
    pub message: String,
    pub line: usize,
    pub kind: LintKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LintKind {
    UnusedImport,
    UnusedVariable,
    DeadCode,
    StyleIssue,
}

pub fn lint(hir: &HirProgram) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    
    check_style_issues(hir, &mut warnings);
    
    warnings
}

fn check_style_issues(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    for item in &hir.items {
        if let HirItem::FnDecl(f) = item {
            if f.return_type.is_none() && !f.body.statements.is_empty() {
                if let Some(stmt) = f.body.statements.last() {
                    if !is_unit_expression(stmt) {
                        warnings.push(LintWarning {
                            message: format!("Function '{}' has no return type annotation", f.name),
                            line: f.span.map(|s| s.0).unwrap_or(0),
                            kind: LintKind::StyleIssue,
                        });
                    }
                }
            }
        }
    }
}

fn is_unit_expression(stmt: &once_hir::HirStmt) -> bool {
    match stmt {
        once_hir::HirStmt::Let(_) => true,
        once_hir::HirStmt::Using(_) => true,
        once_hir::HirStmt::Continue | once_hir::HirStmt::Break => true,
        once_hir::HirStmt::Return(r) => r.value.is_none(),
        once_hir::HirStmt::Expr(e) => matches!(e, once_hir::HirExpr::Literal(once_hir::HirLiteral::Unit)),
    }
}