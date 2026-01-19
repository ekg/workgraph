use anyhow::{Context, Result};
use std::path::Path;
use workgraph::parser::load_graph;
use workgraph::query::cost_of;

use super::graph_path;

pub fn run(dir: &Path, id: &str) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let graph = load_graph(&path).context("Failed to load graph")?;

    if graph.get_task(id).is_none() {
        anyhow::bail!("Task '{}' not found", id);
    }

    let total_cost = cost_of(&graph, id);

    println!("Total cost for '{}' (including dependencies): ${:.2}", id, total_cost);

    Ok(())
}
