use once_mir::{MirProgram, MirFunction, MirOp, MirLocation, MirValue, MirBinOp};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MirEvalError {
    #[error("MIR evaluation error: {0}")]
    EvalError(String),
    #[error("Undefined label: {0}")]
    UndefinedLabel(usize),
    #[error("Function not found: {0}")]
    FunctionNotFound(String),
    #[error("Division by zero")]
    DivisionByZero,
}

pub struct MirEvaluator {
    pub program: MirProgram,
}

impl MirEvaluator {
    pub fn new(program: MirProgram) -> Self {
        Self { program }
    }

    pub fn eval_function(&self, name: &str, args: &[MirValue]) -> Result<MirValue, MirEvalError> {
        let func = self.program.functions.iter()
            .find(|f| f.name == name)
            .ok_or_else(|| MirEvalError::FunctionNotFound(name.to_string()))?;

        self.eval_function_body(func, args)
    }

    fn eval_function_body(&self, func: &MirFunction, args: &[MirValue]) -> Result<MirValue, MirEvalError> {
        use std::collections::HashMap;

        let mut memory: HashMap<MirLocation, MirValue> = HashMap::new();
        let mut labels: HashMap<usize, usize> = HashMap::new();

        // Bind parameters to Param locations
        for (i, val) in args.iter().enumerate() {
            memory.insert(MirLocation::Param(i), val.clone());
        }

        // Map labels to statement indices
        for (i, stmt) in func.body.statements.iter().enumerate() {
            if let MirOp::Label { id } = &stmt.op {
                labels.insert(*id, i);
            }
        }

        let mut i = 0;
        while i < func.body.statements.len() {
            let stmt = &func.body.statements[i];
            i += 1;

            match &stmt.op {
                MirOp::LoadLiteral { value, dest } => {
                    memory.insert(dest.clone(), value.clone());
                }
                MirOp::Move { from, to } => {
                    if let Some(val) = memory.get(from) {
                        memory.insert(to.clone(), val.clone());
                    } else {
                        return Err(MirEvalError::EvalError(format!(
                            "Move source {:?} not found in memory", from
                        )));
                    }
                }
                MirOp::BinOp { op, left, right, dest } => {
                    let l = memory.get(left).cloned();
                    let r = memory.get(right).cloned();
                    match (l, r) {
                        (Some(lv), Some(rv)) => {
                            let result = self.eval_binop(op, &lv, &rv)?;
                            memory.insert(dest.clone(), result);
                        }
                        (None, _) => return Err(MirEvalError::EvalError(format!(
                            "BinOp left operand {:?} not found", left
                        ))),
                        (_, None) => return Err(MirEvalError::EvalError(format!(
                            "BinOp right operand {:?} not found", right
                        ))),
                    }
                }
                MirOp::Return { value } => {
                    return match value {
                        Some(loc) => memory.get(loc).cloned()
                            .ok_or_else(|| MirEvalError::EvalError("return value not found".to_string())),
                        None => Ok(MirValue::Bool(false)),
                    };
                }
                MirOp::Jump { target } => {
                    if let Some(&idx) = labels.get(target) {
                        i = idx;
                    } else {
                        return Err(MirEvalError::UndefinedLabel(*target));
                    }
                }
                MirOp::Branch { condition, true_target, false_target } => {
                    let cond_val = memory.get(condition).cloned();
                    let target = match cond_val {
                        Some(MirValue::Bool(true)) => true_target,
                        _ => false_target,
                    };
                    if let Some(&idx) = labels.get(target) {
                        i = idx;
                    } else {
                        return Err(MirEvalError::UndefinedLabel(*target));
                    }
                }
                MirOp::Label { .. } => {
                    // No-op, just a label marker
                }
                MirOp::Call { function, args, result } => {
                    let resolved_args: Vec<MirValue> = args.iter()
                        .filter_map(|a| memory.get(a).cloned())
                        .collect();
                    if resolved_args.len() != args.len() {
                        return Err(MirEvalError::EvalError(format!(
                            "Not all call args resolved for {}", function
                        )));
                    }
                    let call_result = self.eval_function(function, &resolved_args)?;
                    memory.insert(result.clone(), call_result);
                }
                MirOp::LoadLength { base: _, dest } => {
                    // For a simple interpreter without real arrays, store 0
                    memory.insert(dest.clone(), MirValue::Int(0));
                }
                MirOp::Drop { .. }
                | MirOp::FreeRegion { .. }
                | MirOp::Allocate { .. }
                | MirOp::BoundsCheck { .. }
                | MirOp::ChannelSend { .. }
                | MirOp::ChannelRecv { .. }
                | MirOp::SpawnTask { .. }
                | MirOp::AwaitTask { .. }
                | MirOp::CreateGroup { .. }
                | MirOp::SpawnInGroup { .. }
                | MirOp::AwaitGroup { .. }
                | MirOp::TryBlock { .. } => {
                    // No-ops for test evaluation
                }
            }
        }

        // If we reach the end without a Return, return Unit
        Ok(MirValue::Unit)
    }

    fn eval_binop(&self, op: &MirBinOp, left: &MirValue, right: &MirValue) -> Result<MirValue, MirEvalError> {
        match (left, right) {
            (MirValue::Int(a), MirValue::Int(b)) => match op {
                MirBinOp::Add => Ok(MirValue::Int(a + b)),
                MirBinOp::Sub => Ok(MirValue::Int(a - b)),
                MirBinOp::Mul => Ok(MirValue::Int(a * b)),
                MirBinOp::Div => {
                    if *b == 0 { return Err(MirEvalError::DivisionByZero); }
                    Ok(MirValue::Int(a / b))
                }
                MirBinOp::Eq => Ok(MirValue::Bool(a == b)),
                MirBinOp::Ne => Ok(MirValue::Bool(a != b)),
                MirBinOp::Lt => Ok(MirValue::Bool(a < b)),
                MirBinOp::Le => Ok(MirValue::Bool(a <= b)),
                MirBinOp::Gt => Ok(MirValue::Bool(a > b)),
                MirBinOp::Ge => Ok(MirValue::Bool(a >= b)),
                _ => Ok(MirValue::Bool(false)),
            },
            (MirValue::Bool(a), MirValue::Bool(b)) => match op {
                MirBinOp::And => Ok(MirValue::Bool(*a && *b)),
                MirBinOp::Or => Ok(MirValue::Bool(*a || *b)),
                MirBinOp::Eq => Ok(MirValue::Bool(a == b)),
                MirBinOp::Ne => Ok(MirValue::Bool(a != b)),
                _ => Ok(MirValue::Bool(false)),
            },
            (MirValue::Float(a), MirValue::Float(b)) => match op {
                MirBinOp::Add => Ok(MirValue::Float(a + b)),
                MirBinOp::Sub => Ok(MirValue::Float(a - b)),
                MirBinOp::Mul => Ok(MirValue::Float(a * b)),
                MirBinOp::Div => {
                    if *b == 0.0 { return Err(MirEvalError::DivisionByZero); }
                    Ok(MirValue::Float(a / b))
                }
                MirBinOp::Eq => Ok(MirValue::Bool(a == b)),
                MirBinOp::Ne => Ok(MirValue::Bool(a != b)),
                MirBinOp::Lt => Ok(MirValue::Bool(a < b)),
                MirBinOp::Le => Ok(MirValue::Bool(a <= b)),
                MirBinOp::Gt => Ok(MirValue::Bool(a > b)),
                MirBinOp::Ge => Ok(MirValue::Bool(a >= b)),
                _ => Ok(MirValue::Bool(false)),
            },
            _ => Ok(MirValue::Bool(false)),
        }
    }
}
