use anyhow::{Context, Result};
use std::path::Path;
use workgraph::parser::load_graph;
use workgraph::query::blocked_by;

use super::graph_path;

pub fn run(dir: &Path, id: &str, json: bool) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let graph = load_graph(&path).context("Failed to load graph")?;

    if graph.get_task(id).is_none() {
        anyhow::bail!("Task '{}' not found", id);
    }

    let blockers = blocked_by(&graph, id);

    if json {
        let output: Vec<_> = blockers
            .iter()
            .map(|t| serde_json::json!({
                "id": t.id,
                "title": t.title,
                "status": t.status,
            }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if blockers.is_empty() {
            println!("Task '{}' has no blockers", id);
        } else {
            println!("Task '{}' is blocked by:", id);
            for blocker in blockers {
                println!("  {} - {} [{:?}]", blocker.id, blocker.title, blocker.status);
            }
        }
    }

    Ok(())
}
