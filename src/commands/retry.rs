use anyhow::{Context, Result};
use std::path::Path;
use workgraph::graph::Status;
use workgraph::parser::{load_graph, save_graph};

use super::graph_path;

pub fn run(dir: &Path, id: &str) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let mut graph = load_graph(&path).context("Failed to load graph")?;

    let task = graph
        .get_task_mut(id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;

    if task.status != Status::Failed {
        anyhow::bail!(
            "Task '{}' is not failed (status: {:?}). Only failed tasks can be retried.",
            id,
            task.status
        );
    }

    // Check if max retries exceeded
    if let Some(max) = task.max_retries {
        if task.retry_count >= max {
            anyhow::bail!(
                "Task '{}' has reached max retries ({}/{}). Consider abandoning or increasing max_retries.",
                id,
                task.retry_count,
                max
            );
        }
    }

    task.status = Status::Open;
    // Keep retry_count for history - don't reset it
    // Clear failure_reason since we're retrying
    task.failure_reason = None;

    // Extract values we need for printing before saving
    let retry_count = task.retry_count;
    let max_retries = task.max_retries;

    save_graph(&graph, &path).context("Failed to save graph")?;

    println!("Reset '{}' to open for retry (attempt #{})", id, retry_count + 1);

    if let Some(max) = max_retries {
        println!("  Retries remaining after this: {}", max - retry_count);
    }

    Ok(())
}
