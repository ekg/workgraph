use crate::graph::{Status, Task, WorkGraph};

/// Find all tasks that are ready to work on (no open blockers)
pub fn ready_tasks(graph: &WorkGraph) -> Vec<&Task> {
    graph
        .tasks()
        .filter(|task| {
            // Must be open
            if task.status != Status::Open {
                return false;
            }
            // All blockers must be done
            task.blocked_by.iter().all(|blocker_id| {
                graph
                    .get_task(blocker_id)
                    .map(|t| t.status == Status::Done)
                    .unwrap_or(true) // If blocker doesn't exist, treat as unblocked
            })
        })
        .collect()
}

/// Find what tasks are blocking a given task
pub fn blocked_by<'a>(graph: &'a WorkGraph, task_id: &str) -> Vec<&'a Task> {
    let Some(task) = graph.get_task(task_id) else {
        return vec![];
    };

    task.blocked_by
        .iter()
        .filter_map(|id| graph.get_task(id))
        .filter(|t| t.status != Status::Done)
        .collect()
}

/// Calculate total cost of a task and all its transitive dependencies
pub fn cost_of(graph: &WorkGraph, task_id: &str) -> f64 {
    let mut visited = std::collections::HashSet::new();
    cost_of_recursive(graph, task_id, &mut visited)
}

fn cost_of_recursive(
    graph: &WorkGraph,
    task_id: &str,
    visited: &mut std::collections::HashSet<String>,
) -> f64 {
    if visited.contains(task_id) {
        return 0.0;
    }
    visited.insert(task_id.to_string());

    let Some(task) = graph.get_task(task_id) else {
        return 0.0;
    };

    let self_cost = task
        .estimate
        .as_ref()
        .and_then(|e| e.cost)
        .unwrap_or(0.0);

    let deps_cost: f64 = task
        .blocked_by
        .iter()
        .map(|dep_id| cost_of_recursive(graph, dep_id, visited))
        .sum();

    self_cost + deps_cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Estimate, Node};

    fn make_task(id: &str, title: &str) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            status: Status::Open,
            assigned: None,
            estimate: None,
            blocks: vec![],
            blocked_by: vec![],
            requires: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn test_ready_tasks_empty_graph() {
        let graph = WorkGraph::new();
        let ready = ready_tasks(&graph);
        assert!(ready.is_empty());
    }

    #[test]
    fn test_ready_tasks_single_open_task() {
        let mut graph = WorkGraph::new();
        graph.add_node(Node::Task(make_task("t1", "Task 1")));

        let ready = ready_tasks(&graph);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "t1");
    }

    #[test]
    fn test_ready_tasks_excludes_done() {
        let mut graph = WorkGraph::new();
        let mut task = make_task("t1", "Task 1");
        task.status = Status::Done;
        graph.add_node(Node::Task(task));

        let ready = ready_tasks(&graph);
        assert!(ready.is_empty());
    }

    #[test]
    fn test_ready_tasks_excludes_blocked() {
        let mut graph = WorkGraph::new();

        let blocker = make_task("blocker", "Blocker");
        let mut blocked = make_task("blocked", "Blocked");
        blocked.blocked_by = vec!["blocker".to_string()];

        graph.add_node(Node::Task(blocker));
        graph.add_node(Node::Task(blocked));

        let ready = ready_tasks(&graph);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "blocker");
    }

    #[test]
    fn test_ready_tasks_unblocked_when_blocker_done() {
        let mut graph = WorkGraph::new();

        let mut blocker = make_task("blocker", "Blocker");
        blocker.status = Status::Done;

        let mut blocked = make_task("blocked", "Blocked");
        blocked.blocked_by = vec!["blocker".to_string()];

        graph.add_node(Node::Task(blocker));
        graph.add_node(Node::Task(blocked));

        let ready = ready_tasks(&graph);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, "blocked");
    }

    #[test]
    fn test_blocked_by_returns_blockers() {
        let mut graph = WorkGraph::new();

        let blocker = make_task("blocker", "Blocker");
        let mut blocked = make_task("blocked", "Blocked");
        blocked.blocked_by = vec!["blocker".to_string()];

        graph.add_node(Node::Task(blocker));
        graph.add_node(Node::Task(blocked));

        let blockers = blocked_by(&graph, "blocked");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].id, "blocker");
    }

    #[test]
    fn test_blocked_by_excludes_done_blockers() {
        let mut graph = WorkGraph::new();

        let mut blocker = make_task("blocker", "Blocker");
        blocker.status = Status::Done;

        let mut blocked = make_task("blocked", "Blocked");
        blocked.blocked_by = vec!["blocker".to_string()];

        graph.add_node(Node::Task(blocker));
        graph.add_node(Node::Task(blocked));

        let blockers = blocked_by(&graph, "blocked");
        assert!(blockers.is_empty());
    }

    #[test]
    fn test_cost_of_single_task() {
        let mut graph = WorkGraph::new();
        let mut task = make_task("t1", "Task 1");
        task.estimate = Some(Estimate {
            hours: Some(10.0),
            cost: Some(1000.0),
        });
        graph.add_node(Node::Task(task));

        assert_eq!(cost_of(&graph, "t1"), 1000.0);
    }

    #[test]
    fn test_cost_of_with_dependencies() {
        let mut graph = WorkGraph::new();

        let mut dep = make_task("dep", "Dependency");
        dep.estimate = Some(Estimate {
            hours: None,
            cost: Some(500.0),
        });

        let mut task = make_task("main", "Main task");
        task.blocked_by = vec!["dep".to_string()];
        task.estimate = Some(Estimate {
            hours: None,
            cost: Some(1000.0),
        });

        graph.add_node(Node::Task(dep));
        graph.add_node(Node::Task(task));

        assert_eq!(cost_of(&graph, "main"), 1500.0);
    }

    #[test]
    fn test_cost_of_handles_cycles() {
        let mut graph = WorkGraph::new();

        let mut t1 = make_task("t1", "Task 1");
        t1.blocked_by = vec!["t2".to_string()];
        t1.estimate = Some(Estimate {
            hours: None,
            cost: Some(100.0),
        });

        let mut t2 = make_task("t2", "Task 2");
        t2.blocked_by = vec!["t1".to_string()];
        t2.estimate = Some(Estimate {
            hours: None,
            cost: Some(200.0),
        });

        graph.add_node(Node::Task(t1));
        graph.add_node(Node::Task(t2));

        // Should not infinite loop, should count each once
        let cost = cost_of(&graph, "t1");
        assert_eq!(cost, 300.0);
    }

    #[test]
    fn test_cost_of_nonexistent_task() {
        let graph = WorkGraph::new();
        assert_eq!(cost_of(&graph, "nonexistent"), 0.0);
    }
}
