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

    /// Clipboard operation errors
    #[cfg(feature = "clipboard")]
    #[error("Clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),

    // --- Self-update related errors ---

    /// Wrap all self_update crate errors
    /// This gives us access to the full range of update-specific errors
    /// without duplicating their logic
    #[error("Update error: {0}")]
    SelfUpdate(#[from] self_update::errors::Error),

    /// Update check/install was cancelled by user
    #[error("Update cancelled by user")]
    UpdateCancelled,

    /// Platform-specific update errors
    #[error("Update not supported on this platform: {0}")]
    UnsupportedPlatform(String),

    // --- Application-specific errors ---

    /// Failed to convert from FileItem to CollectionItem
    #[error("The FileItem is a directory and cannot be collected")]
    NotAFile,

    #[error("File too large: {size} bytes (max: {max} bytes)")]
    FileTooLarge { size: u64, max: usize },

    #[error("Binary file detected (contains null bytes)")]
    BinaryFile,

    #[error("Not a recognized text file type: {extension:?}")]
    UnrecognizedFileType { extension: Option<String> },

    #[error("File encoding error: too many invalid UTF-8 sequences")]
    EncodingError,

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

    /// Feature not supported error (used when clipboard is disabled)
    #[cfg(not(feature = "clipboard"))]
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
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

    /// Convert update errors to user-friendly messages
    /// This helps maintain consistent messaging across the app
    pub fn user_friendly_message(&self) -> String {
        match self {
            AppError::SelfUpdate(e) => match e {
                self_update::errors::Error::Network(msg) => {
                    // Check for specific network errors
                    if msg.contains("404") {
                        "No releases found. This might be the first release or the repository might not have any releases yet.\n".to_string()
                    } else if msg.contains("rate limit") {
                        "GitHub API rate limit exceeded. Please try again later.".to_string()
                    } else if msg.contains("timeout") {
                        "Connection timed out. Please check your internet connection and try again.".to_string()
                    } else if msg.contains("api.github.com") {
                        "Unable to connect to GitHub. Please check your internet connection.".to_string()
                    } else {
                        "Network error while checking for updates. Please try again later.".to_string()
                    }
                },
                self_update::errors::Error::Release(msg) => {
                    if msg.contains("No releases") {
                        "No releases available yet.".to_string()
                    } else {
                        format!("Update check failed: {}", msg)
                    }
                },
                self_update::errors::Error::Io(io_err) if io_err.kind() == io::ErrorKind::PermissionDenied => 
                    "Update failed: Permission denied. Try running with elevated permissions.".to_string(),
                self_update::errors::Error::Reqwest(req_err) => {
                    format!("HTTP error while checking for updates: {}", req_err)
                },
                _ => format!("Update error: {}", e),
            },
            AppError::UpdateCancelled => "Update cancelled".to_string(),
            AppError::UnsupportedPlatform(platform) => 
                format!("Updates not available for {} - please build from source", platform),
            #[cfg(feature = "clipboard")]
            AppError::Clipboard(e) => match e {
                arboard::Error::ContentNotAvailable => 
                    "Clipboard is empty or content format not supported".to_string(),
                arboard::Error::ClipboardNotSupported => 
                    "This clipboard type isn't supported on your system".to_string(),
                arboard::Error::ClipboardOccupied => 
                    "Clipboard is being used by another application".to_string(),
                arboard::Error::ConversionFailure => 
                    "Failed to convert content for clipboard".to_string(),
                arboard::Error::Unknown { description } => 
                    format!("Clipboard error: {}", description),
                _ => format!("Clipboard error: {}", e),
            },
            _ => self.to_string(),
        }
    }
}