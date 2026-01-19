use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn run(dir: &Path) -> Result<()> {
    if dir.exists() {
        anyhow::bail!("Workgraph already initialized at {}", dir.display());
    }

    fs::create_dir_all(dir).context("Failed to create workgraph directory")?;

    let graph_path = dir.join("graph.jsonl");
    fs::write(&graph_path, "").context("Failed to create graph.jsonl")?;

    println!("Initialized workgraph at {}", dir.display());
    Ok(())
}
