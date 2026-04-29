//! Region Inference Solver for the Once language
//! 
//! Implements static lifetime inference for:
//! - Region constraint generation
//! - Escape analysis
//! - Liveness analysis
//! - Region DAG construction
//! - Fallback to box/rc when inference fails

use once_hir::*;
use once_ty::{Type, TypeVar};
use once_lex::Span;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use thiserror::Error;

/// Region inference errors
#[derive(Error, Debug, Clone)]
pub enum RegionError {
    #[error("Region constraint unsatisfiable: {0}")]
    UnsatisfiableConstraint(String),
    
    #[error("Escape analysis failed: {0}")]
    EscapeAnalysisFailed(String),
    
    #[error("Liveness analysis failed: {0}")]
    LivenessAnalysisFailed(String),
    
    #[error("Region DAG construction failed: {0}")]
    RegionDagFailed(String),
    
    #[error("Fallback required: {0}")]
    FallbackRequired(String),
}

/// Region variable
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Region {
    pub id: usize,
    pub name: String,
    pub is_primary: bool,
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_primary {
            write!(f, "R_{}", self.name)
        } else {
            write!(f, "ρ_{}", self.name)
        }
    }
}

/// Region constraint types
#[derive(Debug, Clone, PartialEq)]
pub enum RegionConstraint {
    /// Allocation constraint: `alloc(e) ∈ ρ`
    Allocation {
        expr_id: usize,
        region: Region,
    },
    /// Escape constraint: `escapes(v, ρ_src → ρ_dst)`
    Escape {
        value: String,
        source: Region,
        destination: Region,
    },
    /// Liveness constraint: `ρ` must outlive last use
    Liveness {
        region: Region,
        last_use: usize,
    },
    /// Subregion constraint: `ρ1 ⊆ ρ2`
    Subregion {
        sub: Region,
        super_: Region,
    },
    /// Equality constraint: `ρ1 = ρ2`
    Equality {
        left: Region,
        right: Region,
    },
}

/// Region DAG node
#[derive(Debug, Clone)]
pub struct RegionNode {
    pub region: Region,
    pub allocations: Vec<usize>,
    pub escapes: Vec<String>,
    pub last_use: Option<usize>,
    pub free_point: Option<usize>,
}

/// Region DAG
#[derive(Debug, Clone)]
pub struct RegionDag {
    pub nodes: HashMap<Region, RegionNode>,
    pub edges: Vec<(Region, Region)>,
    pub free_points: Vec<(Region, usize)>,
}

/// Region inference solver
pub struct RegionSolver {
    regions: HashMap<usize, Region>,
    constraints: Vec<RegionConstraint>,
    next_region_id: usize,
    errors: Vec<RegionError>,
}

impl RegionSolver {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            constraints: Vec::new(),
            next_region_id: 0,
            errors: Vec::new(),
        }
    }

    pub fn solve(&mut self, hir: &HirProgram) -> Result<RegionDag, Vec<RegionError>> {
        // Generate constraints from HIR
        self.generate_constraints(hir)?;
        
        // Solve constraints to build region DAG
        let dag = self.build_region_dag()?;
        
        // Place free points optimally
        self.place_free_points(&mut dag.clone())?;
        
        if self.errors.is_empty() {
            Ok(dag)
        } else {
            Err(self.errors.clone())
        }
    }

    fn generate_constraints(&mut self, hir: &HirProgram) -> Result<(), Vec<RegionError>> {
        for item in &hir.items {
            self.generate_item_constraints(item)?;
        }
        Ok(())
    }

    fn generate_item_constraints(&mut self, item: &HirItem) -> Result<(), Vec<RegionError>> {
        match item {
            HirItem::FnDecl(fn_decl) => {
                // Create primary region for function
                let primary_region = self.create_region(&format!("fn_{}", fn_decl.name), true);
                
                // Generate constraints for function body
                self.generate_block_constraints(&fn_decl.body, primary_region)?;
            }
            HirItem::LetDecl(_) => {
                // Global let declarations don't need region constraints
            }
        }
        Ok(())
    }

    fn generate_block_constraints(&mut self, block: &HirBlock, region: Region) -> Result<(), Vec<RegionError>> {
        for stmt in &block.statements {
            self.generate_stmt_constraints(stmt, region.clone())?;
        }
        Ok(())
    }

    fn generate_stmt_constraints(&mut self, stmt: &HirStmt, region: Region) -> Result<(), Vec<RegionError>> {
        match stmt {
            HirStmt::Let(let_stmt) => {
                // Generate constraints for the value expression
                self.generate_expr_constraints(&let_stmt.value, region.clone())?;
                
                // If this is an allocation, add allocation constraint
                if self.is_allocation(&let_stmt.value) {
                    self.constraints.push(RegionConstraint::Allocation {
                        expr_id: 0, // TODO: Use actual expression ID
                        region: region.clone(),
                    });
                }
            }
            HirStmt::Return(return_stmt) => {
                if let Some(_expr) = &return_stmt.value {
                    // Return value escapes to caller's region
                    let caller_region = self.create_region("caller", false);
                    self.constraints.push(RegionConstraint::Escape {
                        value: "return_value".to_string(),
                        source: region,
                        destination: caller_region,
                    });
                }
            }
            HirStmt::Expr(expr) => {
                self.generate_expr_constraints(expr, region)?;
            }
            HirStmt::Using(using_stmt) => {
                // Check init expression constraints
                self.generate_expr_constraints(&using_stmt.init, region.clone())?;
                // Check body constraints
                for stmt in &using_stmt.body.statements {
                    self.generate_stmt_constraints(stmt, region.clone())?;
                }
            }
        }
        Ok(())
    }

    fn generate_expr_constraints(&mut self, expr: &HirExpr, region: Region) -> Result<(), Vec<RegionError>> {
        match expr {
            HirExpr::Literal(_) => {
                // Literals don't need region constraints
            }
            HirExpr::Ident(_) => {
                // Variable references don't need region constraints
            }
            HirExpr::Call { function, args } => {
                // Generate constraints for arguments
                for arg in args {
                    self.generate_expr_constraints(arg, region.clone())?;
                }
                
                // Check for escape operations
                if self.is_escape_operation(function) {
                    let escape_region = self.create_region("escape", false);
                    self.constraints.push(RegionConstraint::Escape {
                        value: "call_result".to_string(),
                        source: region,
                        destination: escape_region,
                    });
                }
            }
            HirExpr::Binary { left, op: _, right } => {
                self.generate_expr_constraints(left, region.clone())?;
                self.generate_expr_constraints(right, region.clone())?;
            }
            HirExpr::Block(block) => {
                // Create subregion for block
                let subregion = self.create_region("block", false);
                self.constraints.push(RegionConstraint::Subregion {
                    sub: subregion.clone(),
                    super_: region,
                });
                
                self.generate_block_constraints(block, subregion)?;
            }
        }
        Ok(())
    }

    fn is_allocation(&self, expr: &HirExpr) -> bool {
        match expr {
            HirExpr::Call { function, .. } => {
                matches!(function.as_str(), "Vec::new" | "Box::new" | "String::new")
            }
            _ => false,
        }
    }

    fn is_escape_operation(&self, function: &str) -> bool {
        matches!(function, "send" | "spawn" | "return" | "channel_send")
    }

    fn create_region(&mut self, name: &str, is_primary: bool) -> Region {
        let region = Region {
            id: self.next_region_id,
            name: name.to_string(),
            is_primary,
        };
        self.regions.insert(self.next_region_id, region.clone());
        self.next_region_id += 1;
        region
    }

    fn build_region_dag(&mut self) -> Result<RegionDag, Vec<RegionError>> {
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        
        // Create nodes for each region
        for (_id, region) in &self.regions {
            let node = RegionNode {
                region: region.clone(),
                allocations: Vec::new(),
                escapes: Vec::new(),
                last_use: None,
                free_point: None,
            };
            nodes.insert(region.clone(), node);
        }
        
        // Process constraints to build edges
        for constraint in &self.constraints {
            match constraint {
                RegionConstraint::Subregion { sub, super_ } => {
                    edges.push((sub.clone(), super_.clone()));
                }
                RegionConstraint::Equality { left, right } => {
                    // Merge regions
                    if let Some(left_node) = nodes.get_mut(left) {
                        left_node.region = right.clone();
                    }
                }
                RegionConstraint::Allocation { expr_id, region } => {
                    if let Some(node) = nodes.get_mut(region) {
                        node.allocations.push(*expr_id);
                    }
                }
                RegionConstraint::Escape { value, source, destination } => {
                    if let Some(node) = nodes.get_mut(source) {
                        node.escapes.push(value.clone());
                    }
                    edges.push((source.clone(), destination.clone()));
                }
                RegionConstraint::Liveness { region, last_use } => {
                    if let Some(node) = nodes.get_mut(region) {
                        node.last_use = Some(*last_use);
                    }
                }
            }
        }
        
        // Check for cycles in the DAG
        if self.has_cycle(&edges) {
            self.errors.push(RegionError::RegionDagFailed(
                "Circular dependencies detected in region DAG".to_string()
            ));
            return Err(self.errors.clone());
        }
        
        Ok(RegionDag {
            nodes,
            edges,
            free_points: Vec::new(),
        })
    }

    fn has_cycle(&self, edges: &[(Region, Region)]) -> bool {
        let mut graph: HashMap<Region, Vec<Region>> = HashMap::new();
        
        // Build adjacency list
        for (from, to) in edges {
            graph.entry(from.clone()).or_insert_with(Vec::new).push(to.clone());
        }
        
        // DFS to detect cycles
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        
        for region in self.regions.values() {
            if !visited.contains(region) {
                if self.dfs_has_cycle(region, &graph, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        
        false
    }

    fn dfs_has_cycle(
        &self,
        region: &Region,
        graph: &HashMap<Region, Vec<Region>>,
        visited: &mut HashSet<Region>,
        rec_stack: &mut HashSet<Region>,
    ) -> bool {
        visited.insert(region.clone());
        rec_stack.insert(region.clone());
        
        if let Some(neighbors) = graph.get(region) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.dfs_has_cycle(neighbor, graph, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }
        
        rec_stack.remove(region);
        false
    }

    fn place_free_points(&mut self, dag: &mut RegionDag) -> Result<(), Vec<RegionError>> {
        // Topological sort to determine free point placement order
        let sorted_regions = self.topological_sort(&dag.edges)?;
        
        for region in sorted_regions {
            if let Some(node) = dag.nodes.get_mut(&region) {
                // Place free point at the earliest safe location
                let free_point = self.calculate_free_point(node);
                node.free_point = Some(free_point);
                dag.free_points.push((region, free_point));
            }
        }
        
        Ok(())
    }

    fn topological_sort(&mut self, edges: &[(Region, Region)]) -> Result<Vec<Region>, Vec<RegionError>> {
        let mut in_degree: HashMap<Region, usize> = HashMap::new();
        let mut graph: HashMap<Region, Vec<Region>> = HashMap::new();
        
        // Initialize in-degree and build graph
        for region in self.regions.values() {
            in_degree.insert(region.clone(), 0);
        }
        
        for (from, to) in edges {
            graph.entry(from.clone()).or_insert_with(Vec::new).push(to.clone());
            *in_degree.get_mut(to).unwrap() += 1;
        }
        
        // Kahn's algorithm
        let mut queue = VecDeque::new();
        let mut result = Vec::new();
        
        for (region, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(region.clone());
            }
        }
        
        while let Some(region) = queue.pop_front() {
            result.push(region.clone());
            
            if let Some(neighbors) = graph.get(&region) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }
        
        if result.len() != self.regions.len() {
            self.errors.push(RegionError::RegionDagFailed(
                "Topological sort failed - cycle detected".to_string()
            ));
            return Err(self.errors.clone());
        }
        
        Ok(result)
    }

    fn calculate_free_point(&self, node: &RegionNode) -> usize {
        // Calculate the earliest safe point for freeing this region
        // This is a simplified implementation - in practice, this would be more sophisticated
        if let Some(last_use) = node.last_use {
            last_use + 1
        } else {
            0
        }
    }

    /// Suggest fallback regions when inference fails
    pub fn suggest_fallbacks(&self, dag: &RegionDag) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for (region, node) in &dag.nodes {
            if node.allocations.len() > 10 {
                suggestions.push(format!(
                    "Consider using `box` for region {} with {} allocations",
                    region, node.allocations.len()
                ));
            }
            
            if node.escapes.len() > 5 {
                suggestions.push(format!(
                    "Consider using `rc` for region {} with {} escapes",
                    region, node.escapes.len()
                ));
            }
        }
        
        suggestions
    }
}

/// Region inference checker
pub struct RegionChecker {
    solver: RegionSolver,
}

impl RegionChecker {
    pub fn new() -> Self {
        Self {
            solver: RegionSolver::new(),
        }
    }

    pub fn check(&mut self, hir: &HirProgram) -> Result<RegionDag, Vec<RegionError>> {
        self.solver.solve(hir)
    }

    pub fn explain_regions(&self, dag: &RegionDag) -> String {
        let mut explanation = String::new();
        explanation.push_str("Region Inference Results:\n");
        explanation.push_str("=======================\n\n");
        
        for (region, node) in &dag.nodes {
            explanation.push_str(&format!("Region {}:\n", region));
            explanation.push_str(&format!("  Allocations: {}\n", node.allocations.len()));
            explanation.push_str(&format!("  Escapes: {}\n", node.escapes.len()));
            if let Some(free_point) = node.free_point {
                explanation.push_str(&format!("  Free point: {}\n", free_point));
            }
            explanation.push_str("\n");
        }
        
        explanation.push_str("Free Points:\n");
        for (region, point) in &dag.free_points {
            explanation.push_str(&format!("  {} freed at point {}\n", region, point));
        }
        
        explanation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral};

    #[test]
    fn test_region_creation() {
        let mut solver = RegionSolver::new();
        let region = solver.create_region("test", true);
        
        assert_eq!(region.name, "test");
        assert!(region.is_primary);
        assert_eq!(region.id, 0);
    }

    #[test]
    fn test_constraint_generation() {
        let mut solver = RegionSolver::new();
        let region = solver.create_region("test", true);
        
        solver.constraints.push(RegionConstraint::Allocation {
            expr_id: 1,
            region: region.clone(),
        });
        
        assert_eq!(solver.constraints.len(), 1);
    }

    #[test]
    fn test_cycle_detection() {
        let mut solver = RegionSolver::new();
        let r1 = solver.create_region("r1", false);
        let r2 = solver.create_region("r2", false);
        
        let edges = vec![(r1.clone(), r2.clone()), (r2, r1)];
        assert!(solver.has_cycle(&edges));
    }
}