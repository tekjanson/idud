use std::fmt;
use std::path::{Path, PathBuf};

use blake3;

/// A structural pointer that is stable across whitespace or comment-only edits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphPointer {
    /// A deterministic hash derived from the node's structural identity.
    pub hash: String,
    /// The source path that owns the node.
    pub source_path: PathBuf,
    /// The semantic kind of the node.
    pub kind: String,
    /// A human-readable label that can be used in CLI output.
    pub label: String,
}

/// The semantic categories that can be represented by a graph pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphPointerKind {
    File,
    Module,
    Item,
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Use,
    Other,
}

impl GraphPointer {
    /// Create a pointer from a structural digest.
    pub fn from_digest(path: impl AsRef<Path>, kind: impl Into<String>, structural_digest: &str) -> Self {
        let path = path.as_ref().to_path_buf();
        let kind = kind.into();
        let seed = format!("{}:{}:{}", kind, path.display(), structural_digest);
        let hash = blake3::hash(seed.as_bytes()).to_hex().to_string();

        Self {
            hash: hash.clone(),
            source_path: path,
            kind: kind.clone(),
            label: kind,
        }
    }

    /// Create a pointer for a file-level root artifact.
    pub fn file(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let hash = blake3::hash(path.to_string_lossy().as_bytes()).to_hex().to_string();
        Self {
            hash: hash.clone(),
            source_path: path.to_path_buf(),
            kind: "file".to_string(),
            label: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        }
    }
}

impl fmt::Display for GraphPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.kind, self.hash)
    }
}
