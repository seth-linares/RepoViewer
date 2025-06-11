//! File collection functionality for the RepoViewer application.
//! 
//! This module manages the core feature of RepoViewer - collecting files
//! from various directories and maintaining them as a cohesive collection
//! that can be exported for sharing with LLMs or documentation purposes.
//! 
//! The collection system is designed to handle the dynamic nature of
//! software development, where files are constantly being edited, moved,
//! and deleted. It provides change detection and synchronization to keep
//! the collection up-to-date with the filesystem.

use super::{App, FileItem};
use super::state::{FileStatus, RefreshResult, RefreshSummary};
use crate::{
    app_error::AppError,
    utils::{get_file_type, read_file_safely, MEGABYTE},
};
use std::{
    fs,
    path::PathBuf,
    time::SystemTime,
};

/// Represents a file that has been collected for export
/// 
/// This struct is more than just file content - it's a complete snapshot
/// of a file at a specific point in time. The metadata allows us to:
/// - Detect when the source file has changed (via last_modified)
/// - Quickly check if content is different (via content_hash)
/// - Display human-readable paths (via relative_path)
/// - Apply proper syntax highlighting (via language)
#[derive(Debug, Clone)]
pub struct CollectedFile {
    pub path: PathBuf,              // Absolute path to the original file
    pub relative_path: String,      // Human-readable relative path for display
    pub content: String,            // The actual file content at collection time
    pub language: String,           // Programming language for syntax highlighting
    pub collected_at: SystemTime,   // When we collected this snapshot
    pub content_hash: u64,          // Quick fingerprint for change detection
    pub file_size: u64,             // Original file size in bytes
    pub last_modified: SystemTime,  // File's modification time when collected
}

// Implementation of file collection operations
impl App {
    /// Add the currently selected file to the collection
    /// 
    /// This method handles several important scenarios:
    /// - Validates that a file (not directory) is selected
    /// - Checks file size and type safety before reading
    /// - Updates existing entries rather than creating duplicates
    /// - Provides detailed error messages for various failure modes
    pub fn add_current_file(&mut self) -> Result<(), AppError> {
        // First, ensure we have a valid selection
        let current_item = match self.current_selection() {
            Some(item) => item,
            None => {
                self.set_error_message("No file selected".to_string());
                return Ok(());
            }
        };

        // Directories can't be collected - we only collect file contents
        if current_item.is_dir {
            self.set_error_message("Cannot collect directories".to_string());
            return Ok(());
        }

        // Try to create a CollectedFile from the selected item
        // This is where we read the file and validate it's safe to collect
        let new_collected_file = match self.create_collected_file(current_item) {
            Ok(file) => file,
            Err(e) => {
                // Transform technical errors into user-friendly messages
                // This helps users understand why a file couldn't be collected
                let error_message = match e {
                    AppError::FileTooLarge { size, max } => {
                        format!("File too large: {} (max: {})", 
                            self.format_size(size as usize),
                            self.format_size(max)
                        )
                    },
                    AppError::BinaryFile => {
                        "Cannot collect binary files - only text files are supported".to_string()
                    },
                    AppError::UnrecognizedFileType { extension } => {
                        match extension {
                            Some(ext) => format!("Unsupported file type: .{}", ext),
                            None => "File has no extension - cannot determine type".to_string()
                        }
                    },
                    AppError::EncodingError => {
                        "File has encoding issues - too many invalid UTF-8 characters".to_string()
                    },
                    _ => format!("Failed to read file: {}", e),
                };
                self.set_error_message(error_message);
                return Ok(());
            }
        };

        // Calculate size information for user feedback
        let size_kb = new_collected_file.content.len() / 1024;
        let name = current_item.name.clone();

        // Check if this file is already in our collection
        // This allows users to "refresh" a file by adding it again
        let existing_index = self.collected_files
            .iter()
            .position(|f| f.path == new_collected_file.path);
        
        let old_count = self.collected_files.len();

        // Either update the existing entry or add a new one
        match existing_index {
            Some(index) => {
                // Replace the old version with the new one
                self.collected_files[index] = new_collected_file;
                
                // Build success message with size warning if applicable
                let mut message = format!(
                    "Updated {} ({} KB) - Total: {} files",
                    name, size_kb, old_count
                );
                
                // Add warning if collection is getting large
                if let Some(warning) = self.get_size_warning() {
                    message.push_str(&format!(" | {}", warning));
                }
                
                self.set_success_message(message);
            }
            None => {
                // Add as a new file to the collection
                self.collected_files.push(new_collected_file);
                
                // Build success message with size warning if applicable
                let mut message = format!(
                    "Added {} ({} KB) - Total: {} files",
                    name, size_kb, old_count + 1
                );
                
                // Add warning if collection is getting large
                if let Some(warning) = self.get_size_warning() {
                    message.push_str(&format!(" | {}", warning));
                }
                
                self.set_success_message(message);
            }
        }

        Ok(())
    }
    
    /// Add all files in the current directory to the collection
    /// 
    /// This bulk operation is perfect for collecting all source files in a
    /// module or package. It intelligently:
    /// - Skips directories and non-text files
    /// - Updates files that are already in the collection
    /// - Provides a summary of what was added/updated/skipped
    /// - Warns when the collection size is getting large
    pub fn add_all_files_in_dir(&mut self) -> Result<(), AppError> {
        let mut added = 0;
        let mut updated = 0;
        let mut skipped = 0;
        let mut errors = 0;

        // Store initial size to detect if we're crossing size thresholds
        let initial_size = self.get_collection_size();

        // Process each item in the current directory
        for item in &self.items {
            // Skip directories - we only collect files
            if item.is_dir {
                skipped += 1;
                continue;
            }

            // Check if this file is already in our collection
            if let Some(index) = self.collected_files.iter().position(|f| f.path == item.path) {
                // File exists - try to update it with fresh content
                match self.create_collected_file(item) {
                    Ok(new_file) => {
                        self.collected_files[index] = new_file;
                        updated += 1;
                    }
                    Err(AppError::NotAFile) => {
                        skipped += 1;
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }
            } else {
                // New file - try to add it to collection
                match self.create_collected_file(item) {
                    Ok(new_file) => {
                        self.collected_files.push(new_file);
                        added += 1;
                    }
                    Err(AppError::NotAFile) => {
                        skipped += 1;
                    }
                    Err(_) => {
                        errors += 1;
                    }
                }
            }
        }

        // Prepare comprehensive feedback for the user
        let total = self.collected_files.len();
        let current_size = self.get_collection_size();
        let size_str = self.format_size(current_size);
        
        // Build the status message with all the statistics
        let mut message = format!(
            "Added {} files, updated {}, skipped {} (errors: {}) - Total: {} files ({})",
            added, updated, skipped, errors, total, size_str
        );
        
        // Add size warning if we've crossed a threshold
        if let Some(warning) = self.get_size_warning() {
            message.push_str(&format!("\n{}", warning));
            
            // Special tip if we just crossed into warning territory
            const WARNING_THRESHOLD: usize = 25 * MEGABYTE;
            if initial_size < WARNING_THRESHOLD && current_size >= WARNING_THRESHOLD {
                message.push_str("\nTip: Use 'd' to remove individual files or 'D' to clear all");
            }
        }
        
        self.set_success_message(message);

        Ok(())
    }
    
    /// Remove the currently selected file from the collection
    /// 
    /// This allows users to curate their collection by removing files
    /// they no longer need. The file remains in the filesystem - we're
    /// just removing it from our export collection.
    pub fn remove_current_file(&mut self) -> Result<(), AppError> {
        let current_item = match self.current_selection() {
            Some(item) => item,
            None => {
                self.set_error_message("No file selected".to_string());
                return Ok(());
            }
        };

        if current_item.is_dir {
            self.set_error_message("Cannot remove directories from collection".to_string());
            return Ok(());
        }

        // Clone values to avoid borrowing issues with the iterator
        let path_to_remove = current_item.path.clone();
        let name = current_item.name.clone();
        
        // Find the file in our collection
        let index = self.collected_files.iter().position(|f| f.path == path_to_remove);
        
        if let Some(index) = index {
            // Remove the file from the collection
            // swap_remove is O(1) but changes order - that's fine for us
            let removed_file = self.collected_files.swap_remove(index);
            let size_kb = removed_file.content.len() / 1024;
            self.set_success_message(format!(
                "Removed {} ({} KB) - Total: {} files",
                name, size_kb, self.collected_files.len()
            ));
        } else {
            self.set_error_message(format!(
                "{} is not in the collection",
                name
            ));
        }

        Ok(())
    }
    
    /// Clear the entire collection
    /// 
    /// This is a quick way to start over with a fresh collection.
    /// Useful when switching between different features or projects.
    pub fn clear_collection(&mut self) -> Result<(), AppError> {
        if self.collected_files.is_empty() {
            self.set_error_message("Collection is already empty".to_string());
            return Ok(());
        }

        let count = self.collected_files.len();
        self.collected_files.clear();
        self.set_success_message(format!("Cleared {} files from collection", count));

        Ok(())
    }

    /// Check if a collected file has changed on disk
    /// 
    /// This method performs a comprehensive health check on a collected file:
    /// - Does the file still exist?
    /// - Is it still a regular file (not replaced by a directory)?
    /// - Has it been modified since we collected it?
    /// 
    /// This information is crucial for keeping collections synchronized
    /// with actively developed codebases.
    pub fn check_file_status(&self, collected: &CollectedFile) -> FileStatus {
        // First check if the file still exists at its original location
        if !collected.path.exists() {
            return FileStatus::Deleted;
        }
        
        // Check if it's still a regular file
        // Sometimes files get replaced by directories during refactoring
        if !collected.path.is_file() {
            return FileStatus::NotAFile;
        }
        
        // Check modification time to detect changes
        match fs::metadata(&collected.path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => {
                        // Compare modification times
                        // If the file was modified after we collected it, it's changed
                        if modified > collected.last_modified {
                            FileStatus::Modified
                        } else {
                            FileStatus::Unchanged
                        }
                    }
                    Err(_) => FileStatus::Unknown,
                }
            }
            Err(_) => FileStatus::Inaccessible,
        }
    }
    
    /// Refresh a single collected file if it has changed
    /// 
    /// This private method handles the actual refresh operation for one file.
    /// It's used by refresh_all_collected to update the entire collection.
    fn refresh_collected_file(&mut self, index: usize) -> Result<RefreshResult, AppError> {
        if index >= self.collected_files.len() {
            return Err(AppError::LogicError("Invalid collection index".to_string()));
        }
        
        let old_file = &self.collected_files[index];
        let status = self.check_file_status(old_file);
        
        match status {
            FileStatus::Unchanged => Ok(RefreshResult::NoChange),
            FileStatus::Modified => {
                // File has changed - create a fresh snapshot
                // We create a temporary FileItem to reuse our existing logic
                let temp_item = FileItem {
                    path: old_file.path.clone(),
                    name: old_file.path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    is_dir: false,
                    is_symlink: false,
                    is_hidden: false,
                };

                // Try to re-read the file with all our safety checks
                match self.create_collected_file(&temp_item) {
                    Ok(new_file) => {
                        self.collected_files[index] = new_file;
                        Ok(RefreshResult::Updated)
                    }
                    Err(e) => Ok(RefreshResult::Failed(e.to_string())),
                }
            }
            FileStatus::Deleted => Ok(RefreshResult::FileDeleted),
            FileStatus::Inaccessible => Ok(RefreshResult::FileInaccessible),
            FileStatus::NotAFile => Ok(RefreshResult::Failed("No longer a file".to_string())),
            FileStatus::Unknown => Ok(RefreshResult::Failed("Cannot check status".to_string())),
        }
    }
    
    /// Refresh all collected files, removing deleted ones
    /// 
    /// This is one of RepoViewer's most powerful features. It synchronizes
    /// your collection with the current state of the filesystem, ensuring
    /// that you're always working with up-to-date content. The method:
    /// - Checks each file for changes
    /// - Updates modified files with fresh content
    /// - Removes files that no longer exist
    /// - Provides a detailed summary of what changed
    pub fn refresh_all_collected(&mut self) -> RefreshSummary {
        let mut summary = RefreshSummary::default();
        let mut indices_to_remove = Vec::new();

        // Process each file and track what happens
        // We use indices to avoid holding mutable references
        for index in 0..self.collected_files.len() {
            match self.refresh_collected_file(index) {
                Ok(RefreshResult::NoChange) => summary.unchanged += 1,
                Ok(RefreshResult::Updated) => summary.updated += 1,
                Ok(RefreshResult::FileDeleted) => {
                    summary.deleted += 1;
                    indices_to_remove.push(index);
                }
                Ok(RefreshResult::FileInaccessible) => {
                    summary.inaccessible += 1;
                    indices_to_remove.push(index);
                }
                Ok(RefreshResult::Failed(_)) => summary.failed += 1,
                Err(_) => summary.failed += 1,
            }
        }

        // Remove deleted/inaccessible files from the collection
        // We remove in reverse order to maintain valid indices
        indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices_to_remove {
            self.collected_files.remove(index);
        }

        summary
    }

    /// Create a CollectedFile from a FileItem
    /// 
    /// This is the core method that transforms a file reference into a
    /// collected snapshot. It performs multiple safety checks and extracts
    /// all the metadata we need for change tracking. The method:
    /// - Validates the file is safe to read (size, type, encoding)
    /// - Reads the content with proper error handling
    /// - Calculates a content hash for quick change detection
    /// - Determines the appropriate relative path for display
    /// - Identifies the programming language for syntax highlighting
    pub(super) fn create_collected_file(&self, item: &FileItem) -> Result<CollectedFile, AppError> {
        if item.is_dir {
            return Err(AppError::NotAFile);
        }
        
        // Get file metadata before reading content
        // This helps us validate the file and capture its state
        let metadata = fs::metadata(&item.path)?;
        let last_modified = metadata.modified()?;
        let file_size = metadata.len();
        
        // Try to read the file content safely
        // This function handles size limits, binary detection, and encoding
        let content = read_file_safely(&item.path, 10 * MEGABYTE)?;
        
        // Calculate content hash for quick change comparison
        // We use the default hasher which is fast and good enough for our needs
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let content_hash = hasher.finish();
        
        // Calculate a meaningful relative path for display
        let relative_path = self.calculate_relative_path(&item.path)?;
        
        // Determine the language for syntax highlighting
        let language = get_file_type(&item.path)
            .unwrap_or("plaintext")
            .to_string();
        
        Ok(CollectedFile {
            path: item.path.clone(),
            relative_path,
            content,
            language,
            collected_at: SystemTime::now(),
            content_hash,
            file_size,
            last_modified,
        })
    }
}