use anyhow::{Context, Result};
use std::path::Path;
use workgraph::parser::load_graph;
use workgraph::query::ready_tasks;

use super::graph_path;

pub fn run(dir: &Path, json: bool) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let graph = load_graph(&path).context("Failed to load graph")?;
    let ready = ready_tasks(&graph);

    if json {
        let output: Vec<_> = ready
            .iter()
            .map(|t| serde_json::json!({
                "id": t.id,
                "title": t.title,
                "assigned": t.assigned,
                "estimate": t.estimate,
            }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if ready.is_empty() {
            println!("No tasks ready");
        } else {
            println!("Ready tasks:");
            for task in ready {
                let assigned = task
                    .assigned
                    .as_ref()
                    .map(|a| format!(" ({})", a))
                    .unwrap_or_default();
                println!("  {} - {}{}", task.id, task.title, assigned);
            }
        }
    }

    Ok(())
}
