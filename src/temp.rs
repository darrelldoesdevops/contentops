use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::Context;

pub struct TempFileRegistry {
    paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl TempFileRegistry {
    pub fn new() -> Self {
        Self {
            paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn register(&self, path: PathBuf) {
        self.paths.lock().unwrap().push(path);
    }

    pub fn remove(&self, path: &Path) {
        self.paths.lock().unwrap().retain(|p| p != path);
    }

    pub fn clone_inner(&self) -> Arc<Mutex<Vec<PathBuf>>> {
        Arc::clone(&self.paths)
    }
}

pub fn register_cleanup(temp_paths: Arc<Mutex<Vec<PathBuf>>>) {
    ctrlc::set_handler(move || {
        let paths = temp_paths.lock().unwrap();
        for path in paths.iter() {
            let _ = std::fs::remove_file(path);
        }
        std::process::exit(1);
    })
    .expect("failed to set Ctrl-C handler");
}

pub fn make_temp_file(dir: &Path, ext: &str) -> anyhow::Result<tempfile::NamedTempFile> {
    tempfile::Builder::new()
        .prefix(".contentops_tmp_")
        .suffix(ext)
        .tempfile_in(dir)
        .with_context(|| format!("creating temp file in {}", dir.display()))
}
