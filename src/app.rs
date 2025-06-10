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

        // Create collected file early
        let new_collected_file = match self.create_collected_file(current_item) {
            Ok(file) => file,
            Err(AppError::NotAFile) => {
                self.set_error_message("Cannot collect: not a text file".to_string());
                return Ok(());
            }
            Err(e) => {
                self.set_error_message(format!("Failed to read file: {}", e));
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
                self.set_success_message(format!(
                    "Updated {name} ({size_kb} KB) - Total: {old_count} files"
                ));
            }
            None => {
                // Add new file
                self.collected_files.push(new_collected_file);
                self.set_success_message(format!(
                    "Added {name} ({size_kb} KB) - Total: {} files",
                    old_count + 1
                ));
            }
        }

        Ok(())
    }
    
    pub fn add_all_files_in_dir(&mut self) -> Result<(), AppError> {
        // Iterate through current directory, add all text files
        todo!()
    }
    
    pub fn remove_current_file(&mut self) -> Result<(), AppError> {
        // Find and remove from collected_files vec
        todo!()
    }
    
    pub fn clear_collection(&mut self) -> Result<(), AppError> {
        // Simply clear the vec and show message
        todo!()
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
            output.push_str(&format!("```{}\n", file.language));
            output.push_str(&file.content);
            output.push_str("\n```\n");
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
    pub fn refresh_collected_file(&mut self, index: usize) -> Result<RefreshResult, AppError> {
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
}

/// Util functions for App
impl App {
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
        let content = read_file_safely(&item.path, 10 * MEGABYTE)?
            .ok_or_else(|| AppError::FileReadFailure)?;
        
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
        
        // If all else fails, use just the filename
        Ok(path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string()))
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
                // FILE_ATTRIBUTE_HIDDEN = 0x2 so we check if there is 2
                if (attributes & 2) != 0 {
                    return true;
                }
            }
        }

        // -- else AND for linux/unix we can just check for the `.` prefix 
        name.starts_with('.')
    }
}