//! Dependency graph primitive: topological waves + cycle detection (Phase 3).

use std::collections::VecDeque;

/// A dependency cycle was detected; carries a node id stuck in the cycle.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("dependency cycle involving node {0}")]
pub struct CycleError(pub usize);

/// Dependency graph of task nodes; a node's id is its insertion index.
///
/// An edge `(task, dependency)` means `task` waits for `dependency`: the
/// dependency must appear in an earlier execution wave than the task.
#[derive(Default)]
pub struct Dag {
    nodes: Vec<String>,
    edges: Vec<(usize, usize)>,
}

impl Dag {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Append a task node; returns its id (insertion index).
    pub fn add_node(&mut self, task: String) -> usize {
        self.nodes.push(task);
        self.nodes.len() - 1
    }

    /// Record that `task` depends on `dependency`. Out-of-range ids are
    /// ignored so callers can pass through unvalidated plan data.
    pub fn add_edge(&mut self, task: usize, dependency: usize) {
        if task < self.nodes.len() && dependency < self.nodes.len() {
            self.edges.push((task, dependency));
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Waves of node ids runnable in parallel (Kahn's algorithm): every
    /// dependency of a node appears in an earlier wave. On a dependency
    /// cycle, returns [`CycleError`] with a node id stuck in the cycle.
    pub fn topological_waves(&self) -> Result<Vec<Vec<usize>>, CycleError> {
        let mut in_degree = vec![0usize; self.nodes.len()];
        let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        for &(task, dependency) in &self.edges {
            in_degree[task] += 1;
            dependents[dependency].push(task);
        }

        let mut ready: VecDeque<usize> = (0..self.nodes.len())
            .filter(|&node| in_degree[node] == 0)
            .collect();
        let mut waves = Vec::new();
        let mut resolved = 0usize;
        while !ready.is_empty() {
            let wave: Vec<usize> = ready.drain(..).collect();
            resolved += wave.len();
            for &node in &wave {
                for &dependent in &dependents[node] {
                    in_degree[dependent] -= 1;
                    if in_degree[dependent] == 0 {
                        ready.push_back(dependent);
                    }
                }
            }
            waves.push(wave);
        }

        if resolved < self.nodes.len() {
            let stuck = (0..self.nodes.len())
                .find(|&node| in_degree[node] > 0)
                .expect("unresolved nodes remain when waves are incomplete");
            return Err(CycleError(stuck));
        }
        Ok(waves)
    }
}
