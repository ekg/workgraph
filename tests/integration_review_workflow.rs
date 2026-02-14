//! Integration tests for the review workflow (submit, approve, reject).
//!
//! Tests the complete lifecycle of tasks that require verification:
//! - Submitting work (InProgress -> Done, delegates to `wg done`)
//! - Approving work (any -> Done, delegates to `wg done`)
//! - Rejecting work (Done or InProgress -> Open with retry_count incremented)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;
use workgraph::graph::{Node, Status, Task, WorkGraph};
use workgraph::parser::{load_graph, save_graph};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the path to the compiled `wg` binary
fn wg_binary() -> PathBuf {
    let mut path = std::env::current_exe().expect("could not get current exe path");
    path.pop(); // remove the binary name
    if path.ends_with("deps") {
        path.pop(); // remove deps/
    }
    path.push("wg");
    assert!(
        path.exists(),
        "wg binary not found at {:?}. Run `cargo build` first.",
        path
    );
    path
}

/// Run `wg` with given args in a specific workgraph directory
fn wg_cmd(wg_dir: &Path, args: &[&str]) -> std::process::Output {
    let wg = wg_binary();
    Command::new(&wg)
        .arg("--dir")
        .arg(wg_dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("Failed to run wg {:?}: {}", args, e))
}

/// Run `wg` and assert success, returning stdout as string
fn wg_ok(wg_dir: &Path, args: &[&str]) -> String {
    let output = wg_cmd(wg_dir, args);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "wg {:?} failed.\nstdout: {}\nstderr: {}",
        args,
        stdout,
        stderr
    );
    stdout
}

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

fn setup_workgraph(tmp: &TempDir) -> std::path::PathBuf {
    let wg_dir = tmp.path().join(".workgraph");
    fs::create_dir_all(&wg_dir).unwrap();
    let graph_path = wg_dir.join("graph.jsonl");
    let graph = WorkGraph::new();
    save_graph(&graph, &graph_path).unwrap();
    wg_dir
}

// ===========================================================================
// Submit workflow tests
// ===========================================================================

#[test]
fn test_submit_delegates_to_done_for_open_task() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create a task that's Open
    let mut graph = WorkGraph::new();
    let mut task = make_task("task-1", "Test Task", Status::Open);
    task.verify = Some("Must be perfect".to_string());
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Submit now delegates to done, so Open tasks succeed
    let output = wg_cmd(&wg_dir, &["submit", "task-1"]);
    assert!(output.status.success());

    // Verify task is Done
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Done);
}

#[test]
fn test_submit_transitions_to_done() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create an InProgress task
    let mut graph = WorkGraph::new();
    let mut task = make_task("task-1", "Test Task", Status::InProgress);
    task.assigned = Some("agent-1".to_string());
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Submit the task (now delegates to done)
    wg_ok(&wg_dir, &["submit", "task-1"]);

    // Verify status changed to Done
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Done);

    // Verify log entry was added
    assert!(!task.log.is_empty());
    assert!(task.log.last().unwrap().message.contains("done"));
}

#[test]
fn test_submit_checks_blockers() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create two tasks: one blocker (Open), one blocked (InProgress)
    let mut graph = WorkGraph::new();
    graph.add_node(Node::Task(make_task(
        "blocker",
        "Blocker Task",
        Status::Open,
    )));

    let mut task = make_task("task-1", "Test Task", Status::InProgress);
    task.blocked_by = vec!["blocker".to_string()];
    task.verify = Some("Must be perfect".to_string());
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Try to submit - should fail because of unresolved blocker
    let output = wg_cmd(&wg_dir, &["submit", "task-1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("blocked by"));
}

// ===========================================================================
// Reject workflow tests
// ===========================================================================

#[test]
fn test_reject_requires_done_or_in_progress_status() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create a task that's Open (not Done or InProgress)
    let mut graph = WorkGraph::new();
    graph.add_node(Node::Task(make_task("task-1", "Test Task", Status::Open)));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Try to reject - should fail because status is not Done or InProgress
    let output = wg_cmd(
        &wg_dir,
        &["reject", "task-1", "--reason", "Not good enough"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Done or InProgress"));
}

#[test]
fn test_reject_transitions_back_to_open() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create a Done task
    let mut graph = WorkGraph::new();
    let mut task = make_task("task-1", "Test Task", Status::Done);
    task.verify = Some("Must be perfect".to_string());
    task.assigned = Some("agent-1".to_string());
    task.retry_count = 0;
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Reject the task
    wg_ok(
        &wg_dir,
        &["reject", "task-1", "--reason", "Not perfect enough"],
    );

    // Verify status changed to Open
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Open);

    // Verify assigned was cleared
    assert_eq!(task.assigned, None);

    // Verify retry_count was incremented
    assert_eq!(task.retry_count, 1);

    // Verify log entry was added with reason
    assert!(!task.log.is_empty());
    let log_msg = &task.log.last().unwrap().message;
    assert!(log_msg.contains("rejected"));
    assert!(log_msg.contains("Not perfect enough"));
}

#[test]
fn test_reject_without_reason() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create a Done task
    let mut graph = WorkGraph::new();
    let task = make_task("task-1", "Test Task", Status::Done);
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Reject without a reason
    wg_ok(&wg_dir, &["reject", "task-1"]);

    // Verify log message indicates no reason given
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    let log_msg = &task.log.last().unwrap().message;
    assert!(log_msg.contains("no reason given"));
}

#[test]
fn test_reject_increments_retry_count() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create a Done task with existing retry_count
    let mut graph = WorkGraph::new();
    let mut task = make_task("task-1", "Test Task", Status::Done);
    task.retry_count = 2;
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Reject the task
    wg_ok(&wg_dir, &["reject", "task-1", "--reason", "Try again"]);

    // Verify retry_count was incremented from 2 to 3
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.retry_count, 3);
}

// ===========================================================================
// Approve workflow tests
// ===========================================================================

#[test]
fn test_approve_transitions_in_progress_to_done() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create a task that's InProgress
    let mut graph = WorkGraph::new();
    graph.add_node(Node::Task(make_task(
        "task-1",
        "Test Task",
        Status::InProgress,
    )));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Approve now delegates to done, so InProgress -> Done succeeds
    wg_ok(&wg_dir, &["approve", "task-1"]);

    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Done);
}

#[test]
fn test_approve_transitions_to_done() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create an InProgress task
    let mut graph = WorkGraph::new();
    let mut task = make_task("task-1", "Test Task", Status::InProgress);
    task.verify = Some("Must be perfect".to_string());
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Approve the task (delegates to done)
    wg_ok(&wg_dir, &["approve", "task-1"]);

    // Verify status changed to Done
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Done);

    // Verify completed_at was set
    assert!(task.completed_at.is_some());

    // Verify log entry was added
    assert!(!task.log.is_empty());
    assert!(task.log.last().unwrap().message.contains("done"));
}

#[test]
fn test_approve_checks_blockers() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Create two tasks: one blocker (Open), one blocked (InProgress)
    let mut graph = WorkGraph::new();
    graph.add_node(Node::Task(make_task(
        "blocker",
        "Blocker Task",
        Status::Open,
    )));

    let mut task = make_task("task-1", "Test Task", Status::InProgress);
    task.blocked_by = vec!["blocker".to_string()];
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Try to approve - should fail because of unresolved blocker
    let output = wg_cmd(&wg_dir, &["approve", "task-1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("blocked by"));
}

// ===========================================================================
// Complete workflow integration test
// ===========================================================================

#[test]
fn test_complete_review_cycle_with_rejection() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    // Step 1: Create an InProgress task with verification requirement
    let mut graph = WorkGraph::new();
    let mut task = make_task("task-1", "Important Feature", Status::InProgress);
    task.verify = Some("Code must be well-tested and documented".to_string());
    task.assigned = Some("agent-1".to_string());
    graph.add_node(Node::Task(task));

    let graph_path = wg_dir.join("graph.jsonl");
    save_graph(&graph, &graph_path).unwrap();

    // Step 2: Agent submits work (now transitions to Done)
    wg_ok(&wg_dir, &["submit", "task-1"]);

    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Done);
    assert_eq!(task.retry_count, 0);

    // Step 3: Reviewer rejects the work (Done -> Open)
    wg_ok(
        &wg_dir,
        &["reject", "task-1", "--reason", "Tests are insufficient"],
    );

    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Open);
    assert_eq!(task.assigned, None);
    assert_eq!(task.retry_count, 1);

    // Step 4: Agent claims and works on it again
    // Simulate claiming and moving to InProgress
    let mut graph = load_graph(&graph_path).unwrap();
    let task_mut = graph.get_task_mut("task-1").unwrap();
    task_mut.status = Status::InProgress;
    task_mut.assigned = Some("agent-1".to_string());
    save_graph(&graph, &graph_path).unwrap();

    // Step 5: Agent submits again (transitions to Done)
    wg_ok(&wg_dir, &["submit", "task-1"]);

    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.status, Status::Done);
    assert_eq!(task.retry_count, 1); // Still 1, submit doesn't increment
    assert!(task.completed_at.is_some());

    // Verify the complete log trail
    assert!(task.log.len() >= 3); // submit (done), reject, submit (done)
    let messages: Vec<&str> = task.log.iter().map(|e| e.message.as_str()).collect();
    assert!(messages.iter().any(|m| m.contains("done")));
    assert!(messages.iter().any(|m| m.contains("rejected")));
}

#[test]
fn test_multiple_rejections_increment_retry_count() {
    let tmp = TempDir::new().unwrap();
    let wg_dir = setup_workgraph(&tmp);

    let graph_path = wg_dir.join("graph.jsonl");

    // Initial task
    let mut graph = WorkGraph::new();
    let task = make_task("task-1", "Perfectionist Task", Status::InProgress);
    graph.add_node(Node::Task(task));
    save_graph(&graph, &graph_path).unwrap();

    // Submit (-> Done), reject (-> Open), update to InProgress cycle - three times
    for i in 0..3 {
        // Submit (transitions to Done)
        wg_ok(&wg_dir, &["submit", "task-1"]);

        // Reject (Done -> Open)
        let reason = format!("Not perfect enough, attempt {}", i + 1);
        wg_ok(&wg_dir, &["reject", "task-1", "--reason", &reason]);

        // Verify retry_count incremented
        let loaded = load_graph(&graph_path).unwrap();
        let task = loaded.get_task("task-1").unwrap();
        assert_eq!(task.retry_count, i + 1);
        assert_eq!(task.status, Status::Open);

        // Move back to InProgress for next cycle (if not last iteration)
        if i < 2 {
            let mut graph = load_graph(&graph_path).unwrap();
            let task_mut = graph.get_task_mut("task-1").unwrap();
            task_mut.status = Status::InProgress;
            save_graph(&graph, &graph_path).unwrap();
        }
    }

    // Final state: rejected 3 times, retry_count = 3
    let loaded = load_graph(&graph_path).unwrap();
    let task = loaded.get_task("task-1").unwrap();
    assert_eq!(task.retry_count, 3);
}
