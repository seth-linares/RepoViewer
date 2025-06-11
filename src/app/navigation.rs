//! Navigation-related functionality for the RepoViewer application.
//! 
//! While core navigation (navigate_into, navigate_up) lives in mod.rs,
//! this module contains helper functions and extensions for directory
//! traversal and navigation features.

use super::App;
use std::path::PathBuf;
use crate::app_error::AppError;

impl App {
    /// Get the depth of the current directory relative to the start directory
    /// This can be useful for showing breadcrumbs or limiting navigation depth
    pub fn get_current_depth(&self) -> usize {
        if let Ok(relative) = self.current_dir.strip_prefix(&self.start_dir) {
            relative.components().count()
        } else {
            // If we're somehow outside the start directory, return 0
            0
        }
    }

    /// Get a breadcrumb trail from the start directory to current directory
    /// Returns a vector of (name, full_path) tuples
    pub fn get_breadcrumbs(&self) -> Vec<(String, PathBuf)> {
        let mut breadcrumbs = Vec::new();
        let mut current = self.current_dir.clone();
        
        // Build breadcrumbs from current back to start
        loop {
            let name = current
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| {
                    if current == self.start_dir {
                        "~".to_string()  // Home/start indicator
                    } else {
                        "/".to_string()  // Root indicator
                    }
                });
            
            breadcrumbs.push((name, current.clone()));
            
            // Stop at start directory or root
            if current == self.start_dir || current.parent().is_none() {
                break;
            }
            
            // Move up to parent
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        // Reverse to get start -> current order
        breadcrumbs.reverse();
        breadcrumbs
    }

    /// Check if we can navigate up from current position
    /// Useful for UI to enable/disable up navigation
    pub fn can_navigate_up(&self) -> bool {
        self.current_dir.parent().is_some()
    }

    /// Check if the current selection is a directory we can enter
    /// Useful for UI to show navigation hints
    pub fn can_navigate_into_selection(&self) -> bool {
        self.current_selection()
            .map(|item| item.is_dir && !item.is_symlink)
            .unwrap_or(false)
    }

    /// Navigate to a specific path directly
    /// This could be used for bookmarks or jump-to functionality
    pub fn navigate_to_path(&mut self, path: PathBuf) -> Result<(), AppError> {
        // Validate that the path exists and is a directory
        if !path.exists() {
            return Err(AppError::DirectoryNotFound(
                path.to_string_lossy().to_string()
            ));
        }
        
        if !path.is_dir() {
            return Err(AppError::NotADirectory(
                path.to_string_lossy().to_string()
            ));
        }
        
        // Update current directory and refresh
        self.current_dir = path;
        self.refresh_files()?;
        
        Ok(())
    }

    /// Navigate to the git root if we're in a git repository
    /// Convenient shortcut for jumping to project root
    pub fn navigate_to_git_root(&mut self) -> Result<(), AppError> {
        if let Some(git_root) = self.git_root.clone() {
            self.navigate_to_path(git_root)
        } else {
            Err(AppError::LogicError(
                "Not in a git repository".to_string()
            ))
        }
    }

    /// Navigate to the start directory (where repoviewer was launched)
    /// Provides a "home" functionality
    pub fn navigate_to_start(&mut self) -> Result<(), AppError> {
        let start_dir = self.start_dir.clone();
        self.navigate_to_path(start_dir)
    }
}

// Note: Tree generation functions are in export.rs since they're
// primarily about exporting/displaying directory structure rather than
// navigating it.