use anyhow::{Context, Result};
use std::path::Path;
use workgraph::graph::Status;
use workgraph::parser::load_graph;

use super::graph_path;

pub fn run(dir: &Path) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let graph = load_graph(&path).context("Failed to load graph")?;

    // Print DOT format for visualization
    println!("digraph workgraph {{");
    println!("  rankdir=LR;");
    println!("  node [shape=box];");
    println!();

    // Print nodes
    for task in graph.tasks() {
        let style = match task.status {
            Status::Done => "style=filled, fillcolor=lightgreen",
            Status::InProgress => "style=filled, fillcolor=lightyellow",
            Status::Blocked => "style=filled, fillcolor=lightcoral",
            Status::Open => "style=filled, fillcolor=white",
            Status::Failed => "style=filled, fillcolor=salmon",
            Status::Abandoned => "style=filled, fillcolor=lightgray",
        };
        println!("  \"{}\" [label=\"{}\\n{}\", {}];", task.id, task.id, task.title, style);
    }

    for actor in graph.actors() {
        let name = actor.name.as_deref().unwrap_or(&actor.id);
        println!("  \"{}\" [label=\"{}\", shape=ellipse, style=filled, fillcolor=lightblue];", actor.id, name);
    }

    for resource in graph.resources() {
        let name = resource.name.as_deref().unwrap_or(&resource.id);
        println!("  \"{}\" [label=\"{}\", shape=diamond, style=filled, fillcolor=lightyellow];", resource.id, name);
    }

    println!();

    // Print edges
    for task in graph.tasks() {
        for blocked in &task.blocked_by {
            println!("  \"{}\" -> \"{}\" [label=\"blocks\"];", blocked, task.id);
        }
        if let Some(ref assigned) = task.assigned {
            println!("  \"{}\" -> \"{}\" [style=dashed, label=\"assigned\"];", task.id, assigned);
        }
        for req in &task.requires {
            println!("  \"{}\" -> \"{}\" [style=dotted, label=\"requires\"];", task.id, req);
        }
    }

    println!("}}");

    Ok(())
}
