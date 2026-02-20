use anyhow::Result;
use std::path::Path;
use workgraph::trace_function::{
    self, FunctionInput, InputType, TaskTemplate, TraceFunction,
};

/// List all available trace functions.
pub fn run_list(dir: &Path, json: bool, verbose: bool) -> Result<()> {
    let func_dir = trace_function::functions_dir(dir);
    let functions = trace_function::load_all_functions(&func_dir)?;

    if functions.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No trace functions found.");
            println!("  Extract one with: wg trace extract <task-id>");
        }
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&functions)?);
        return Ok(());
    }

    println!("Functions:");
    // Calculate column widths for alignment
    let id_width = functions.iter().map(|f| f.id.len()).max().unwrap_or(4).max(4);
    let name_width = functions
        .iter()
        .map(|f| f.name.len())
        .max()
        .unwrap_or(4)
        .max(4);

    for func in &functions {
        println!(
            "  {:<id_w$}  {:<name_w$}  {} tasks, {} inputs",
            func.id,
            format!("\"{}\"", func.name),
            func.tasks.len(),
            func.inputs.len(),
            id_w = id_width,
            name_w = name_width + 2, // +2 for quotes
        );

        if verbose {
            if !func.inputs.is_empty() {
                println!("    Inputs:");
                for input in &func.inputs {
                    print_input_summary(input, "      ");
                }
            }
            if !func.tasks.is_empty() {
                println!("    Tasks:");
                for template in &func.tasks {
                    print_template_summary(template, "      ");
                }
            }
            println!();
        }
    }

    Ok(())
}

/// Show details of a single trace function.
pub fn run_show(dir: &Path, id: &str, json: bool) -> Result<()> {
    let func_dir = trace_function::functions_dir(dir);
    let func = trace_function::find_function_by_prefix(&func_dir, id)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&func)?);
        return Ok(());
    }

    print_function_details(&func);

    Ok(())
}

fn print_function_details(func: &TraceFunction) {
    println!("Function: {}", func.id);
    println!("Name: {}", func.name);
    if !func.description.is_empty() {
        println!("Description: {}", func.description);
    }
    println!("Version: {}", func.version);

    if !func.tags.is_empty() {
        println!("Tags: {}", func.tags.join(", "));
    }

    // Provenance
    if !func.extracted_from.is_empty() {
        println!();
        println!("Extracted from:");
        for source in &func.extracted_from {
            print!("  - {}", source.task_id);
            if let Some(ref run_id) = source.run_id {
                print!(" ({})", run_id);
            }
            println!(" at {}", source.timestamp);
        }
    }
    if let Some(ref by) = func.extracted_by {
        println!("Extracted by: {}", by);
    }
    if let Some(ref at) = func.extracted_at {
        println!("Extracted at: {}", at);
    }

    // Inputs
    if !func.inputs.is_empty() {
        println!();
        println!("Inputs ({}):", func.inputs.len());
        for input in &func.inputs {
            print_input_detail(input);
        }
    }

    // Task templates
    if !func.tasks.is_empty() {
        println!();
        println!("Tasks ({}):", func.tasks.len());
        for template in &func.tasks {
            print_template_detail(template);
        }
    }

    // Outputs
    if !func.outputs.is_empty() {
        println!();
        println!("Outputs ({}):", func.outputs.len());
        for output in &func.outputs {
            println!(
                "  - {} (from {}.{}): {}",
                output.name, output.from_task, output.field, output.description
            );
        }
    }
}

fn format_input_type(t: &InputType) -> &'static str {
    match t {
        InputType::String => "string",
        InputType::Text => "text",
        InputType::FileList => "file_list",
        InputType::FileContent => "file_content",
        InputType::Number => "number",
        InputType::Url => "url",
        InputType::Enum => "enum",
        InputType::Json => "json",
    }
}

fn print_input_summary(input: &FunctionInput, indent: &str) {
    let required_str = if input.required { ", required" } else { "" };
    println!(
        "{}{} ({}{}): {}",
        indent,
        input.name,
        format_input_type(&input.input_type),
        required_str,
        input.description
    );
}

fn print_input_detail(input: &FunctionInput) {
    let required_str = if input.required {
        "required"
    } else {
        "optional"
    };
    println!(
        "  - {} ({}, {})",
        input.name,
        format_input_type(&input.input_type),
        required_str,
    );
    println!("    {}", input.description);
    if let Some(ref default) = input.default {
        println!("    Default: {}", format_yaml_value(default));
    }
    if let Some(ref example) = input.example {
        println!("    Example: {}", format_yaml_value(example));
    }
    if let Some(ref values) = input.values {
        println!("    Values: {}", values.join(", "));
    }
    if let Some(min) = input.min {
        print!("    Range: [{}", min);
        if let Some(max) = input.max {
            println!(", {}]", max);
        } else {
            println!(", ∞)");
        }
    } else if let Some(max) = input.max {
        println!("    Range: (-∞, {}]", max);
    }
}

fn print_template_summary(template: &TaskTemplate, indent: &str) {
    let deps = if template.blocked_by.is_empty() {
        String::new()
    } else {
        format!(" (blocked by: {})", template.blocked_by.join(", "))
    };
    let loops = if template.loops_to.is_empty() {
        String::new()
    } else {
        let targets: Vec<&str> = template.loops_to.iter().map(|l| l.target.as_str()).collect();
        format!(" (loops to: {})", targets.join(", "))
    };
    println!("{}{}: {}{}{}", indent, template.template_id, template.title, deps, loops);
}

fn print_template_detail(template: &TaskTemplate) {
    println!("  - {} : {}", template.template_id, template.title);

    // Description (indent multiline)
    let desc = template.description.trim();
    if !desc.is_empty() {
        for line in desc.lines() {
            println!("    {}", line);
        }
    }

    if !template.blocked_by.is_empty() {
        println!("    Blocked by: {}", template.blocked_by.join(", "));
    }
    if !template.loops_to.is_empty() {
        for edge in &template.loops_to {
            print!("    Loops to: {} (max {})", edge.target, edge.max_iterations);
            if let Some(ref guard) = edge.guard {
                print!(", guard: {}", guard);
            }
            if let Some(ref delay) = edge.delay {
                print!(", delay: {}", delay);
            }
            println!();
        }
    }
    if !template.skills.is_empty() {
        println!("    Skills: {}", template.skills.join(", "));
    }
    if let Some(ref role) = template.role_hint {
        println!("    Role hint: {}", role);
    }
    if !template.deliverables.is_empty() {
        println!("    Deliverables: {}", template.deliverables.join(", "));
    }
    if let Some(ref verify) = template.verify {
        println!("    Verify: {}", verify);
    }
    if !template.tags.is_empty() {
        println!("    Tags: {}", template.tags.join(", "));
    }
}

fn format_yaml_value(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::Null => "null".to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => format!("\"{}\"", s),
        serde_yaml::Value::Sequence(seq) => {
            let items: Vec<String> = seq.iter().map(format_yaml_value).collect();
            format!("[{}]", items.join(", "))
        }
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Tagged(_) => {
            serde_yaml::to_string(v).unwrap_or_else(|_| "?".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use workgraph::trace_function::*;

    fn sample_function() -> TraceFunction {
        TraceFunction {
            kind: "trace-function".to_string(),
            version: 1,
            id: "impl-feature".to_string(),
            name: "Implement Feature".to_string(),
            description: "Plan, implement, test a new feature".to_string(),
            extracted_from: vec![ExtractionSource {
                task_id: "impl-global-config".to_string(),
                run_id: Some("run-003".to_string()),
                timestamp: "2026-02-18T14:30:00Z".to_string(),
            }],
            extracted_by: Some("scout".to_string()),
            extracted_at: Some("2026-02-19T12:00:00Z".to_string()),
            tags: vec!["implementation".to_string()],
            inputs: vec![
                FunctionInput {
                    name: "feature_name".to_string(),
                    input_type: InputType::String,
                    description: "Short name for the feature".to_string(),
                    required: true,
                    default: None,
                    example: Some(serde_yaml::Value::String("global-config".to_string())),
                    min: None,
                    max: None,
                    values: None,
                },
                FunctionInput {
                    name: "test_command".to_string(),
                    input_type: InputType::String,
                    description: "Command to verify".to_string(),
                    required: false,
                    default: Some(serde_yaml::Value::String("cargo test".to_string())),
                    example: None,
                    min: None,
                    max: None,
                    values: None,
                },
            ],
            tasks: vec![
                TaskTemplate {
                    template_id: "plan".to_string(),
                    title: "Plan {{input.feature_name}}".to_string(),
                    description: "Plan the implementation".to_string(),
                    skills: vec!["analysis".to_string()],
                    blocked_by: vec![],
                    loops_to: vec![],
                    role_hint: Some("analyst".to_string()),
                    deliverables: vec![],
                    verify: None,
                    tags: vec![],
                },
                TaskTemplate {
                    template_id: "implement".to_string(),
                    title: "Implement {{input.feature_name}}".to_string(),
                    description: "Build the feature".to_string(),
                    skills: vec!["implementation".to_string()],
                    blocked_by: vec!["plan".to_string()],
                    loops_to: vec![],
                    role_hint: Some("programmer".to_string()),
                    deliverables: vec![],
                    verify: None,
                    tags: vec![],
                },
                TaskTemplate {
                    template_id: "validate".to_string(),
                    title: "Validate {{input.feature_name}}".to_string(),
                    description: "Review the implementation".to_string(),
                    skills: vec!["review".to_string()],
                    blocked_by: vec!["implement".to_string()],
                    loops_to: vec![],
                    role_hint: None,
                    deliverables: vec![],
                    verify: None,
                    tags: vec![],
                },
            ],
            outputs: vec![FunctionOutput {
                name: "modified_files".to_string(),
                description: "Files changed".to_string(),
                from_task: "implement".to_string(),
                field: "artifacts".to_string(),
            }],
        }
    }

    #[test]
    fn list_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("functions")).unwrap();
        assert!(run_list(dir, false, false).is_ok());
    }

    #[test]
    fn list_empty_json() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("functions")).unwrap();
        assert!(run_list(dir, true, false).is_ok());
    }

    #[test]
    fn list_with_functions() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        assert!(run_list(dir, false, false).is_ok());
    }

    #[test]
    fn list_with_functions_verbose() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        assert!(run_list(dir, false, true).is_ok());
    }

    #[test]
    fn list_with_functions_json() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        assert!(run_list(dir, true, false).is_ok());
    }

    #[test]
    fn show_by_exact_id() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        assert!(run_show(dir, "impl-feature", false).is_ok());
    }

    #[test]
    fn show_by_prefix() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        assert!(run_show(dir, "impl", false).is_ok());
    }

    #[test]
    fn show_json() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        assert!(run_show(dir, "impl-feature", true).is_ok());
    }

    #[test]
    fn show_not_found() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        save_function(&sample_function(), &func_dir).unwrap();
        let result = run_show(dir, "nonexistent", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No function matching"));
    }

    #[test]
    fn show_ambiguous() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");
        let mut f1 = sample_function();
        f1.id = "impl-feature".to_string();
        let mut f2 = sample_function();
        f2.id = "impl-bug".to_string();
        save_function(&f1, &func_dir).unwrap();
        save_function(&f2, &func_dir).unwrap();

        let result = run_show(dir, "impl", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("matches"));
    }

    #[test]
    fn list_multiple_functions() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();
        let func_dir = dir.join("functions");

        let mut f1 = sample_function();
        f1.id = "alpha-func".to_string();
        f1.name = "Alpha Function".to_string();

        let mut f2 = sample_function();
        f2.id = "beta-func".to_string();
        f2.name = "Beta Function".to_string();
        f2.inputs = vec![]; // No inputs
        f2.tasks = vec![]; // No tasks

        save_function(&f1, &func_dir).unwrap();
        save_function(&f2, &func_dir).unwrap();

        assert!(run_list(dir, false, false).is_ok());
        assert!(run_list(dir, true, false).is_ok());
        assert!(run_list(dir, false, true).is_ok());
    }
}
