use std::{fs::{self}, path::{Path, PathBuf}, time::{Duration, Instant}};

use ignore::gitignore::Gitignore;
use ratatui::widgets::ListState;

use crate::{app_error::AppError, utils::{find_repo, get_file_type, read_file_safely, MEGABYTE}};


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
    pub relative_path: String, // Path relative to git root or current working directory
    pub content: String,
    pub language: String,      // For markdown code block highlighting
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

// Util functions for App
impl App {
    /// Convert a FileItem to a CollectedFile, using our knowledge of the base directory d
    fn create_collected_file(&self, item: &FileItem) -> Result<CollectedFile, AppError> {
        // First, check if this is even a file (not a directory)
        if item.is_dir {
            return Err(AppError::NotAFile);
        }
        
        // Try to read the file content
        let content = read_file_safely(&item.path, 10 * MEGABYTE)?
            .ok_or_else(|| AppError::FileReadFailure)?;
        
        // Calculate the relative path
        // Use git root if available, otherwise use the directory where we started
        let base_path = self.git_root.as_ref().unwrap_or(&self.start_dir);
        
        let relative_path = match item.path.strip_prefix(base_path) {
            Ok(rel_path) => {
                // Successfully got a relative path
                rel_path.to_string_lossy().to_string()
            }
            Err(_) => {
                // File is outside our base directory
                // This could happen if they navigated up past the git root
                // In this case, we might want to use the full path
                // or calculate relative to current_dir instead
                item.path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| item.path.display().to_string())
            }
        };
        
        // Get the language for syntax highlighting
        let language = get_file_type(&item.path)
            .unwrap_or("plaintext")
            .to_string();
        
        Ok(CollectedFile {
            path: item.path.clone(),
            relative_path,
            content,
            language,
        })
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