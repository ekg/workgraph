use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;
use workgraph::parser::{load_graph, save_graph};

use super::graph_path;

/// Update an actor's last_seen timestamp (heartbeat)
pub fn run(dir: &Path, actor_id: &str) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let mut graph = load_graph(&path).context("Failed to load graph")?;

    let actor = graph
        .get_actor_mut(actor_id)
        .ok_or_else(|| anyhow::anyhow!("Actor '{}' not found", actor_id))?;

    let now = Utc::now().to_rfc3339();
    actor.last_seen = Some(now.clone());

    save_graph(&graph, &path).context("Failed to save graph")?;

    println!("Heartbeat recorded for '{}' at {}", actor_id, now);
    Ok(())
}

/// Check for stale actors (no heartbeat within threshold)
pub fn run_check(dir: &Path, threshold_minutes: u64, json: bool) -> Result<()> {
    let path = graph_path(dir);

    if !path.exists() {
        anyhow::bail!("Workgraph not initialized. Run 'wg init' first.");
    }

    let graph = load_graph(&path).context("Failed to load graph")?;

    let now = Utc::now();
    let threshold = chrono::Duration::minutes(threshold_minutes as i64);

    let mut stale_actors = Vec::new();
    let mut active_actors = Vec::new();

    for actor in graph.actors() {
        if let Some(ref last_seen_str) = actor.last_seen {
            if let Ok(last_seen) = chrono::DateTime::parse_from_rfc3339(last_seen_str) {
                let elapsed = now.signed_duration_since(last_seen);
                if elapsed > threshold {
                    stale_actors.push((actor.id.clone(), last_seen_str.clone(), elapsed.num_minutes()));
                } else {
                    active_actors.push((actor.id.clone(), last_seen_str.clone(), elapsed.num_minutes()));
                }
            }
        } else {
            // Never seen - considered stale
            stale_actors.push((actor.id.clone(), "never".to_string(), -1));
        }
    }

    if json {
        let output = serde_json::json!({
            "threshold_minutes": threshold_minutes,
            "stale": stale_actors.iter().map(|(id, last_seen, mins)| {
                serde_json::json!({
                    "id": id,
                    "last_seen": last_seen,
                    "minutes_ago": mins,
                })
            }).collect::<Vec<_>>(),
            "active": active_actors.iter().map(|(id, last_seen, mins)| {
                serde_json::json!({
                    "id": id,
                    "last_seen": last_seen,
                    "minutes_ago": mins,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Heartbeat status (threshold: {} minutes):", threshold_minutes);
        println!();

        if !active_actors.is_empty() {
            println!("Active actors:");
            for (id, _, mins) in &active_actors {
                println!("  {} (seen {} min ago)", id, mins);
            }
        }

        if !stale_actors.is_empty() {
            println!();
            println!("Stale actors (may be dead):");
            for (id, last_seen, mins) in &stale_actors {
                if *mins < 0 {
                    println!("  {} (never seen)", id);
                } else {
                    println!("  {} (last seen {} min ago: {})", id, mins, last_seen);
                }
            }
        }

        if active_actors.is_empty() && stale_actors.is_empty() {
            println!("No actors registered.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use workgraph::graph::{Actor, Node, TrustLevel, WorkGraph};
    use workgraph::parser::save_graph;

    fn setup_with_actor() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("graph.jsonl");

        let mut graph = WorkGraph::new();
        let actor = Actor {
            id: "test-agent".to_string(),
            name: Some("Test Agent".to_string()),
            role: Some("agent".to_string()),
            rate: None,
            capacity: None,
            capabilities: vec!["rust".to_string()],
            context_limit: Some(100000),
            trust_level: TrustLevel::Provisional,
            last_seen: None,
        };
        graph.add_node(Node::Actor(actor));
        save_graph(&graph, &path).unwrap();

        temp_dir
    }

    #[test]
    fn test_heartbeat() {
        let temp_dir = setup_with_actor();

        let result = run(temp_dir.path(), "test-agent");
        assert!(result.is_ok());

        // Verify last_seen was updated
        let graph = load_graph(&graph_path(temp_dir.path())).unwrap();
        let actor = graph.get_actor("test-agent").unwrap();
        assert!(actor.last_seen.is_some());
    }

    #[test]
    fn test_heartbeat_unknown_actor() {
        let temp_dir = setup_with_actor();

        let result = run(temp_dir.path(), "unknown-agent");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_stale() {
        let temp_dir = setup_with_actor();

        // Actor has no heartbeat yet, should be stale
        let result = run_check(temp_dir.path(), 5, false);
        assert!(result.is_ok());
    }
}
