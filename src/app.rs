use std::{fs, path::{Path, PathBuf}};

use git2::Repository;
use ignore::gitignore::Gitignore;
use ratatui::widgets::ListState;
use tui_input::Input;

use crate::app_error::AppError;

/// Main application state
pub struct App {
    pub current_dir: PathBuf,
    pub git_root: Option<PathBuf>,
    pub items: Vec<FileItem>,
    pub state: ListState,
    pub input: Input,
    pub gitignore: Option<Gitignore>,
    pub show_hidden: bool,
    pub show_gitignored: bool,
}

/// Represents a file system entry
#[derive(Debug)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
}


impl App {
    pub fn new(start_dir: PathBuf) -> Result<Self, AppError> {
        let (git_root, gitignore) = match Repository::discover(&start_dir) {
            Ok(repo) => {
                let root = repo.workdir()
                    .or_else(|| repo.path().parent()) // fallback to git parent
                    .map(|p| p.to_path_buf())
                    .ok_or(AppError::GitRepoNoParent)?;
            
                // Handle gitignore errors gracefully
                let (gitignore, err) = Gitignore::new(root.join(".gitignore"));
                if let Some(err) = err {
                    return Err(err.into());
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
            input: Input::default(),
            gitignore,
            show_hidden: false,
            show_gitignored: false,
            
        };

        app.refresh_files()?;

        Ok(app)
    }

    pub fn refresh_files(&mut self) -> Result<(), AppError>{
        // Clear items out
        self.items.clear();

        let entries = fs::read_dir(&self.current_dir)
            .map_err(|e| AppError::Io(e)
            .with_path_context(&self.current_dir))?;

        for entry_result in entries {
            let entry = entry_result?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip over the `.git` dir since it's just git metadata
            if name == ".git" && path.is_dir() {
                continue;
            }


            // Check if file should be included based on settings
            if self.should_include_file(&path, &name) {
                let metadata = entry.metadata()?;
                let is_dir = metadata.is_dir();
                let is_symlink = metadata.is_symlink();
                let is_hidden = self.is_hidden(&path, &name);

                self.items.push(FileItem { 
                    path, 
                    name, 
                    is_dir, 
                    is_symlink, 
                    is_hidden, 
                });
            }
        }

        // Sort - directories > file or alphabetically
        self.items.sort_by(|a, b| {
            if a.is_dir && !b.is_dir {
                std::cmp::Ordering::Less
            } 
            else if !a.is_dir && b.is_dir {
                std::cmp::Ordering::Greater
            }
            else {
                a.name.cmp(&b.name)
            }
        });

        // Reset selection to the top
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
        else {
            self.state.select(None);
        }
            

       Ok(())
    }


    fn should_include_file(&self, path: &Path, name: &str) -> bool {

        if self.is_hidden(path, name) && !self.show_hidden {
            return false;
        }

        if let Some(ignore) = &self.gitignore {
            match ignore.matched(path, false) {
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

        // else and for linux/unix we can just check for the `.` prefix
        name.starts_with('.')
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

    
}


