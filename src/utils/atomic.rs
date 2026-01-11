//! Atomic file operations
//!
//! This module provides utilities for atomic file writes to prevent
//! data corruption during crashes or power failures.
//!
//! # Pattern
//!
//! 1. Write to a temporary file (.tmp)
//! 2. Call sync_all() to flush to disk
//! 3. Rename temp file to final path (atomic on most filesystems)
//!
//! This ensures that the final file is either:
//! - The old version (if crash before rename)
//! - The new version (if rename completed)
//! - Never a partial/corrupted state

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Result type for atomic operations
pub type AtomicResult<T> = Result<T, AtomicError>;

/// Errors that can occur during atomic operations
#[derive(Debug)]
pub enum AtomicError {
    Io(io::Error),
    TempFileExists(String),
}

impl std::fmt::Display for AtomicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomicError::Io(e) => write!(f, "IO error: {}", e),
            AtomicError::TempFileExists(path) => write!(f, "Temp file already exists: {}", path),
        }
    }
}

impl std::error::Error for AtomicError {}

impl From<io::Error> for AtomicError {
    fn from(e: io::Error) -> Self {
        AtomicError::Io(e)
    }
}

/// Atomically write content to a file
///
/// This function:
/// 1. Writes content to a .tmp file
/// 2. Syncs the file to disk
/// 3. Atomically renames to the final path
///
/// # Arguments
///
/// * `path` - The final destination path
/// * `content` - The content to write
///
/// # Example
///
/// ```ignore
/// atomic_write("data/snapshot.jsonl", "line1\nline2\n")?;
/// ```
pub fn atomic_write<P: AsRef<Path>>(path: P, content: &str) -> AtomicResult<()> {
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temp file
    let mut file = File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;

    // Sync to disk (ensure data is durable)
    file.sync_all()?;

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Atomically write content using a writer function
///
/// This is more efficient for large files as it doesn't require
/// building the entire content string in memory first.
///
/// # Arguments
///
/// * `path` - The final destination path
/// * `write_fn` - A function that writes content to the file
///
/// # Example
///
/// ```ignore
/// atomic_write_with("data/snapshot.jsonl", |file| {
///     writeln!(file, "line1")?;
///     writeln!(file, "line2")?;
///     Ok(())
/// })?;
/// ```
pub fn atomic_write_with<P, F>(path: P, write_fn: F) -> AtomicResult<()>
where
    P: AsRef<Path>,
    F: FnOnce(&mut File) -> io::Result<()>,
{
    let path = path.as_ref();
    let temp_path = path.with_extension("tmp");

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temp file using the provided function
    let mut file = File::create(&temp_path)?;
    write_fn(&mut file)?;

    // Sync to disk
    file.sync_all()?;

    // Atomic rename
    fs::rename(&temp_path, path)?;

    Ok(())
}

/// Safely rename a file, creating a backup if the destination exists
///
/// # Arguments
///
/// * `from` - Source file path
/// * `to` - Destination file path
/// * `backup` - Optional backup path for existing destination
///
/// # Returns
///
/// * `Ok(true)` - File was renamed successfully
/// * `Ok(false)` - Source file doesn't exist
/// * `Err(...)` - An error occurred
pub fn safe_rename<P1, P2, P3>(from: P1, to: P2, backup: Option<P3>) -> AtomicResult<bool>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let from = from.as_ref();
    let to = to.as_ref();

    // Check if source exists
    if !from.exists() {
        return Ok(false);
    }

    // Backup existing destination if requested
    if let Some(backup_path) = backup {
        if to.exists() {
            // Remove old backup if exists
            let backup = backup_path.as_ref();
            if backup.exists() {
                fs::remove_file(backup)?;
            }
            fs::rename(to, backup)?;
        }
    }

    // Rename source to destination
    fs::rename(from, to)?;

    Ok(true)
}

/// Clean up any leftover temp files from interrupted operations
///
/// Call this on startup to clean up .tmp files that may have been
/// left behind from crashes.
pub fn cleanup_temp_files<P: AsRef<Path>>(dir: P) -> AtomicResult<usize> {
    let dir = dir.as_ref();
    let mut cleaned = 0;

    if !dir.exists() {
        return Ok(0);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "tmp").unwrap_or(false) {
            fs::remove_file(&path)?;
            cleaned += 1;
        }
    }

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");

        atomic_write(&path, "Hello, World!").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello, World!");

        // Temp file should not exist
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn test_atomic_write_with() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");

        atomic_write_with(&path, |file| {
            writeln!(file, "Line 1")?;
            writeln!(file, "Line 2")?;
            Ok(())
        })
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Line 1\nLine 2\n");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("subdir").join("nested").join("test.txt");

        atomic_write(&path, "nested content").unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "nested content");
    }

    #[test]
    fn test_safe_rename_with_backup() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("source.txt");
        let to = temp_dir.path().join("dest.txt");
        let backup = temp_dir.path().join("backup.txt");

        // Create source and destination
        fs::write(&from, "new content").unwrap();
        fs::write(&to, "old content").unwrap();

        // Rename with backup
        let result = safe_rename(&from, &to, Some(&backup)).unwrap();
        assert!(result);

        // Check results
        assert!(!from.exists());
        assert_eq!(fs::read_to_string(&to).unwrap(), "new content");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "old content");
    }

    #[test]
    fn test_safe_rename_source_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("nonexistent.txt");
        let to = temp_dir.path().join("dest.txt");

        let result = safe_rename(&from, &to, None::<&Path>).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_cleanup_temp_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create some temp files
        fs::write(temp_dir.path().join("file1.tmp"), "temp1").unwrap();
        fs::write(temp_dir.path().join("file2.tmp"), "temp2").unwrap();
        fs::write(temp_dir.path().join("keep.txt"), "keep").unwrap();

        let cleaned = cleanup_temp_files(temp_dir.path()).unwrap();
        assert_eq!(cleaned, 2);

        // Check that .tmp files are gone but .txt remains
        assert!(!temp_dir.path().join("file1.tmp").exists());
        assert!(!temp_dir.path().join("file2.tmp").exists());
        assert!(temp_dir.path().join("keep.txt").exists());
    }
}
