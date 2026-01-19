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

    let mut warnings = 0;
    let mut errors = 0;

    // Cycles are warnings (allowed for recurring tasks)
    if !result.cycles.is_empty() {
        println!("Warning: Cycles detected (this is OK for recurring tasks):");
        for cycle in &result.cycles {
            println!("  {}", cycle.join(" -> "));
            warnings += 1;
        }
    }

    // Orphan references are errors
    if !result.orphan_refs.is_empty() {
        println!("Error: Orphan references:");
        for orphan in &result.orphan_refs {
            println!("  {} --[{}]--> {} (not found)", orphan.from, orphan.relation, orphan.to);
            errors += 1;
        }
    }

    if errors > 0 {
        anyhow::bail!("Found {} error(s) and {} warning(s)", errors, warnings);
    } else if warnings > 0 {
        println!("Graph OK: {} nodes, {} warning(s)", graph.len(), warnings);
    } else {
        println!("Graph OK: {} nodes, no issues found", graph.len());
    }

    Ok(())
}
