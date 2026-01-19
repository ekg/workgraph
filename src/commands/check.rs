use anyhow::{Context, Result};
use std::path::Path;
use workgraph::check::check_all;
use workgraph::parser::load_graph;

use super::graph_path;

pub fn run(dir: &Path) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let graph = load_graph(&path).context("Failed to load graph")?;
    let result = check_all(&graph);

    if result.ok {
        println!("Graph OK: {} nodes, no issues found", graph.len());
        return Ok(());
    }

    let mut issues = 0;

    if !result.cycles.is_empty() {
        println!("Cycles detected:");
        for cycle in &result.cycles {
            println!("  {}", cycle.join(" -> "));
            issues += 1;
        }
    }

    if !result.orphan_refs.is_empty() {
        println!("Orphan references:");
        for orphan in &result.orphan_refs {
            println!("  {} --[{}]--> {} (not found)", orphan.from, orphan.relation, orphan.to);
            issues += 1;
        }
    }

    anyhow::bail!("Found {} issue(s)", issues);
}
