use crate::{MirProgram, MirFunction, MirOp, MirLocation};
use thiserror::Error;
use std::collections::HashSet;

#[derive(Error, Debug)]
pub enum MirVerifyError {
    #[error("used-before-assigned: location {location} is read before being written")]
    UsedBeforeAssigned { location: String },

    #[error("unused assignment: location {location} is written but never read")]
    UnusedAssignment { location: String },

    #[error("undefined label target: label {label} referenced but never defined")]
    UndefinedLabelTarget { label: usize },

    #[error("unused label: label {label} defined but never referenced")]
    UnusedLabel { label: usize },

    #[error("unreachable code after unconditional jump/return at statement {stmt_index}")]
    UnreachableCode { stmt_index: usize },

    #[error("region free without allocation: freeing region {region} with no prior allocation in function")]
    RegionFreeWithoutAlloc { region: String },

    #[error("double free: region {region} freed more than once")]
    DoubleFree { region: String },

    #[error("allocation never freed: region {region} allocated but never freed in function")]
    AllocNeverFreed { region: String },

    #[error("concurrency verification not yet implemented: {message}")]
    ConcurrencyViolation { message: String },
}

pub struct MirVerifier;

impl MirVerifier {
    pub fn new() -> Self {
        Self
    }

    pub fn verify_program(&self, mir: &MirProgram) -> Result<(), Vec<MirVerifyError>> {
        let mut errors = Vec::new();
        for func in &mir.functions {
            if let Err(mut e) = self.verify_function(func) {
                errors.append(&mut e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn verify_function(&self, func: &MirFunction) -> Result<(), Vec<MirVerifyError>> {
        let mut errors = Vec::new();

        // ── label tracking ──
        let mut defined_labels: HashSet<usize> = HashSet::new();
        let mut referenced_labels: HashSet<usize> = HashSet::new();

        // ── region tracking ──
        let mut allocated_regions: HashSet<String> = HashSet::new();
        let mut freed_regions: HashSet<String> = HashSet::new();

        // ── location tracking ──
        let mut written_locs: HashSet<String> = HashSet::new();
        let mut read_locs: HashSet<String> = HashSet::new();

        // Parameters are implicitly written at function entry
        for (i, _) in func.params.iter().enumerate() {
            written_locs.insert(format!("param_{}", i));
        }

        // Return slot is treated as written
        written_locs.insert("return".to_string());

        // Track whether we are past a terminal (Jump or Return) for unreachable-code detection
        let mut after_terminal = false;
        let stmts = &func.body.statements;

        for (i, stmt) in stmts.iter().enumerate() {
            if after_terminal {
                match &stmt.op {
                    MirOp::Label { .. } => {
                        after_terminal = false;
                    }
                    _ => {
                        errors.push(MirVerifyError::UnreachableCode { stmt_index: i });
                        continue;
                    }
                }
            }

            match &stmt.op {
                MirOp::Label { id } => {
                    defined_labels.insert(*id);
                }
                MirOp::Jump { target } => {
                    referenced_labels.insert(*target);
                    after_terminal = true;
                }
                MirOp::Branch {
                    true_target,
                    false_target,
                    ..
                } => {
                    referenced_labels.insert(*true_target);
                    referenced_labels.insert(*false_target);
                }
                MirOp::Return { .. } => {
                    after_terminal = true;
                }
                _ => {}
            }

            // ── region safety ──
            match &stmt.op {
                MirOp::Allocate { region, .. } => {
                    allocated_regions.insert(format!("{:?}", region));
                }
                MirOp::FreeRegion { region } => {
                    let key = format!("{:?}", region);
                    if freed_regions.contains(&key) {
                        errors.push(MirVerifyError::DoubleFree { region: key.clone() });
                    }
                    freed_regions.insert(key);
                }
                _ => {}
            }

            // ── location reads / writes ──
            self.track_locations(&stmt.op, &mut written_locs, &mut read_locs);
        }

        // ── label checks ──
        for label in &referenced_labels {
            if !defined_labels.contains(label) {
                errors.push(MirVerifyError::UndefinedLabelTarget { label: *label });
            }
        }
        for label in &defined_labels {
            if !referenced_labels.contains(label) {
                errors.push(MirVerifyError::UnusedLabel { label: *label });
            }
        }

        // ── used-before-assigned ──
        for loc in &read_locs {
            if !written_locs.contains(loc) {
                errors.push(MirVerifyError::UsedBeforeAssigned {
                    location: loc.clone(),
                });
            }
        }

        // ── unused assignments ──
        for loc in &written_locs {
            if !read_locs.contains(loc)
                && !loc.starts_with("param_")
                && loc != "return"
            {
                errors.push(MirVerifyError::UnusedAssignment {
                    location: loc.clone(),
                });
            }
        }

        // ── region safety checks ──
        for freed in &freed_regions {
            if !allocated_regions.contains(freed) {
                errors.push(MirVerifyError::RegionFreeWithoutAlloc {
                    region: freed.clone(),
                });
            }
        }
        for allocd in &allocated_regions {
            if !freed_regions.contains(allocd) {
                errors.push(MirVerifyError::AllocNeverFreed {
                    region: allocd.clone(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn track_locations(
        &self,
        op: &MirOp,
        written: &mut HashSet<String>,
        read: &mut HashSet<String>,
    ) {
        match op {
            MirOp::Move { from, to } => {
                self.collect_read(from, read);
                self.collect_write(to, written);
            }
            MirOp::Drop { location } => {
                self.collect_read(location, read);
            }
            MirOp::FreeRegion { .. } => {}
            MirOp::Allocate { dest, .. } => {
                self.collect_write(dest, written);
            }
            MirOp::BoundsCheck { index, bound, .. } => {
                self.collect_read(index, read);
                self.collect_read(bound, read);
            }
            MirOp::BinOp {
                left, right, dest, ..
            } => {
                self.collect_read(left, read);
                self.collect_read(right, read);
                self.collect_write(dest, written);
            }
            MirOp::ChannelSend { channel, value } => {
                self.collect_read(channel, read);
                self.collect_read(value, read);
            }
            MirOp::ChannelRecv { channel, result } => {
                self.collect_read(channel, read);
                self.collect_write(result, written);
            }
            MirOp::SpawnTask { args, result, .. } => {
                for a in args {
                    self.collect_read(a, read);
                }
                self.collect_write(result, written);
            }
            MirOp::AwaitTask { task, result } => {
                self.collect_read(task, read);
                self.collect_write(result, written);
            }
            MirOp::CreateGroup { result } => {
                self.collect_write(result, written);
            }
            MirOp::SpawnInGroup {
                group, args, result, ..
            } => {
                self.collect_read(group, read);
                for a in args {
                    self.collect_read(a, read);
                }
                self.collect_write(result, written);
            }
            MirOp::AwaitGroup { group, result } => {
                self.collect_read(group, read);
                self.collect_write(result, written);
            }
            MirOp::Call { args, result, .. } => {
                for a in args {
                    self.collect_read(a, read);
                }
                self.collect_write(result, written);
            }
            MirOp::Return { value } => {
                if let Some(v) = value {
                    self.collect_read(v, read);
                }
            }
            MirOp::LoadLiteral { dest, .. } => {
                self.collect_write(dest, written);
            }
            MirOp::Jump { .. } => {}
            MirOp::Branch { condition, .. } => {
                self.collect_read(condition, read);
            }
            MirOp::Label { .. } => {}
            MirOp::TryBlock { result } => {
                self.collect_read(result, read);
            }
            MirOp::LoadLength { base, dest } => {
                self.collect_read(base, read);
                self.collect_write(dest, written);
            }
        }
    }

    fn collect_read(&self, loc: &MirLocation, read: &mut HashSet<String>) {
        read.insert(format!("{}", loc));
        // Recurse into compound locations
        match loc {
            MirLocation::Field { base, .. } => {
                self.collect_read(base, read);
            }
            MirLocation::Index { base, index } => {
                self.collect_read(base, read);
                self.collect_read(index, read);
            }
            _ => {}
        }
    }

    fn collect_write(&self, loc: &MirLocation, written: &mut HashSet<String>) {
        written.insert(format!("{}", loc));
    }
}
