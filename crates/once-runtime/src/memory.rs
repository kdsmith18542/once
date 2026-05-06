use crate::value::RuntimeError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub name: String,
    pub size: usize,
    pub allocations: Vec<usize>,
    pub free_point: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub id: usize,
    pub size: usize,
    pub region: String,
    pub is_freed: bool,
}

pub struct MemoryManager {
    pub regions: HashMap<String, RegionInfo>,
    pub allocations: HashMap<usize, AllocationInfo>,
    pub next_allocation_id: usize,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            regions: HashMap::new(),
            allocations: HashMap::new(),
            next_allocation_id: 0,
        }
    }

    pub fn allocate(&mut self, size: usize, region: String) -> usize {
        let allocation_id = self.next_allocation_id;
        self.next_allocation_id += 1;

        let allocation = AllocationInfo {
            id: allocation_id,
            size,
            region: region.clone(),
            is_freed: false,
        };

        self.allocations.insert(allocation_id, allocation);

        if let Some(region_info) = self.regions.get_mut(&region) {
            region_info.allocations.push(allocation_id);
        } else {
            self.regions.insert(
                region.clone(),
                RegionInfo {
                    name: region.clone(),
                    size: 0,
                    allocations: vec![allocation_id],
                    free_point: None,
                },
            );
        }

        allocation_id
    }

    pub fn free(&mut self, allocation_id: usize) -> Result<(), RuntimeError> {
        if let Some(allocation) = self.allocations.get_mut(&allocation_id) {
            if allocation.is_freed {
                return Err(RuntimeError::MemoryError("Double free detected".to_string()));
            }
            allocation.is_freed = true;
            Ok(())
        } else {
            Err(RuntimeError::MemoryError("Allocation not found".to_string()))
        }
    }

    pub fn free_region(&mut self, region_name: &str) -> Result<(), RuntimeError> {
        if let Some(region_info) = self.regions.get(region_name) {
            let allocations = region_info.allocations.clone();
            for allocation_id in allocations {
                self.free(allocation_id)?;
            }
            Ok(())
        } else {
            Err(RuntimeError::MemoryError("Region not found".to_string()))
        }
    }
}
