use std::path::{Path, PathBuf};

pub enum ToolPath {
    Overridden(PathBuf),
    Cached {
        bin_path: PathBuf,
        base: PathBuf,
        sub_paths: Vec<PathBuf>,
    },
}

impl ToolPath {
    pub fn is_overridden(&self) -> bool {
        if let ToolPath::Overridden(_) = self {
            true
        } else {
            false
        }
    }

    pub fn bin_path(&self) -> &Path {
        match self {
            ToolPath::Overridden(p) => p,
            ToolPath::Cached { bin_path, .. } => bin_path,
        }
    }

    pub fn cache_paths(&self) -> Option<(&std::path::Path, &Vec<PathBuf>)> {
        match self {
            ToolPath::Cached {
                base, sub_paths, ..
            } => Some((base, sub_paths)),
            _ => None,
        }
    }
}
