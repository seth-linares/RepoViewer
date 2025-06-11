mod state;
mod collection;
mod export;
mod navigation;

// Re-export important types so they're accessible as `app::TypeName`
// This maintains the same public API that other files expect
pub use state::Message;
pub use collection::CollectedFile;

// Standard library imports
use std::{
    fs::{self},
    path::{PathBuf},
};

// External crate imports
use ignore::gitignore::Gitignore;
use ratatui::widgets::ListState;

// Internal imports from our project
use crate::{
    app_error::AppError,
    utils::{find_repo},
};

/// Main application state
/// This struct holds all the data our TUI needs to function
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
/// This is what we display in our file list
#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
}

// Core implementation - the fundamental methods that set up our app
impl App {
    /// Create a new App instance with the given starting directory
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

    /// Refresh the file list for the current directory
    /// This is called whenever we navigate or toggle visibility options
    pub fn refresh_files(&mut self) -> Result<(), AppError> {
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

                // Always exclude .git directory
                if name == ".git" {
                    return None;
                }
                
                // Check visibility settings for all other files
                if !self.should_include_file(&path, &name, path.is_dir()) {
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

    /// Get the currently selected item from the file list
    pub fn current_selection(&self) -> Option<&FileItem> {
        self.state
            .selected()
            .and_then(|index| self.items.get(index))
    }

    /// Navigate into the selected directory
    pub fn navigate_into(&mut self) -> Result<(), AppError> {
        if let Some(selection) = self.current_selection() {
            if selection.is_dir {
                self.current_dir = selection.path.clone();
                self.refresh_files()?;
            }
        }

        Ok(())
    }

    /// Navigate up to the parent directory (cd ..)
    pub fn navigate_up(&mut self) -> Result<(), AppError> {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh_files()?;
        }

        Ok(())
    }
}