//! Linter for Once source code.
//!
//! Detects:
//! - Dead code (unused functions, unused let bindings)
//! - Unused imports
//! - Unused variables
//! - Style issues (missing return type, naming conventions)
//! - Linear-resource leaks

use once_hir::{HirProgram, HirItem, HirFnDecl, HirStmt, HirExpr, HirBlock};
use once_ty::effects;
use std::collections::{HashSet, HashMap};

pub struct LintWarning {
    pub message: String,
    pub line: usize,
    pub kind: LintKind,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LintKind {
    UnusedImport,
    UnusedVariable,
    DeadCode,
    StyleIssue,
    LinearResourceLeak,
    CapabilityViolation,
    BoxRcWarning,
    UnusedEffect,
}

pub fn lint(hir: &HirProgram) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    check_dead_code(hir, &mut warnings);
    check_style_issues(hir, &mut warnings);
    check_unused_imports(hir, &mut warnings);
    check_unused_variables(hir, &mut warnings);
    check_linear_resource_leaks(hir, &mut warnings);
    check_box_rc_usage(hir, &mut warnings);
    check_unused_effects(hir, &mut warnings);

    warnings
}

/// Detect dead code: public items without callers and local-lets that shadow nothing.
fn check_dead_code(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    // Collect all function calls to determine which functions are used.
    let mut called_functions: HashSet<String> = HashSet::new();

    fn collect_calls(block: &HirBlock, called: &mut HashSet<String>) {
        for stmt in &block.statements {
            collect_calls_in_stmt(stmt, called);
        }
    }

    fn collect_calls_in_stmt(stmt: &HirStmt, called: &mut HashSet<String>) {
        match stmt {
            HirStmt::Let(l) => collect_calls_in_expr(&l.value, called),
            HirStmt::Return(r) => {
                if let Some(ref e) = r.value {
                    collect_calls_in_expr(e, called);
                }
            }
            HirStmt::Expr(e) => collect_calls_in_expr(e, called),
            HirStmt::Using(u) => {
                collect_calls_in_expr(&u.init, called);
                collect_calls(&u.body, called);
            }
            _ => {}
        }
    }

    fn collect_calls_in_expr(expr: &HirExpr, called: &mut HashSet<String>) {
        match expr {
            HirExpr::Call { function, args, .. } => {
                called.insert(function.clone());
                for a in args {
                    collect_calls_in_expr(a, called);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                collect_calls_in_expr(left, called);
                collect_calls_in_expr(right, called);
            }
            HirExpr::Block(b, _) => collect_calls(b, called),
            HirExpr::If { condition, then_branch, else_branch, .. } => {
                collect_calls_in_expr(condition, called);
                collect_calls(then_branch, called);
                if let Some(ref e) = else_branch {
                    collect_calls_in_expr(e, called);
                }
            }
            HirExpr::Match { expr, arms, .. } => {
                collect_calls_in_expr(expr, called);
                for arm in arms {
                    collect_calls_in_expr(&arm.body, called);
                }
            }
            HirExpr::For { collection, body, .. } => {
                collect_calls_in_expr(collection, called);
                collect_calls(body, called);
            }
            HirExpr::While { condition, body, .. } => {
                collect_calls_in_expr(condition, called);
                collect_calls(body, called);
            }
            HirExpr::Index { base, index, .. } => {
                collect_calls_in_expr(base, called);
                collect_calls_in_expr(index, called);
            }
            HirExpr::Try(inner, _) => collect_calls_in_expr(inner, called),
            HirExpr::Struct { fields, .. } => {
                for (_, e) in fields {
                    collect_calls_in_expr(e, called);
                }
            }
            HirExpr::FieldAccess { base, .. } => collect_calls_in_expr(base, called),
            _ => {}
        }
    }

    for item in &hir.items {
        match item {
            HirItem::FnDecl(f) => collect_calls(&f.body, &mut called_functions),
            HirItem::LetDecl(l) => collect_calls_in_expr(&l.value, &mut called_functions),
            _ => {}
        }
    }

    // Flag functions that are never called (skip "main")
    for item in &hir.items {
        if let HirItem::FnDecl(f) = item {
            if f.name != "main" && !f.is_public && !called_functions.contains(&f.name) {
                warnings.push(LintWarning {
                    message: format!("Function '{}' is defined but never called", f.name),
                    line: f.span.map(|s| s.line).unwrap_or(0),
                    kind: LintKind::DeadCode,
                    suggestion: Some("Consider removing this function or marking it as public.".to_string()),
                });
            }
        }
    }
}

/// Detect unused imports
fn check_unused_imports(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    // Collect all identifiers referenced in the program
    let mut used_symbols: HashSet<String> = HashSet::new();

    fn collect_idents(block: &HirBlock, used: &mut HashSet<String>) {
        for stmt in &block.statements {
            collect_idents_in_stmt(stmt, used);
        }
    }

    fn collect_idents_in_stmt(stmt: &HirStmt, used: &mut HashSet<String>) {
        match stmt {
            HirStmt::Let(l) => collect_idents_in_expr(&l.value, used),
            HirStmt::Return(r) => {
                if let Some(ref e) = r.value {
                    collect_idents_in_expr(e, used);
                }
            }
            HirStmt::Expr(e) => collect_idents_in_expr(e, used),
            HirStmt::Using(u) => {
                collect_idents_in_expr(&u.init, used);
                collect_idents(&u.body, used);
            }
            _ => {}
        }
    }

    fn collect_idents_in_expr(expr: &HirExpr, used: &mut HashSet<String>) {
        match expr {
            HirExpr::Ident(name, _) => { used.insert(name.clone()); }
            HirExpr::Call { function, args, .. } => {
                used.insert(function.clone());
                for a in args {
                    collect_idents_in_expr(a, used);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                collect_idents_in_expr(left, used);
                collect_idents_in_expr(right, used);
            }
            HirExpr::Block(b, _) => collect_idents(b, used),
            HirExpr::If { condition, then_branch, else_branch, .. } => {
                collect_idents_in_expr(condition, used);
                collect_idents(then_branch, used);
                if let Some(ref e) = else_branch {
                    collect_idents_in_expr(e, used);
                }
            }
            HirExpr::Match { expr, arms, .. } => {
                collect_idents_in_expr(expr, used);
                for arm in arms {
                    collect_idents_in_expr(&arm.body, used);
                }
            }
            HirExpr::For { item: _, collection, body, .. } => {
                collect_idents_in_expr(collection, used);
                collect_idents(body, used);
            }
            HirExpr::While { condition, body, .. } => {
                collect_idents_in_expr(condition, used);
                collect_idents(body, used);
            }
            HirExpr::Index { base, index, .. } => {
                collect_idents_in_expr(base, used);
                collect_idents_in_expr(index, used);
            }
            HirExpr::Try(inner, _) => collect_idents_in_expr(inner, used),
            HirExpr::Struct { fields, .. } => {
                for (_, e) in fields {
                    collect_idents_in_expr(e, used);
                }
            }
            HirExpr::FieldAccess { base, .. } => collect_idents_in_expr(base, used),
            _ => {}
        }
    }

    for item in &hir.items {
        match item {
            HirItem::FnDecl(f) => {
                collect_idents(&f.body, &mut used_symbols);
            }
            HirItem::LetDecl(l) => {
                collect_idents_in_expr(&l.value, &mut used_symbols);
            }
            _ => {}
        }
    }

    // Check each import for usage
    for import in &hir.imports {
        let is_used = import.items.iter().any(|item_name| {
            if item_name == "*" || item_name == "prelude" {
                true // wildcard imports are always "used"
            } else {
                used_symbols.contains(item_name)
            }
        });

        if !is_used {
            warnings.push(LintWarning {
                message: format!("Import '{}' is never used", import.path),
                line: 0,
                kind: LintKind::UnusedImport,
                suggestion: Some(format!("Consider removing 'import {}'", import.path)),
            });
        }
    }
}

/// Detect unused local variables (let bindings that are never read after definition)
fn check_unused_variables(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    fn check_block(block: &HirBlock, outer_used: &mut HashSet<String>, warnings: &mut Vec<LintWarning>) {
        let mut local_defs: HashMap<String, usize> = HashMap::new(); // name -> statement index
        let mut used: HashSet<String> = HashSet::new();

        // First collect all variable usages in the block
        fn collect_uses_in_block(block: &HirBlock, used: &mut HashSet<String>) {
            for stmt in &block.statements {
                match stmt {
                    HirStmt::Let(l) => collect_idents_in_expr_light(&l.value, used),
                    HirStmt::Return(r) => {
                        if let Some(ref e) = r.value {
                            collect_idents_in_expr_light(e, used);
                        }
                    }
                    HirStmt::Expr(e) => collect_idents_in_expr_light(e, used),
                    HirStmt::Using(u) => {
                        collect_idents_in_expr_light(&u.init, used);
                        collect_uses_in_block(&u.body, used);
                    }
                    _ => {}
                }
            }
        }

        fn collect_idents_in_expr_light(expr: &HirExpr, used: &mut HashSet<String>) {
            match expr {
                HirExpr::Ident(name, _) => { used.insert(name.clone()); }
                HirExpr::Call { function, args, .. } => {
                    used.insert(function.clone());
                    for a in args { collect_idents_in_expr_light(a, used); }
                }
                HirExpr::Binary { left, right, .. } => {
                    collect_idents_in_expr_light(left, used);
                    collect_idents_in_expr_light(right, used);
                }
                HirExpr::Block(b, _) => collect_uses_in_block(b, used),
                HirExpr::If { condition, then_branch, else_branch, .. } => {
                    collect_idents_in_expr_light(condition, used);
                    collect_uses_in_block(then_branch, used);
                    if let Some(ref e) = else_branch {
                        collect_idents_in_expr_light(e, used);
                    }
                }
                HirExpr::Match { expr, arms, .. } => {
                    collect_idents_in_expr_light(expr, used);
                    for arm in arms { collect_idents_in_expr_light(&arm.body, used); }
                }
                HirExpr::For { collection, body, .. } => {
                    collect_idents_in_expr_light(collection, used);
                    collect_uses_in_block(body, used);
                }
                HirExpr::While { condition, body, .. } => {
                    collect_idents_in_expr_light(condition, used);
                    collect_uses_in_block(body, used);
                }
                HirExpr::Index { base, index, .. } => {
                    collect_idents_in_expr_light(base, used);
                    collect_idents_in_expr_light(index, used);
                }
                HirExpr::Try(inner, _) => collect_idents_in_expr_light(inner, used),
                HirExpr::Struct { fields, .. } => {
                    for (_, e) in fields { collect_idents_in_expr_light(e, used); }
                }
                HirExpr::FieldAccess { base, .. } => collect_idents_in_expr_light(base, used),
                _ => {}
            }
        }

        collect_uses_in_block(block, &mut used);

        // Now check each let binding
        for (idx, stmt) in block.statements.iter().enumerate() {
            if let HirStmt::Let(l) = stmt {
                if !l.name.starts_with('_') && !used.contains(&l.name) {
                    warnings.push(LintWarning {
                        message: format!("Variable '{}' is assigned but never used", l.name),
                        line: l.span.map(|s| s.line).unwrap_or(0),
                        kind: LintKind::UnusedVariable,
                        suggestion: Some(format!("Consider prefixing with '_' or removing 'let {}'", l.name)),
                    });
                }
            }
        }
    }

    for item in &hir.items {
        match item {
            HirItem::FnDecl(f) => {
                let mut used = HashSet::new();
                check_block(&f.body, &mut used, warnings);
            }
            _ => {}
        }
    }
}

/// Detect linear resource leaks
fn check_linear_resource_leaks(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    // Check for `using` blocks that might leak resources
    // Also check for functions with linear parameters that are never consumed
    for item in &hir.items {
        if let HirItem::FnDecl(f) = item {
            let has_linear_param = f.params.iter().any(|p| p.is_linear);
            if has_linear_param {
                // Check if the function body actually uses (consumes) all linear params
                let linear_params: HashSet<String> = f.params.iter()
                    .filter(|p| p.is_linear)
                    .map(|p| p.name.clone())
                    .collect();

                let mut used_params: HashSet<String> = HashSet::new();
                fn collect_used_idents(block: &HirBlock, used: &mut HashSet<String>) {
                    for stmt in &block.statements {
                        match stmt {
                            HirStmt::Expr(HirExpr::Ident(name, _)) => { used.insert(name.clone()); }
                            HirStmt::Return(r) => {
                                if let Some(HirExpr::Ident(name, _)) = r.value.as_ref() {
                                    used.insert(name.clone());
                                }
                            }
                            HirStmt::Let(l) => {
                                if let HirExpr::Ident(name, _) = &l.value {
                                    used.insert(name.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                collect_used_idents(&f.body, &mut used_params);

                for param_name in &linear_params {
                    if !used_params.contains(param_name) {
                        warnings.push(LintWarning {
                            message: format!(
                                "Linear parameter '{}' in function '{}' is never used",
                                param_name, f.name
                            ),
                            line: f.span.map(|s| s.line).unwrap_or(0),
                            kind: LintKind::LinearResourceLeak,
                            suggestion: Some(format!(
                                "Ensure '{}' is consumed before the function returns",
                                param_name
                            )),
                        });
                    }
                }
            }
        }
    }
}

/// Check style issues (missing annotations, naming)
fn check_style_issues(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    for item in &hir.items {
        match item {
            HirItem::FnDecl(f) => {
                if f.return_type.is_none() && !f.body.statements.is_empty() {
                    if let Some(stmt) = f.body.statements.last() {
                        if !is_unit_expression(stmt) {
                            warnings.push(LintWarning {
                                message: format!("Function '{}' has no return type annotation", f.name),
                                line: f.span.map(|s| s.line).unwrap_or(0),
                                kind: LintKind::StyleIssue,
                                suggestion: Some("Consider adding an explicit return type.".to_string()),
                            });
                        }
                    }
                }
                // Check naming convention: functions should be snake_case
                if f.name.contains(char::is_uppercase) {
                    warnings.push(LintWarning {
                        message: format!(
                            "Function '{}' uses PascalCase; consider using snake_case",
                            f.name
                        ),
                        line: f.span.map(|s| s.line).unwrap_or(0),
                        kind: LintKind::StyleIssue,
                        suggestion: Some(format!(
                            "Rename to '{}' for consistency",
                            to_snake_case(&f.name)
                        )),
                    });
                }
            }
            HirItem::LetDecl(l) => {
                // Top-level constants should be UPPER_SNAKE_CASE
                if l.name.chars().all(|c| c.is_lowercase() || c == '_') == false
                    && l.name.contains(char::is_lowercase)
                {
                    // Mixed case let — not enforcing for now
                }
            }
            HirItem::StructDecl(s) => {
                // Structs should be PascalCase
                if s.name.chars().next().map_or(true, |c| !c.is_uppercase()) {
                    warnings.push(LintWarning {
                        message: format!(
                            "Struct '{}' should use PascalCase",
                            s.name
                        ),
                        line: s.span.map(|s| s.line).unwrap_or(0),
                        kind: LintKind::StyleIssue,
                        suggestion: Some(format!(
                            "Rename to '{}'",
                            to_pascal_case(&s.name)
                        )),
                    });
                }
            }
            _ => {}
        }
    }
}

fn is_unit_expression(stmt: &HirStmt) -> bool {
    match stmt {
        HirStmt::Let(_) => true,
        HirStmt::Using(_) => true,
        HirStmt::Continue(_) | HirStmt::Break(_) => true,
        HirStmt::Return(r) => r.value.is_none(),
        HirStmt::Expr(e) => matches!(e, HirExpr::Literal(once_hir::HirLiteral::Unit, _)),
    }
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

fn to_pascal_case(name: &str) -> String {
    let mut result = String::new();
    let mut capitalize = true;
    for c in name.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            result.push(c.to_uppercase().next().unwrap());
            capitalize = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Warn when `box T` or `rc T` is used (escape hatch from RMM, see ONCE-004 §2.4)
fn check_box_rc_usage(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    fn check_type(ty: &once_hir::HirType, warnings: &mut Vec<LintWarning>) {
        match ty {
            once_hir::HirType::Linear(inner) => {
                if let once_hir::HirType::Ident(name) = inner.as_ref() {
                    if name == "box" {
                        warnings.push(LintWarning {
                            message: "Usage of `box T` escape hatch detected — prefer region-based allocation (ONCE-004 §2.4)".to_string(),
                            line: 0,
                            kind: LintKind::BoxRcWarning,
                            suggestion: Some("Consider using region-inferred allocation instead of box.".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    
    for item in &hir.items {
        match item {
            once_hir::HirItem::FnDecl(f) => {
                for param in &f.params {
                    if let Some(ty) = &param.type_annotation {
                        check_type(ty, warnings);
                    }
                }
            }
            once_hir::HirItem::LetDecl(l) => {
                if let Some(ty) = &l.type_annotation {
                    check_type(ty, warnings);
                }
            }
            _ => {}
        }
    }
}

/// Warn when a function declares an effect that is never used in its body
fn check_unused_effects(hir: &HirProgram, warnings: &mut Vec<LintWarning>) {
    for item in &hir.items {
        if let once_hir::HirItem::FnDecl(f) = item {
            if let Some(ref effect_row) = f.effects {
                if effect_row.effects.is_empty() {
                    continue;
                }
                
                // Collect all identifiers (function calls) in the body
                let mut called_functions: HashSet<String> = HashSet::new();
                collect_calls_in_body(&f.body, &mut called_functions);
                
                // Run effect checker to get actual effect set
                let mut effect_checker = once_ty::effects::EffectChecker::new();
                if let Ok(()) = effect_checker.check(hir) {
                    for declared_effect in &effect_row.effects {
                        // Check if the declared effect matches any inferred effect
                        let mut found = false;
                        for (_, body_effects) in &effect_checker.env.bindings {
                            use once_ty::effects::EffectLabel;
                            let label = match declared_effect.as_str() {
                                "io" => EffectLabel::Io,
                                "net" => EffectLabel::Net,
                                "spawn" => EffectLabel::Spawn,
                                "time" => EffectLabel::Time,
                                "ffi" => EffectLabel::Ffi,
                                _ => EffectLabel::Custom(declared_effect.clone()),
                            };
                            if effect_checker.contains_effect(body_effects, &label) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            warnings.push(LintWarning {
                                message: format!("Function '{}' declares `!{}` effect but it is never used", f.name, declared_effect),
                                line: f.span.map(|s| s.line).unwrap_or(0),
                                kind: LintKind::UnusedEffect,
                                suggestion: Some(format!("Consider removing '!{}' from the effect annotation.", declared_effect)),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn collect_calls_in_body(block: &HirBlock, called: &mut HashSet<String>) {
    for stmt in &block.statements {
        match stmt {
            HirStmt::Let(l) => collect_calls_in_expr_lint(&l.value, called),
            HirStmt::Return(r) => {
                if let Some(ref e) = r.value {
                    collect_calls_in_expr_lint(e, called);
                }
            }
            HirStmt::Expr(e) => collect_calls_in_expr_lint(e, called),
            HirStmt::Using(u) => {
                collect_calls_in_expr_lint(&u.init, called);
                collect_calls_in_body(&u.body, called);
            }
            _ => {}
        }
    }
}

fn collect_calls_in_expr_lint(expr: &HirExpr, called: &mut HashSet<String>) {
    match expr {
        HirExpr::Call { function, args, .. } => {
            called.insert(function.clone());
            for a in args {
                collect_calls_in_expr_lint(a, called);
            }
        }
        HirExpr::Binary { left, right, .. } => {
            collect_calls_in_expr_lint(left, called);
            collect_calls_in_expr_lint(right, called);
        }
        HirExpr::Block(b, _) => collect_calls_in_body(b, called),
        HirExpr::If { condition, then_branch, else_branch, .. } => {
            collect_calls_in_expr_lint(condition, called);
            collect_calls_in_body(then_branch, called);
            if let Some(ref e) = else_branch {
                collect_calls_in_expr_lint(e, called);
            }
        }
        HirExpr::Match { expr, arms, .. } => {
            collect_calls_in_expr_lint(expr, called);
            for arm in arms {
                collect_calls_in_expr_lint(&arm.body, called);
            }
        }
        HirExpr::For { collection, body, .. } => {
            collect_calls_in_expr_lint(collection, called);
            collect_calls_in_body(body, called);
        }
        HirExpr::While { condition, body, .. } => {
            collect_calls_in_expr_lint(condition, called);
            collect_calls_in_body(body, called);
        }
        HirExpr::Index { base, index, .. } => {
            collect_calls_in_expr_lint(base, called);
            collect_calls_in_expr_lint(index, called);
        }
        HirExpr::Try(inner, _) => collect_calls_in_expr_lint(inner, called),
        HirExpr::Struct { fields, .. } => {
            for (_, e) in fields {
                collect_calls_in_expr_lint(e, called);
            }
        }
        HirExpr::FieldAccess { base, .. } => collect_calls_in_expr_lint(base, called),
        _ => {}
    }
}
