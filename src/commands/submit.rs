//! Submit command - DEPRECATED, behaves like `wg done`.
//!
//! The pending-review status has been removed. Use `wg done` instead.

use anyhow::Result;
use std::path::Path;

pub fn run(dir: &Path, task_id: &str, _actor: Option<&str>) -> Result<()> {
    eprintln!(
        "Warning: 'wg submit' is deprecated and will be removed in a future release. Use 'wg done' instead."
    );
    super::done::run(dir, task_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use workgraph::graph::{Node, Status, Task, WorkGraph};
    use workgraph::parser::{load_graph, save_graph};

    use crate::commands::graph_path;

    fn make_task(id: &str, title: &str, status: Status) -> Task {
        Task {
            id: id.to_string(),
            title: title.to_string(),
            description: None,
            status,
            assigned: None,
            estimate: None,
            blocks: vec![],
            blocked_by: vec![],
            requires: vec![],
            tags: vec![],
            skills: vec![],
            inputs: vec![],
            deliverables: vec![],
            artifacts: vec![],
            exec: None,
            not_before: None,
            created_at: None,
            started_at: None,
            completed_at: None,
            log: vec![],
            retry_count: 0,
            max_retries: None,
            failure_reason: None,
            model: None,
            verify: None,
            agent: None,
            loops_to: vec![],
            loop_iteration: 0,
            ready_after: None,
        }
    }

    fn setup_workgraph(dir: &Path, tasks: Vec<Task>) -> std::path::PathBuf {
        fs::create_dir_all(dir).unwrap();
        let path = graph_path(dir);
        let mut graph = WorkGraph::new();
        for task in tasks {
            graph.add_node(Node::Task(task));
        }
        save_graph(&graph, &path).unwrap();
        path
    }

    #[test]
    fn test_submit_delegates_to_done() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join(".workgraph");
        let task = make_task("t1", "Test task", Status::InProgress);
        setup_workgraph(&dir, vec![task]);

        run(&dir, "t1", Some("agent-1")).unwrap();

        let graph = load_graph(graph_path(&dir)).unwrap();
        let t = graph.get_task("t1").unwrap();
        assert_eq!(t.status, Status::Done);
    }

    #[test]
    fn test_submit_works_for_verified_tasks() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join(".workgraph");
        let mut task = make_task("t1", "Verified task", Status::InProgress);
        task.verify = Some("Check output".to_string());
        setup_workgraph(&dir, vec![task]);

        // Submit should work even for verified tasks (no more gating)
        run(&dir, "t1", None).unwrap();

        let graph = load_graph(graph_path(&dir)).unwrap();
        let t = graph.get_task("t1").unwrap();
        assert_eq!(t.status, Status::Done);
    }
}
