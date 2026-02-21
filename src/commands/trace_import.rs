use anyhow::{Context, Result};
use std::path::Path;

use workgraph::agency;
use workgraph::provenance;

use super::trace_export::TraceExport;

pub fn run(
    dir: &Path,
    file: &str,
    source: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    // Read and deserialize the export file
    let contents =
        std::fs::read_to_string(file).with_context(|| format!("Failed to read '{}'", file))?;
    let export: TraceExport = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse '{}' as trace export", file))?;

    // Determine source tag
    let source_tag = source
        .map(String::from)
        .or_else(|| export.metadata.source.clone())
        .unwrap_or_else(|| {
            Path::new(file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

    let task_count = export.tasks.len();
    let eval_count = export.evaluations.len();
    let op_count = export.operations.len();

    if dry_run {
        println!("=== Dry Run: wg trace import ===");
        println!("File:        {}", file);
        println!("Source:      {}", source_tag);
        println!("Visibility:  {}", export.metadata.visibility);
        println!("Tasks:       {}", task_count);
        println!("Evaluations: {}", eval_count);
        println!("Operations:  {}", op_count);

        if !export.tasks.is_empty() {
            println!("\nTasks to import:");
            for task in &export.tasks {
                println!(
                    "  imported/{}/{} - {} ({:?})",
                    source_tag, task.id, task.title, task.status
                );
            }
        }

        if json {
            let out = serde_json::json!({
                "dry_run": true,
                "source": source_tag,
                "task_count": task_count,
                "evaluation_count": eval_count,
                "operation_count": op_count,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        return Ok(());
    }

    // Create import directory
    let import_dir = dir.join("imports").join(&source_tag);
    std::fs::create_dir_all(&import_dir)
        .with_context(|| format!("Failed to create import dir: {}", import_dir.display()))?;

    // Import tasks as namespaced YAML
    let tasks_path = import_dir.join("tasks.yaml");
    let imported_tasks: Vec<ImportedTask> = export
        .tasks
        .iter()
        .map(|t| ImportedTask {
            id: format!("imported/{}/{}", source_tag, t.id),
            original_id: t.id.clone(),
            title: t.title.clone(),
            description: t.description.clone(),
            status: "Done".to_string(),
            visibility: "internal".to_string(),
            skills: t.skills.clone(),
            tags: {
                let mut tags = t.tags.clone();
                tags.push("imported".to_string());
                tags.push(format!("source:{}", source_tag));
                tags
            },
            artifacts: t.artifacts.clone(),
            created_at: t.created_at.clone(),
            completed_at: t.completed_at.clone(),
            agent: t.agent.clone(),
            source: source_tag.clone(),
        })
        .collect();

    let tasks_yaml = serde_yaml::to_string(&imported_tasks)
        .context("Failed to serialize imported tasks")?;
    std::fs::write(&tasks_path, tasks_yaml)
        .with_context(|| format!("Failed to write {}", tasks_path.display()))?;

    // Import evaluations with prefix and modified source
    if !export.evaluations.is_empty() {
        let agency_dir = dir.join("agency");
        agency::init(&agency_dir)?;
        let evals_dir = agency_dir.join("evaluations");

        for eval in &export.evaluations {
            let mut imported_eval = eval.clone();
            imported_eval.id = format!("imported-{}", eval.id);
            imported_eval.source = format!("import:{}", eval.source);
            // Save directly without propagating to performance records
            agency::save_evaluation(&imported_eval, &evals_dir)
                .with_context(|| format!("Failed to save imported evaluation {}", imported_eval.id))?;
        }
    }

    // Import operations to separate log
    if !export.operations.is_empty() {
        let ops_path = import_dir.join("operations.jsonl");
        let mut lines = String::new();
        for op in &export.operations {
            let line = serde_json::to_string(op)?;
            lines.push_str(&line);
            lines.push('\n');
        }
        std::fs::write(&ops_path, lines)
            .with_context(|| format!("Failed to write {}", ops_path.display()))?;
    }

    // Record provenance
    let _ = provenance::record(
        dir,
        "trace_import",
        None,
        Some("user"),
        serde_json::json!({
            "source": source_tag,
            "file": file,
            "task_count": task_count,
            "evaluation_count": eval_count,
            "operation_count": op_count,
        }),
        provenance::DEFAULT_ROTATION_THRESHOLD,
    );

    // Output result
    if json {
        let out = serde_json::json!({
            "source": source_tag,
            "import_dir": import_dir.display().to_string(),
            "task_count": task_count,
            "evaluation_count": eval_count,
            "operation_count": op_count,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Imported {} tasks, {} evaluations, {} operations from '{}'",
            task_count, eval_count, op_count, source_tag);
        println!("Import directory: {}", import_dir.display());
    }

    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ImportedTask {
    id: String,
    original_id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    status: String,
    visibility: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    artifacts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,
    source: String,
}
