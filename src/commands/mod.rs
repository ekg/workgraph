pub mod init;
pub mod add;
pub mod done;
pub mod ready;
pub mod blocked;
pub mod check;
pub mod list;
pub mod graph;
pub mod cost;

use std::path::Path;

pub fn graph_path(dir: &Path) -> std::path::PathBuf {
    dir.join("graph.jsonl")
}
