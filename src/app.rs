use std::{path::{Path, PathBuf}};

use git2::Repository;
use ignore::gitignore::Gitignore;
use ratatui::widgets::ListState;

use crate::app_error::AppError;

trait WithPath {
    fn with_path(self, path: &Path) -> Self;
}

impl<T> WithPath for Result<T, AppError> {
    fn with_path(self, path: &Path) -> Self {
        self.map_err(|e| match e {
            AppError::Io(io_err) => AppError::Io(io_err).with_path_context(path),
            other => other,
        })
    }
}


/// Main application state
struct App {
    pub current_dir: PathBuf,      // Currently viewed directory
    pub git_root: Option<PathBuf>, // Root of Git repository (if any)
    pub items: Vec<FileItem>,      // Files in current directory
    pub state: ListState,          // Tracks selected item in list
    pub gitignore: Option<Gitignore>, // .gitignore rules (if in Git repo)
    pub show_hidden: bool,         // Whether to show hidden files
    pub show_gitignored: bool,     // Whether to show gitignored files
}


/// Represents a file system entry
#[derive(Debug)]
struct FileItem {
    path: PathBuf,             // Full path to item
    name: String,              // Display name
    is_dir: bool,              // Is a directory?
    is_symlink: bool,          // Is a symbolic link?
    is_hidden: bool,           // Is a hidden file?
}


impl App {
    pub fn new(start_dir: PathBuf) -> Result<Self, AppError> {
        let (git_root, gitignore) = match Repository::discover(&start_dir) {
            Ok(repo) => {
                let root = repo.path().parent()
                    .ok_or_else(|| AppError::GitRepoNoParent)?
                    .to_path_buf();
            
                // Handle gitignore errors gracefully
                let (gitignore, err) = Gitignore::new(root.join(".gitignore"));
                if let Some(err) = err {
                    return Err(err.into())
                }

                (Some(root), Some(gitignore))

            },
            Err(_) => (None, None),
        };

        let mut app = App {
            current_dir: start_dir,
            git_root,
            items: Vec::new(),
            state: ListState::default(),
            gitignore,
            show_hidden: false,
            show_gitignored: false,
            
        };

        // Need to fill in items
        // app.refresh_files()?;

        Ok(app)
    }

    fn refresh_files(&mut self) {
        // Clear items out
        self.items.clear();
    }


    fn should_include_files(&self, path: &Path, name: &str) -> bool {


        todo!()
    }

    fn is_hidden(&self, path: &Path, name: &str) -> bool {
        // Windows: we need to check file metadata
        #[cfg(windows)]
        {
            use std::
        }
        
        todo!()
    }

    
}


