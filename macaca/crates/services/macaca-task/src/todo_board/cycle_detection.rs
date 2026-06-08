//! Dependency graph cycle detection for task batches.
//!
//! Pure Specification helper used before admitting a new task graph slice.
//! Detects cycles among `depends_on` edges that reference tasks in the same batch.

use std::collections::{HashMap, HashSet};

use macaca_proto::{TaskId, TodoItem};

pub fn detect_cycles(tasks: &[TodoItem]) -> Result<(), String> {
    let id_set: HashSet<TaskId> = tasks.iter().map(|t| t.id).collect();
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();

    fn dfs(
        node: TaskId,
        deps: &HashMap<TaskId, Vec<TaskId>>,
        visited: &mut HashSet<TaskId>,
        in_stack: &mut HashSet<TaskId>,
    ) -> Result<(), String> {
        visited.insert(node);
        in_stack.insert(node);

        if let Some(neighbors) = deps.get(&node) {
            for &dep in neighbors {
                if in_stack.contains(&dep) {
                    return Err(format!("Cycle detected involving task {}", dep));
                }
                if !visited.contains(&dep) {
                    dfs(dep, deps, visited, in_stack)?;
                }
            }
        }

        in_stack.remove(&node);
        Ok(())
    }

    // Build adjacency: task -> its dependencies (only within this batch)
    let deps_map: HashMap<TaskId, Vec<TaskId>> = tasks
        .iter()
        .map(|t| {
            let relevant_deps: Vec<TaskId> = t
                .depends_on
                .iter()
                .filter(|d| id_set.contains(d))
                .copied()
                .collect();
            (t.id, relevant_deps)
        })
        .collect();

    for task in tasks {
        if !visited.contains(&task.id) {
            dfs(task.id, &deps_map, &mut visited, &mut in_stack)?;
        }
    }
    Ok(())
}
