// src/app_error.rs

use std::{io, path::Path};
use thiserror::Error;


/// Custom error types for the application
#[derive(Debug, Error)]
pub enum AppError {
    /// I/O related errors (file operations, TUI drawing, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error), // Covers std::io::Error, crossterm errors, and ratatui::Error

    /// Git repository related errors
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// Command line argument parsing error
    #[error("Argument parsing error: {0}")]
    Clap(#[from] clap::Error), // clap::Error is the type alias for clap::error::Error<DefaultFormatter>

    /// Errors from the 'ignore' crate (e.g., parsing .gitignore files)
    #[error("Ignore pattern error: {0}")]
    Ignore(#[from] ignore::Error),

    #[error("Clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),

    // --- Application-specific errors ---

    /// Failed to convert from FileItem to CollectionItem
    #[error("The FileItem is a directory and cannot be collected")]
    NotAFile,


    /// Failed to read from a FileItem during conversion
    #[error("Failed to read from a FileItem")]
    FileReadFailure,

    /// Invalid path provided or determined
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Path exists but is not a directory when a directory was expected
    #[error("Not a directory: {0}")]
    NotADirectory(String),

    /// Expected directory was not found
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    /// Git repository does not have a parent
    #[error("Git repository does not have a parent")]
    GitRepoNoParent,

    /// A catch-all for other specific errors in your application logic
    #[error("Application logic error: {0}")]
    LogicError(String),
}

impl AppError {
    // meant to work with fs and the paths/files we analyze there
    // Essentially we are overriding the io errors and making a custom print out via `io_err`
    pub fn with_path_context(self, path: &Path) -> Self {
        if let AppError::Io(io_err) = self {
            AppError::Io(std::io::Error::new(
                io_err.kind(),
                format!("{} (path: {})", io_err, path.display()),
            ))
        } else {
            self
        }
    }
}