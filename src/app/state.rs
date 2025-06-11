//! State management and utility functions for the RepoViewer application.
//! 
//! This module contains types and methods for managing application state,
//! including messages, file status tracking, and UI helper functions.

use super::App;
use crate::utils::MEGABYTE;
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime},
};

/// Represents a UI message with optional timeout
/// Messages appear as popups to inform the user of success/failure
#[derive(Clone, Debug)]
pub struct Message {
    pub text: String,
    pub created_at: Instant,
    pub timeout: Duration,
    pub success: bool,
}

/// Result of checking a single file's status
/// Used to determine if collected files need updating
#[derive(Debug)]
pub enum FileStatus {
    Unchanged,
    Modified,
    Deleted,
    NotAFile,
    Inaccessible,
    Unknown,
}

/// Result of refreshing a single file
/// Provides more detail about what happened during refresh
#[derive(Debug)]
pub enum RefreshResult {
    NoChange,
    Updated,
    FileDeleted,
    FileInaccessible,
    Failed,
}

/// Summary of refreshing all collected files
/// Gives the user a complete picture of what changed
#[derive(Debug, Default)]
pub struct RefreshSummary {
    pub unchanged: usize,
    pub updated: usize,
    pub deleted: usize,
    pub inaccessible: usize,
    pub failed: usize,
}

// Message and UI state management implementation
impl App {
    /// Set a success message that will disappear after a timeout
    pub fn set_success_message(&mut self, text: String) {
        self.message = Some(Message {
            text,
            created_at: Instant::now(),
            timeout: Duration::from_secs(3), // 3 seconds timeout
            success: true,
        });
    }

    /// Set an error message that will disappear after a timeout
    pub fn set_error_message(&mut self, text: String) {
        self.message = Some(Message {
            text,
            created_at: Instant::now(),
            timeout: Duration::from_secs(3), // 3 seconds timeout
            success: false,
        });
    }

    /// Check if current message has timed out and clear it if needed
    /// This should be called in the main event loop
    pub fn update_message(&mut self) {
        if let Some(message) = &self.message {
            if message.created_at.elapsed() >= message.timeout {
                self.message = None;
            }
        }
    }

    /// Get contextual hints based on current state
    /// These hints guide new users and provide helpful reminders
    pub fn get_contextual_hint(&self) -> Option<String> {
        // Priority order for hints - show the most relevant one
        
        if self.show_help {
            return Some("Press '?' or ESC to close help".to_string());
        }
        
        // Navigation hints based on current location
        if self.current_dir != self.start_dir && self.get_current_depth() > 3 {
            return Some("Tip: Press '~' to quickly return to the start directory".to_string());
        }
        
        if let Some(git_root) = &self.git_root {
            if self.current_dir != *git_root && self.get_current_depth() > 2 {
                return Some("Tip: Press 'G' to jump to the git repository root".to_string());
            }
        }
        
        // New user hint - no files collected yet
        if self.collected_files.is_empty() {
            if let Some(item) = self.current_selection() {
                if !item.is_dir {
                    return Some("Press 'a' to add this file to your collection".to_string());
                } else if self.items.iter().any(|i| !i.is_dir) {
                    return Some("Press 'A' to add all files in this directory".to_string());
                } else {
                    return Some("Navigate into directories with → to find files to collect".to_string());
                }
            }
            return Some("Navigate to files and press 'a' to start collecting".to_string());
        }
        
        // Collection size warnings
        let size = self.get_collection_size();
        if size > 50 * MEGABYTE {
            return Some("Collection is very large! Consider using 'd' to remove files or 'S' to save".to_string());
        } else if size > 25 * MEGABYTE {
            return Some("Collection growing large. Ready to export with 'S' or 'C'".to_string());
        }
        
        // Suggest refresh if files might be stale (collected over 5 minutes ago)
        if !self.collected_files.is_empty() {
            let oldest_collection = self.collected_files
                .iter()
                .map(|f| f.collected_at)
                .min();
                
            if let Some(oldest) = oldest_collection {
                if let Ok(elapsed) = SystemTime::now().duration_since(oldest) {
                    if elapsed.as_secs() > 300 { // 5 minutes
                        return Some("Files collected a while ago - press 'r' to refresh".to_string());
                    }
                }
            }
        }
        
        // Navigation-specific hints when in empty directories
        if self.items.is_empty() {
            return Some("Empty directory - press ← to go back".to_string());
        }
        
        // Only directories hint
        if self.items.iter().all(|i| i.is_dir) && !self.collected_files.is_empty() {
            return Some("Only directories here - navigate deeper or press 'S' to save your collection".to_string());
        }
        
        // Collection ready hints
        if self.collected_files.len() >= 5 {
            return Some("Press 'S' to save or 'C' to copy your collection".to_string());
        }
        
        // Default hint when collection has some files
        if self.collected_files.len() > 0 {
            return Some(format!("{} files collected - 'a' to add more, 'S' to save", 
                self.collected_files.len()));
        }
        
        None
    }

    

    /// Format a byte size into a human-readable string
    pub fn format_size(&self, bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;
        const GB: usize = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    /// Get the total size of the collection in bytes
    pub fn get_collection_size(&self) -> usize {
        self.collected_files.iter().map(|f| f.content.len()).sum()
    }

    /// Check collection size and return appropriate warning message
    /// This helps users avoid creating collections that are too large
    pub(super) fn get_size_warning(&self) -> Option<String> {
        
        let size = self.get_collection_size(); // iterate over vec and get size in bytes
        
        // Define our warning thresholds
        const WARNING_THRESHOLD: usize = 25 * MEGABYTE;
        const CRITICAL_THRESHOLD: usize = 50 * MEGABYTE;
        
        // Match general size and give feedback for how large the collection is becoming
        match size {
            s if s > CRITICAL_THRESHOLD => {
                Some(format!(
                    "⚠️ Collection is very large ({}) - Consider removing some files", 
                    self.format_size(s)
                ))
            },
            s if s > WARNING_THRESHOLD => {
                Some(format!(
                    "⚠️ Collection is getting large ({})", 
                    self.format_size(s)
                ))
            },
            _ => None
        }
    }

    /// Get a display-friendly path for status messages
    /// This provides shorter, more readable paths in the UI
    /// 
    /// Notes: 
    ///     - If the file is directly in the current directory: shows just the filename ("file.txt")
    ///     - If the file is in a subdirectory: shows "./subdir/file.txt"
    ///     - If the file is outside the current directory: calculates a relative path ("../otherdir/file.txt")
    ///     Fallback: shows the full path if relative path calculation fails
    pub fn get_display_path(&self, path: &Path) -> String {
    
        // So here we are trying to strip the absolute paths
        // e.g.:
        //    Current dir: "/home/user/project"
        //    Input path:  "/home/user/project/src/main.rs"
        //    Output:      "./src/main.rs"
        if let Ok(rel_path) = path.strip_prefix(&self.current_dir) {
            // If file is in current directory (strip leaves just names)
            // then the path will just be the filename
            if rel_path.components().count() == 1 {
                rel_path.to_string_lossy().to_string()
            } else {
                // Show relative path from current directory
                format!("./{}", rel_path.to_string_lossy())
            }
        } else {
            // Use the full relative path calculation for files outside current directory
            self.calculate_relative_path(path)
                .unwrap_or_else(|_| path.to_string_lossy().to_string())
        }
    }

    /// Check if a file is already in the collection
    pub fn is_collected(&self, path: &Path) -> bool {
        // iter through and see if there's a matching path
        self.collected_files.iter().any(|f| f.path == path)
    }

    /// Determine if we should include a file based on hidden and gitignore status
    pub(super) fn should_include_file(&self, path: &Path, name: &str, is_dir: bool) -> bool {
        if self.is_hidden(path, name) && !self.show_hidden {
            return false;
        }

        if let Some(ignore) = &self.gitignore {
            match ignore.matched(path, is_dir) {
                ignore::Match::Ignore(_) if !self.show_gitignored => return false,
                ignore::Match::Whitelist(_) => return true,
                _ => {}
            }
        }

        true
    }

    /// Check if a file is hidden (different on Windows than Linux)
    pub(super) fn is_hidden(&self, path: &Path, name: &str) -> bool {
        // First, check if the file/directory name starts with a dot
        // This is the Unix convention for hidden files
        let is_dot_file = name.starts_with('.');
        
        // On Windows, we also need to check file attributes
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(metadata) = path.metadata() {
                let attributes = metadata.file_attributes();
                // Windows hidden attribute is bit 2 (0x02)
                if (attributes & 2) != 0 {
                    return true;
                }
            }
        }

        let _ = path;
        // The file is hidden if it's a dot file
        is_dot_file
    }

    /// Calculate relative path with better error handling
    /// This provides meaningful paths for collected files
    pub(super) fn calculate_relative_path(&self, path: &Path) -> Result<String, crate::app_error::AppError> {
        // Try multiple strategies to get a meaningful relative path
        
        // First, try relative to git root
        if let Some(git_root) = &self.git_root {
            if let Ok(rel_path) = path.strip_prefix(git_root) {
                return Ok(rel_path.to_string_lossy().to_string());
            }
        }
        
        // Then try relative to start directory
        if let Ok(rel_path) = path.strip_prefix(&self.start_dir) {
            return Ok(rel_path.to_string_lossy().to_string());
        }
        
        // Finally, try relative to current directory
        if let Ok(rel_path) = path.strip_prefix(&self.current_dir) {
            // Prefix with current dir name to provide context
            let current_dir_name = self.current_dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "current".to_string());
            return Ok(format!("{}/{}", current_dir_name, rel_path.to_string_lossy()));
        }
        
        // Enhanced fallback: Always include parent directory context
        // This ensures users understand where the file is located even in edge cases
        
        // Get the parent directory name, handling edge cases
        let parent_name = path.parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                // If there's no parent or parent has no name (like root),
                // try to get a meaningful context
                if let Some(parent) = path.parent() {
                    // Try to get the last two components of the parent path
                    let components: Vec<_> = parent.components()
                        .rev()
                        .take(2)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .map(|c| c.as_os_str().to_string_lossy().to_string())
                        .collect();
                    
                    if components.is_empty() {
                        "root".to_string()
                    } else {
                        components.join("/")
                    }
                } else {
                    "root".to_string()
                }
            });
        
        // Get the file name
        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                // Last resort: just use the last component of the path
                path.components()
                    .last()
                    .map(|c| c.as_os_str().to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            });
        
        // Handle special cases for very long paths
        let relative_path = format!("{}/{}", parent_name, file_name);
        
        // If the path is too long, truncate the parent portion but keep the filename intact
        if relative_path.len() > 60 {
            let max_parent_len = 60_usize.saturating_sub(file_name.len() + 4); // 4 for ".../"
            if parent_name.len() > max_parent_len && max_parent_len > 3 {
                let truncated_parent = format!("...{}", &parent_name[parent_name.len() - max_parent_len + 3..]);
                Ok(format!("{}/{}", truncated_parent, file_name))
            } else {
                Ok(relative_path)
            }
        } else {
            Ok(relative_path)
        }
    }
}