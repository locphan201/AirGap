use crate::io::{read_note, write_atomic, IoError};
use crate::path_safety::{sanitize_path, validate_filename, PathSafetyError};
use crate::trash::{delete_to_trash, TrashError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] IoError),

    #[error("Standard I/O error: {0}")]
    StdIo(#[from] std::io::Error),

    #[error("Path safety error: {0}")]
    PathSafety(#[from] PathSafetyError),

    #[error("Trash error: {0}")]
    Trash(#[from] TrashError),

    #[error("Item already exists at: {0}")]
    AlreadyExists(PathBuf),

    #[error("Item does not exist at: {0}")]
    NotFound(PathBuf),

    #[error("Target is not a directory: {0}")]
    NotADirectory(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VaultItemType {
    Folder,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeNode {
    pub name: String,
    pub relative_path: PathBuf,
    pub item_type: VaultItemType,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct Vault {
    pub root: PathBuf,
}

impl Vault {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Recursively scans vault directory and builds a hierarchical tree node.
    pub fn scan_tree(&self) -> Result<TreeNode, VaultError> {
        self.scan_dir_recursive(&self.root, Path::new(""))
    }

    fn scan_dir_recursive(&self, dir_path: &Path, rel_path: &Path) -> Result<TreeNode, VaultError> {
        let name = if rel_path.as_os_str().is_empty() {
            self.root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Vault".to_string())
        } else {
            rel_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };

        let mut children = Vec::new();
        if dir_path.is_dir() {
            let entries = fs::read_dir(dir_path)?;
            let mut folders = Vec::new();
            let mut notes = Vec::new();

            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().into_owned();

                // Skip hidden files/directories (like .git, .trash)
                if file_name.starts_with('.') {
                    continue;
                }

                let child_path = entry.path();
                let child_rel = rel_path.join(&file_name);

                if child_path.is_dir() {
                    let sub_tree = self.scan_dir_recursive(&child_path, &child_rel)?;
                    folders.push(sub_tree);
                } else if file_name.ends_with(".md")
                    || file_name.ends_with(".markdown")
                    || file_name.ends_with(".txt")
                {
                    notes.push(TreeNode {
                        name: file_name,
                        relative_path: child_rel,
                        item_type: VaultItemType::Note,
                        children: Vec::new(),
                    });
                }
            }

            // Natural sort: folders first (alphabetical), then notes (alphabetical)
            folders.sort_by_key(|a| a.name.to_lowercase());
            notes.sort_by_key(|a| a.name.to_lowercase());

            children.extend(folders);
            children.extend(notes);
        }

        Ok(TreeNode {
            name,
            relative_path: rel_path.to_path_buf(),
            item_type: VaultItemType::Folder,
            children,
        })
    }

    /// Creates a new note in the specified directory with auto-incrementing collision resolution.
    /// E.g. Untitled.md -> Untitled 1.md -> Untitled 2.md
    pub fn create_note_with_collision_handling(
        &self,
        parent_rel: &Path,
        base_name: &str,
    ) -> Result<PathBuf, VaultError> {
        let parent_abs = sanitize_path(&self.root, parent_rel)?;
        if !parent_abs.is_dir() {
            return Err(VaultError::NotADirectory(parent_abs));
        }

        let stem = Path::new(base_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(base_name);

        let ext = Path::new(base_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("md");

        let mut candidate_name = format!("{}.{}", stem, ext);
        validate_filename(&candidate_name)?;

        let mut candidate_path = parent_abs.join(&candidate_name);
        let mut index = 1;

        while candidate_path.exists() {
            candidate_name = format!("{} {}.{}", stem, index, ext);
            validate_filename(&candidate_name)?;
            candidate_path = parent_abs.join(&candidate_name);
            index += 1;
        }

        write_atomic(&candidate_path, "")?;
        let rel_result = candidate_path
            .strip_prefix(&self.root)
            .unwrap_or(&candidate_path)
            .to_path_buf();

        Ok(rel_result)
    }

    /// Creates a new folder.
    pub fn create_folder(&self, parent_rel: &Path, name: &str) -> Result<PathBuf, VaultError> {
        validate_filename(name)?;
        let parent_abs = sanitize_path(&self.root, parent_rel)?;
        let target = parent_abs.join(name);
        if target.exists() {
            return Err(VaultError::AlreadyExists(target));
        }
        fs::create_dir_all(&target)?;
        let rel = target
            .strip_prefix(&self.root)
            .unwrap_or(&target)
            .to_path_buf();
        Ok(rel)
    }

    /// Creates a note using auto-incrementing collision suffixing (Untitled.md, Untitled 1.md...).
    pub fn create_note(&self, parent_rel: &Path, name: &str) -> Result<PathBuf, VaultError> {
        self.create_note_with_collision_handling(parent_rel, name)
    }

    /// Renames a note or folder in the vault.
    pub fn rename_item(&self, from_rel: &Path, to_name: &str) -> Result<PathBuf, VaultError> {
        validate_filename(to_name)?;
        let from_abs = sanitize_path(&self.root, from_rel)?;
        if !from_abs.exists() {
            return Err(VaultError::NotFound(from_abs));
        }

        let parent = from_abs.parent().ok_or_else(|| {
            VaultError::PathSafety(PathSafetyError::TraversalAttempted(
                from_rel.to_string_lossy().into_owned(),
            ))
        })?;
        let to_abs = parent.join(to_name);

        if to_abs.exists() && to_abs != from_abs {
            return Err(VaultError::AlreadyExists(to_abs));
        }

        fs::rename(&from_abs, &to_abs)?;
        let rel_result = to_abs
            .strip_prefix(&self.root)
            .unwrap_or(&to_abs)
            .to_path_buf();
        Ok(rel_result)
    }

    /// Moves a note or folder to the OS recycle bin or .trash/ fallback.
    pub fn delete_item(&self, path_rel: &Path) -> Result<(), VaultError> {
        let abs_path = sanitize_path(&self.root, path_rel)?;
        if !abs_path.exists() {
            return Err(VaultError::NotFound(abs_path));
        }
        delete_to_trash(&self.root, &abs_path)?;
        Ok(())
    }

    /// Reads a note given its relative path.
    pub fn read_note(&self, relative_path: &Path) -> Result<String, VaultError> {
        let abs_path = sanitize_path(&self.root, relative_path)?;
        Ok(read_note(&abs_path)?)
    }

    /// Writes a note atomically given its relative path.
    pub fn write_note_atomic(&self, relative_path: &Path, content: &str) -> Result<(), VaultError> {
        let abs_path = sanitize_path(&self.root, relative_path)?;
        Ok(write_atomic(&abs_path, content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_collision_naming() {
        let dir = tempdir().unwrap();
        let vault = Vault::new(dir.path());

        let note1 = vault
            .create_note_with_collision_handling(Path::new(""), "Untitled.md")
            .unwrap();
        assert_eq!(note1.to_string_lossy(), "Untitled.md");

        let note2 = vault
            .create_note_with_collision_handling(Path::new(""), "Untitled.md")
            .unwrap();
        assert_eq!(note2.to_string_lossy(), "Untitled 1.md");

        let note3 = vault
            .create_note_with_collision_handling(Path::new(""), "Untitled.md")
            .unwrap();
        assert_eq!(note3.to_string_lossy(), "Untitled 2.md");
    }

    #[test]
    fn test_tree_scan_and_sorting() {
        let dir = tempdir().unwrap();
        let vault = Vault::new(dir.path());

        vault.create_folder(Path::new(""), "Docs").unwrap();
        vault.create_folder(Path::new(""), "Assets").unwrap();
        vault
            .create_note_with_collision_handling(Path::new(""), "Alpha.md")
            .unwrap();
        vault
            .create_note_with_collision_handling(Path::new(""), "Beta.md")
            .unwrap();

        let tree = vault.scan_tree().unwrap();
        assert_eq!(tree.children.len(), 4);
        // Folders first: Assets, Docs
        assert_eq!(tree.children[0].name, "Assets");
        assert_eq!(tree.children[1].name, "Docs");
        // Then notes: Alpha.md, Beta.md
        assert_eq!(tree.children[2].name, "Alpha.md");
        assert_eq!(tree.children[3].name, "Beta.md");
    }

    #[test]
    fn test_create_note_rename_and_delete() {
        let dir = tempdir().unwrap();
        let vault = Vault::new(dir.path());

        // create_note
        let note_rel = vault.create_note(Path::new(""), "Untitled.md").unwrap();
        assert_eq!(note_rel, PathBuf::from("Untitled.md"));
        assert!(dir.path().join("Untitled.md").exists());

        // rename_item
        let renamed_rel = vault.rename_item(&note_rel, "Renamed.md").unwrap();
        assert_eq!(renamed_rel, PathBuf::from("Renamed.md"));
        assert!(!dir.path().join("Untitled.md").exists());
        assert!(dir.path().join("Renamed.md").exists());

        // delete_item
        vault.delete_item(&renamed_rel).unwrap();
        assert!(!dir.path().join("Renamed.md").exists());
    }
}
