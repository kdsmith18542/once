//! Canonical formatter for Once source code.
//! Pretty-prints AST back to canonical Once syntax.

use crate::*;

pub fn format_program(program: &Program) -> String {
    let mut out = String::new();
    for item in &program.items {
        format_item(item, 0, &mut out);
        out.push('\n');
    }
    out
}

fn indent(depth: usize) -> String {
    "    ".repeat(depth)
}

fn format_item(item: &Item, depth: usize, out: &mut String) {
    match item {
        Item::FnDecl(f) => {
            out.push_str(&format!("{}fn {}", indent(depth), f.name));
            if !f.type_params.is_empty() {
                out.push('<');
                for (i, p) in f.type_params.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(&p.name);
                }
                out.push('>');
            }
            out.push('(');
            for (i, p) in f.params.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(&p.name);
                if let Some(ty) = &p.type_annotation {
                    out.push_str(": ");
                    format_type(ty, out);
                }
            }
            out.push(')');
            if let Some(ty) = &f.return_type {
                out.push_str(" -> ");
                format_type(ty, out);
            }
            if let Some(eff) = &f.effects {
                out.push_str(&format!(" ![{}]", eff.effects.join(", ")));
            }
            out.push(' ');
            format_block(&f.body, depth, false, out);
        }
        Item::LetDecl(l) => {
            let kw = if l.mutable { "var" } else { "let" };
            out.push_str(&format!("{}{} {}", indent(depth), kw, l.name));
            if let Some(ty) = &l.type_annotation {
                out.push_str(": ");
                format_type(ty, out);
            }
            out.push_str(" = ");
            format_expr(&l.value, depth + 1, true, out);
            out.push(';');
        }
        Item::TypeDecl(t) => {
            out.push_str(&format!("{}type {}", indent(depth), t.name));
            if !t.type_params.is_empty() {
                out.push('<');
                for (i, p) in t.type_params.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(&p.name);
                }
                out.push('>');
            }
            out.push_str(" = ");
            for (i, v) in t.variants.iter().enumerate() {
                if i > 0 { out.push_str(" | "); }
                out.push_str(&v.name);
                if !v.fields.is_empty() {
                    out.push('(');
                    for (j, f) in v.fields.iter().enumerate() {
                        if j > 0 { out.push_str(", "); }
                        format_type(f, out);
                    }
                    out.push(')');
                }
            }
            out.push(';');
        }
        Item::StructDecl(s) => {
            out.push_str(&format!("{}struct {} {{\n", indent(depth), s.name));
            for field in &s.fields {
                out.push_str(&format!("{}    {}: ", indent(depth), field.name));
                format_type(&field.field_type, out);
                out.push_str(",\n");
            }
            out.push_str(&format!("{}}}", indent(depth)));
        }
        Item::TraitDecl(tr) => {
            out.push_str(&format!("{}trait {} {{\n", indent(depth), tr.name));
            for m in &tr.methods {
                format_item(&Item::FnDecl(m.clone()), depth + 1, out);
                out.push('\n');
            }
            out.push_str(&format!("{}}}", indent(depth)));
        }
        Item::ImplBlock(imp) => {
            if let Some(trait_name) = &imp.trait_name {
                out.push_str(&format!("{}impl {} for ", indent(depth), trait_name));
            } else {
                out.push_str(&format!("{}impl ", indent(depth)));
            }
            format_type(&imp.target_type, out);
            out.push_str(" {\n");
            for m in &imp.methods {
                format_item(&Item::FnDecl(m.clone()), depth + 1, out);
                out.push('\n');
            }
            out.push_str(&format!("{}}}", indent(depth)));
        }
        Item::GoalDecl(_) => {
            out.push_str(&format!("{}goal /* ... */ {{}}", indent(depth)));
        }
        Item::ImportDecl(imp) => {
            out.push_str(&format!("{}import {}", indent(depth), imp.path.join("::")));
            if let Some(alias) = &imp.alias {
                out.push_str(&format!(" as {}", alias));
            }
            if !imp.items.is_empty() {
                out.push_str(&format!(" {{ {} }}", imp.items.join(", ")));
            }
            out.push(';');
        }
        Item::SchemaDecl(s) => {
            out.push_str(&format!("{}schema {} from {} for ", indent(depth), s.name, s.source_type));
            format_type(&s.target_type, out);
            out.push_str(" {\n");
            for (field, path) in &s.fields {
                out.push_str(&format!("{}    {}: \"{}\",\n", indent(depth), field, path));
            }
            out.push_str(&format!("{}}}", indent(depth)));
        }
    }
}

fn format_type(ty: &Type, out: &mut String) {
    match ty {
        Type::Ident(n) => out.push_str(n),
        Type::Unit => out.push_str("Unit"),
        Type::Int => out.push_str("Int"),
        Type::Bool => out.push_str("Bool"),
        Type::Float => out.push_str("Float"),
        Type::Str => out.push_str("Str"),
        Type::Hole => out.push('_'),
        Type::Linear(t) => { out.push_str("lin "); format_type(t, out); }
        Type::Affine(t) => { out.push_str("aff "); format_type(t, out); }
        Type::Array(t, n) => { out.push('['); format_type(t, out); out.push_str(&format!("; {}", n)); out.push(']'); }
        Type::Generic(name, args) => {
            out.push_str(name);
            out.push('<');
            for (i, a) in args.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_type(a, out);
            }
            out.push('>');
        }
        Type::Tuple(types) => {
            out.push('(');
            for (i, t) in types.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_type(t, out);
            }
            out.push(')');
        }
        Type::Function(args, ret) => {
            out.push_str("fn(");
            for (i, a) in args.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_type(a, out);
            }
            out.push_str(") -> ");
            format_type(ret, out);
        }
    }
}

fn format_block(block: &Block, depth: usize, inline: bool, out: &mut String) {
    if block.statements.is_empty() {
        out.push_str("{}");
        return;
    }
    if inline && block.statements.len() == 1 {
        if let Stmt::Expr(e) = &block.statements[0] {
            format_expr(e, depth, true, out);
            return;
        }
    }
    out.push_str("{\n");
    for stmt in &block.statements {
        format_stmt(stmt, depth + 1, out);
        out.push('\n');
    }
    out.push_str(&format!("{}}}", indent(depth)));
}

fn format_stmt(stmt: &Stmt, depth: usize, out: &mut String) {
    match stmt {
        Stmt::Let(l) => {
            let kw = if l.mutable { "var" } else { "let" };
            out.push_str(&format!("{}{} {}", indent(depth), kw, l.name));
            if let Some(ty) = &l.type_annotation {
                out.push_str(": ");
                format_type(ty, out);
            }
            out.push_str(" = ");
            format_expr(&l.value, depth + 1, true, out);
            out.push(';');
        }
        Stmt::Return(r) => {
            out.push_str(&format!("{}return", indent(depth)));
            if let Some(v) = &r.value {
                out.push(' ');
                format_expr(v, depth + 1, true, out);
            }
            out.push(';');
        }
        Stmt::Expr(e) => {
            out.push_str(&indent(depth));
            format_expr(e, depth, true, out);
        }
        Stmt::Using(u) => {
            out.push_str(&format!("{}using {} = ", indent(depth), u.name));
            format_expr(&u.init, depth + 1, true, out);
            out.push(' ');
            format_block(&u.body, depth, false, out);
        }
        Stmt::Continue => {
            out.push_str(&format!("{}continue;", indent(depth)));
        }
        Stmt::Break => {
            out.push_str(&format!("{}break;", indent(depth)));
        }
    }
}

fn format_expr(expr: &Expr, depth: usize, _inline: bool, out: &mut String) {
    match expr {
        Expr::Literal(l, _) => match l {
            Literal::Int(n) => out.push_str(&n.to_string()),
            Literal::Float(n) => out.push_str(&n.to_string()),
            Literal::String(s) => out.push_str(&format!("\"{}\"", s)),
            Literal::Bool(b) => out.push_str(&b.to_string()),
            Literal::Unit => out.push_str("()"),
        },
        Expr::Ident(n, _) => out.push_str(n),
        Expr::Call { function, args, .. } => {
            out.push_str(&format!("{}(", function));
            for (i, a) in args.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_expr(a, depth, true, out);
            }
            out.push(')');
        }
        Expr::Binary { left, op, right, .. } => {
            format_expr(left, depth, true, out);
            out.push(' ');
            out.push_str(match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Eq => "==",
                BinaryOp::Ne => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Le => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Ge => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
                BinaryOp::Assign => "=",
            });
            out.push(' ');
            format_expr(right, depth, true, out);
        }
        Expr::Block(b, _) => format_block(b, depth, false, out),
        Expr::If { condition, then_branch, else_branch, .. } => {
            out.push_str("if ");
            format_expr(condition, depth, true, out);
            out.push(' ');
            format_block(then_branch, depth, false, out);
            if let Some(else_expr) = else_branch {
                out.push_str(" else ");
                match else_expr.as_ref() {
                    Expr::If { .. } => {
                        format_expr(else_expr, depth, true, out);
                    }
                    Expr::Block(b, _) => format_block(b, depth, false, out),
                    _ => format_expr(else_expr, depth + 1, true, out),
                }
            }
        }
        Expr::Match { expr, arms, .. } => {
            out.push_str("match ");
            format_expr(expr, depth, true, out);
            out.push_str(" {\n");
            for arm in arms {
                out.push_str(&indent(depth + 1));
                format_pattern(&arm.pattern, out);
                if let Some(ref guard) = arm.guard {
                    out.push_str(" if ");
                    format_expr(guard, depth + 1, true, out);
                }
                out.push_str(" => ");
                format_expr(&arm.body, depth + 1, true, out);
                out.push_str(",\n");
            }
            out.push_str(&format!("{}}}", indent(depth)));
        }
        Expr::For { item, collection, body, .. } => {
            out.push_str(&format!("for {} in ", item));
            format_expr(collection, depth, true, out);
            out.push(' ');
            format_block(body, depth, false, out);
        }
        Expr::While { condition, body, .. } => {
            out.push_str("while ");
            format_expr(condition, depth, true, out);
            out.push(' ');
            format_block(body, depth, false, out);
        }
        Expr::Index { base, index, .. } => {
            format_expr(base, depth, true, out);
            out.push('[');
            format_expr(index, depth, true, out);
            out.push(']');
        }
        Expr::Try(inner, _) => {
            out.push_str("try ");
            format_expr(inner, depth, true, out);
        }
        Expr::Struct { name, fields, .. } => {
            out.push_str(name);
            out.push_str(" { ");
            for (i, (n, e)) in fields.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(&format!("{}: ", n));
                format_expr(e, depth, true, out);
            }
            out.push_str(" }");
        }
        Expr::FieldAccess { base, field, .. } => {
            format_expr(base, depth, true, out);
            out.push('.');
            out.push_str(field);
        }
    }
}

fn format_pattern(pat: &Pattern, out: &mut String) {
    match pat {
        Pattern::Literal(l) => match l {
            Literal::Int(n) => out.push_str(&n.to_string()),
            Literal::Float(n) => out.push_str(&n.to_string()),
            Literal::String(s) => out.push_str(&format!("\"{}\"", s)),
            Literal::Bool(b) => out.push_str(&b.to_string()),
            Literal::Unit => out.push_str("()"),
        },
        Pattern::Ident(n) => out.push_str(n),
        Pattern::Wildcard => out.push('_'),
        Pattern::EnumVariant { name, fields } => {
            out.push_str(name);
            out.push('(');
            for (i, f) in fields.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                format_pattern(f, out);
            }
            out.push(')');
        }
    }
}