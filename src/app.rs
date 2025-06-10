use std::{fs::{self}, path::{Path, PathBuf}, time::{Duration, Instant, SystemTime}};

use ignore::gitignore::Gitignore;
use ratatui::widgets::ListState;

use crate::{app_error::AppError, utils::{find_repo, get_file_type, read_file_safely, MEGABYTE}};

/// Result of refreshing a single file
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
#[derive(Debug)]
pub enum RefreshResult {
    NoChange,
    Updated,
    FileDeleted,
    FileInaccessible,
    Failed(String),
}

/// Summary of refreshing all collected files
#[derive(Debug, Default)]
pub struct RefreshSummary {
    pub unchanged: usize,
    pub updated: usize,
    pub deleted: usize,
    pub inaccessible: usize,
    pub failed: usize,
}


/// Main application state
#[derive(Clone)]
pub struct App {
    pub current_dir: PathBuf,
    pub start_dir: PathBuf,
    pub git_root: Option<PathBuf>,
    pub items: Vec<FileItem>,
    pub collected_files: Vec<CollectedFile>,
    pub state: ListState,
    pub gitignore: Option<Gitignore>,
    pub show_hidden: bool,
    pub show_gitignored: bool,
    pub message: Option<Message>,
    pub show_help: bool,
}

/// Represents a file system entry
#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
}

/// Represents a file that will be added to the clipboard
#[derive(Debug, Clone)]
pub struct CollectedFile {
    pub path: PathBuf,
    pub relative_path: String,
    pub content: String,
    pub language: String,
    pub collected_at: SystemTime,
    pub content_hash: u64,           // Quick way to detect content changes
    pub file_size: u64,              // Original file size
    pub last_modified: SystemTime,   // File's modification time when collected
}

/// Represents a UI message with optional timeout
#[derive(Clone, Debug)]
pub struct Message {
    pub text: String,
    pub created_at: Instant,
    pub timeout: Duration,
    pub success: bool,
}

impl App {
    pub fn new(start_dir: PathBuf) -> Result<Self, AppError> {
        // Try to see if there's a repo where we're looking
        let (git_root, gitignore) = find_repo(&start_dir)?;

        // Create app struct to be filled in and returned
        let mut app = App {
            current_dir: start_dir.clone(),
            start_dir,
            git_root,
            items: Vec::new(),
            collected_files: Vec::new(),
            state: ListState::default(),
            gitignore,
            show_hidden: false,
            show_gitignored: false,
            message: None,
            show_help: false,
        };

        // populate app 
        app.refresh_files()?;

        Ok(app)
    }


    pub fn refresh_files(&mut self) -> Result<(), AppError>{
        // Clear items out of our items vec to reset
        self.items.clear();

        // Unwrap the result into the ReadDir iter and use the fancy error handling if it throws
        self.items = fs::read_dir(&self.current_dir)
            .map_err(|e| AppError::Io(e)
            .with_path_context(&self.current_dir))?
            // We want to filter_map() because we need to exclude some entries (e.g. `.git`) but we also want to create FileItem's
            .filter_map(|entry_result| {  
                // get the entry by converting from result to option (errors = None) and then unwrapping via `?` 
                let entry = entry_result.ok()?;
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Get rid of `.git` since it's just git metadata anyways 
                // Check if this is a file that is hidden or ignored while those are meant to be hidden
                if name == ".git" && !self.should_include_file(&path, &name, path.is_dir()) {
                    return None;
                }

                // Now we "map" and transform our entry results into FileItem's
                Some(FileItem {
                    path: path.clone(),
                    name: name.clone(),
                    is_dir: path.is_dir(),
                    is_symlink: path.is_symlink(),
                    is_hidden: self.is_hidden(&path, &name)
                })

            })
            .collect::<Vec<FileItem>>(); // and now we collect into our self.items vec

        // We still need to do sorting though
        self.items.sort_by(|a, b| {
            match(a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,    // a is ABOVE b
                (false, true) => std::cmp::Ordering::Greater, // a is BELOW b
                _ => a.name.cmp(&b.name),                     // alphanumeric sort
            }
        });

        // Reset to the first item
        self.state.select_first();
    
       Ok(())
    }

    /// Get contextual hints based on current state
    pub fn get_contextual_hint(&self) -> Option<String> {
        // Priority order for hints - show the most relevant one
        
        if self.show_help {
            return Some("Press '?' or ESC to close help".to_string());
        }
        
        // New user hint - no files collected yet
        if self.collected_files.is_empty() {
            if let Some(item) = self.current_selection() {
                if !item.is_dir {
                    return Some("Press 'a' to add this file to your collection".to_string());
                } else {
                    return Some("Press 'A' to add all files in this directory".to_string());
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


    /// Get the currently selected item
    pub fn current_selection(&self) -> Option<&FileItem> {
        self.state
            .selected()
            .and_then(|index| self.items.get(index))
    }

    /// Navigate into selected directory
    pub fn navigate_into(&mut self) -> Result<(), AppError> {
        if let Some(selection) = self.current_selection() {
            if selection.is_dir {
                self.current_dir = selection.path.clone();
                self.refresh_files()?;
            }
        }

        Ok(())
    }

    /// cd ..
    pub fn navigate_up(&mut self) -> Result<(), AppError> {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh_files()?;

        }

        Ok(())
    }


    /// Generate a tree structure string of the current directory
    pub fn generate_tree(&self, max_depth: Option<usize>) -> Result<String, AppError> {
        let mut output = String::new();
        // Print out the current dir we're in as the top line (absolute path)
        output.push_str(&format!("{}\n", self.current_dir.display()));
        // Now we gen the tree structure and chill
        self.generate_tree_recursive(&self.current_dir, &mut output, "", 0, max_depth)?;

        Ok(output)
    }

    

    /// Copy tree to clipboard (requires clipboard feature)
    #[cfg(feature = "clipboard")]
    pub fn copy_tree_to_clipboard(&self) -> Result<(), AppError> {
        use arboard::Clipboard;

        let tree = self.generate_tree(None)?;
        
        Clipboard::new()?
            .set_text(tree)?;

        Ok(())
    }

    /// In case the feature is disabled
    #[cfg(not(feature = "clipboard"))]
    pub fn copy_tree_to_clipboard(&self) -> Result<(), AppError> {
        Err(AppError::UnsupportedOperation(
            "Clipboard support not compiled. Use --features clipboard".to_string(),
        ))
    }

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
    pub fn update_message(&mut self) {
        if let Some(message) = &self.message {
            if message.created_at.elapsed() >= message.timeout {
                self.message = None;
            }
        }
    }
    
    pub fn is_collected(&self, path: &Path) -> bool {
        self.collected_files.iter().any(|f| f.path == path)
    }
    
    pub fn get_collection_size(&self) -> usize {
        self.collected_files.iter().map(|f| f.content.len()).sum()
    }
    
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

    pub fn save_collection_to_file(&mut self, filename: Option<String>) -> Result<(), AppError> {
        if self.collected_files.is_empty() {
            self.set_error_message("Collection is empty".to_string());
            return Ok(());
        }

        let markdown = self.generate_markdown();
        let filename = filename.unwrap_or_else(|| {
            let now = SystemTime::now();
            // Handle potential system time before UNIX_EPOCH
            let since_epoch = now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0));
            format!("code_context_{}.md", since_epoch.as_secs())
        });

        let output_path = self.current_dir.join(&filename);
        std::fs::write(&output_path, markdown)?;

        self.set_success_message(format!("Collection saved to {}", output_path.display()));
        Ok(())
    }

    pub fn copy_collection_to_clipboard(&mut self) -> Result<(), AppError> {
        if self.collected_files.is_empty() {
            self.set_error_message("Collection is empty".to_string());
            return Ok(());
        }

        let markdown = self.generate_markdown();
        
        #[cfg(feature = "clipboard")]
        {
            use arboard::Clipboard;
            Clipboard::new()?.set_text(markdown)?;
            self.set_success_message("Collection copied to clipboard!".to_string());
        }
        
        #[cfg(not(feature = "clipboard"))]
        {
            return Err(AppError::UnsupportedOperation(
                "Clipboard support not compiled. Use --features clipboard".to_string(),
            ));
        }
        
        Ok(())
    }
}


/// File Collection functions for App
impl App {
    pub fn add_current_file(&mut self) -> Result<(), AppError> {
        let current_item = match self.current_selection() {
            Some(item) => item,
            None => {
                self.set_error_message("No file selected".to_string());
                return Ok(());
            }
        };

        if current_item.is_dir {
            self.set_error_message("Cannot collect directories".to_string());
            return Ok(());
        }

        // Create collected file with better error handling
        let new_collected_file = match self.create_collected_file(current_item) {
            Ok(file) => file,
            Err(e) => {
                // Provide specific, actionable error messages
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

        // Calculate size before any borrows
        let size_kb = new_collected_file.content.len() / 1024;
        let name = current_item.name.clone();

        // Use a temporary for position lookup
        let existing_index = self.collected_files
            .iter()
            .position(|f| f.path == new_collected_file.path);
        
        let old_count = self.collected_files.len();

        match existing_index {
            Some(index) => {
                // Replace existing file
                self.collected_files[index] = new_collected_file;
                
                // Build success message with size warning if applicable
                let mut message = format!(
                    "Updated {} ({} KB) - Total: {} files",
                    name, size_kb, old_count
                );
                
                // Check for size warning after update
                if let Some(warning) = self.get_size_warning() {
                    message.push_str(&format!(" | {}", warning));
                }
                
                self.set_success_message(message);
            }
            None => {
                // Add new file
                self.collected_files.push(new_collected_file);
                
                // Build success message with size warning if applicable
                let mut message = format!(
                    "Added {} ({} KB) - Total: {} files",
                    name, size_kb, old_count + 1
                );
                
                // Check for size warning after addition
                if let Some(warning) = self.get_size_warning() {
                    message.push_str(&format!(" | {}", warning));
                }
                
                self.set_success_message(message);
            }
        }

        Ok(())
    }
        
    pub fn add_all_files_in_dir(&mut self) -> Result<(), AppError> {
        let mut added = 0;
        let mut updated = 0;
        let mut skipped = 0;
        let mut errors = 0;

        // Store initial size to detect if we're crossing thresholds
        let initial_size = self.get_collection_size();

        for item in &self.items {
            if item.is_dir {
                skipped += 1;
                continue;
            }

            // Check if file is already collected
            if let Some(index) = self.collected_files.iter().position(|f| f.path == item.path) {
                // File already exists - update it
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
                // New file - add to collection
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

        let total = self.collected_files.len();
        let current_size = self.get_collection_size();
        let size_str = self.format_size(current_size);
        
        // Build the base message
        let mut message = format!(
            "Added {} files, updated {}, skipped {} (errors: {}) - Total: {} files ({})",
            added, updated, skipped, errors, total, size_str
        );
        
        // Add size warning if applicable
        if let Some(warning) = self.get_size_warning() {
            message.push_str(&format!("\n{}", warning));
            
            // Special case: if we crossed from safe to warning/critical in one operation
            const WARNING_THRESHOLD: usize = 25 * MEGABYTE;
            if initial_size < WARNING_THRESHOLD && current_size >= WARNING_THRESHOLD {
                message.push_str("\nTip: Use 'd' to remove individual files or 'D' to clear all");
            }
        }
        
        self.set_success_message(message);

        Ok(())
    }
    
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

        // Clone path to avoid borrowing issues
        let path_to_remove = current_item.path.clone();
        let name = current_item.name.clone();
        
        // Find index without holding a reference to collected_files
        let index = self.collected_files.iter().position(|f| f.path == path_to_remove);
        
        if let Some(index) = index {
            // Remove the file from the collection
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

    pub fn generate_markdown(&self) -> String {
        let mut output = String::new();
        
        // Add a header explaining what this is
        output.push_str("# Code Context\n\n");
        output.push_str(&format!("Generated from: {}\n\n", 
            self.git_root.as_ref().unwrap_or(&self.start_dir).display()));
        
        // For each collected file
        for file in &self.collected_files {
            // Add file header
            output.push_str(&format!("\n## {}\n\n", file.relative_path));
            
            // Add code block with syntax highlighting
            output.push_str(&format!("````{}", file.language));
            output.push_str(&file.content);
            output.push_str("\n````\n");
        }
        
        output
    }

    /// Check if a collected file has changed on disk
    pub fn check_file_status(&self, collected: &CollectedFile) -> FileStatus {
        // First check if the file still exists
        if !collected.path.exists() {
            return FileStatus::Deleted;
        }
        
        // Check if it's still a file (not replaced by a directory)
        if !collected.path.is_file() {
            return FileStatus::NotAFile;
        }
        
        // Check modification time
        match std::fs::metadata(&collected.path) {
            Ok(metadata) => {
                match metadata.modified() {
                    Ok(modified) => {
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
    fn refresh_collected_file(&mut self, index: usize) -> Result<RefreshResult, AppError> {
        if index >= self.collected_files.len() {
            return Err(AppError::LogicError("Invalid collection index".to_string()));
        }
        
        let old_file = &self.collected_files[index];
        let status = self.check_file_status(old_file);
        
        match status {
            FileStatus::Unchanged => Ok(RefreshResult::NoChange),
            FileStatus::Modified => {
                                // Create a temporary FileItem to reuse our existing logic
                                let temp_item = FileItem {
                                    path: old_file.path.clone(),
                                    name: old_file.path.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                    is_dir: false,
                                    is_symlink: false,
                                    is_hidden: false,
                                };
                
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
    pub fn refresh_all_collected(&mut self) -> RefreshSummary {
        let mut summary = RefreshSummary::default();
        let mut indices_to_remove = Vec::new();

        // Use indices instead of iter_mut to avoid holding a mutable reference
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

        // Remove in reverse order to maintain valid indices
        indices_to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices_to_remove {
            self.collected_files.remove(index);
        }

        summary
    }

    pub fn get_display_path(&self, path: &Path) -> String {
        // This method provides a shorter, more readable path for status messages
        // It prioritizes showing the most relevant context
        
        if let Ok(rel_path) = path.strip_prefix(&self.current_dir) {
            // If file is in current directory, just show the filename
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
}

/// Util functions for App
impl App {
    /// Check collection size and return appropriate warning message
    fn get_size_warning(&self) -> Option<String> {
        let size = self.get_collection_size();
        
        // Define our warning thresholds
        const WARNING_THRESHOLD: usize = 25 * MEGABYTE;
        const CRITICAL_THRESHOLD: usize = 50 * MEGABYTE;
        
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

    /// Create a CollectedFile with full metadata for change tracking
    fn create_collected_file(&self, item: &FileItem) -> Result<CollectedFile, AppError> {
        if item.is_dir {
            return Err(AppError::NotAFile);
        }
        
        // Get file metadata before reading content
        let metadata = std::fs::metadata(&item.path)?;
        let last_modified = metadata.modified()?;
        let file_size = metadata.len();
        
        // Try to read the file content
        let content = read_file_safely(&item.path, 10 * MEGABYTE)?;
        
        // Calculate content hash for quick comparison later
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let content_hash = hasher.finish();
        
        // Calculate relative path - but now we handle edge cases better
        let relative_path = self.calculate_relative_path(&item.path)?;
        
        // Get the language for syntax highlighting
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
    
    /// Calculate relative path with better error handling
    fn calculate_relative_path(&self, path: &Path) -> Result<String, AppError> {
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


    fn generate_tree_recursive(
        &self,
        dir: &Path,
        output: &mut String,
        prefix: &str, 
        depth: usize,
        max_depth: Option<usize>,
    ) -> Result<(), AppError> {

        // if we hit the depth then return
        if let Some(max) = max_depth {
            if depth >= max {
                return Ok(());
            }
        }

        // Since we are using recursion we will need to update our entries as we go through the tree -- no need to 
        let mut entries = fs::read_dir(dir)?
            .filter_map(|entry_result| entry_result.ok())
            .filter(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                name != ".git" && self.should_include_file(&path, &name, path.is_dir())
            })
            .collect::<Vec<fs::DirEntry>>();

        // Sort entries: directories first (false < true due to `!`), then alphanumerically by name.
        entries.sort_by_key(|entry| (!entry.path().is_dir(), entry.file_name()));

        let total_entries = entries.len();

        // Now we need to go through and draw out the items as well as thei tree visualization 
        for (i, entries) in entries.iter().enumerate() {
            let is_last_entry = total_entries - 1 == i;
            let path = entries.path();
            let name = entries.file_name().to_string_lossy().to_string();

            // Tree drawing characters
            let connector = if is_last_entry { "└── " } else { "├── " };
            let extension = if is_last_entry { "    " } else { "│   " };

            output.push_str(prefix);
            output.push_str(connector);

            if path.is_dir() {
                output.push_str(&name);
                output.push('/');
                output.push('\n');

                // Recurse into subdirectory
                let new_prefix = format!("{}{}", prefix, extension);
                self.generate_tree_recursive(
                    &path, 
                    output, 
                    &new_prefix, 
                    depth + 1, 
                    max_depth
                )?;
            }
            else {
                output.push_str(&name);
                output.push('\n');
            }
        }

        
        Ok(())
    }

    /// Determine if we should include the files based on hidden and gitignore status
    fn should_include_file(&self, path: &Path, name: &str, is_dir: bool) -> bool {
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
    fn is_hidden(&self, path: &Path, name: &str) -> bool {
        // Windows: we need to check file metadata/attributes
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if let Ok(metadata) = path.metadata() {
                let attributes = metadata.file_attributes();
                if (attributes & 2) != 0 {
                    return true;
                }
            }
        }

        #[cfg(not(windows))]
        let _ = path;

        // -- else AND for linux/unix we can just check for the `.` prefix 
        name.starts_with('.')
    }
}