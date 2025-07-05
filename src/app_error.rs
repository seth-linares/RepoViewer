// src/app_error.rs

use std::{io, path::Path, sync::PoisonError};
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

    /// Synchronization errors (mutex poisoning)
    #[error("Synchronization error: {0}")]
    SyncError(String),

    // --- Self-update related errors ---

    /// Wrap all self_update crate errors
    /// This gives us access to the full range of update-specific errors
    /// without duplicating their logic
    #[error("Update error: {0}")]
    SelfUpdate(#[from] self_update::errors::Error),

    /// Update check/install was cancelled by user
    #[error("Update cancelled by user")]
    UpdateCancelled,

    /// No update available (not really an error, but useful for control flow)
    #[error("Already running the latest version")]
    NoUpdateAvailable,

    /// Update cache-related errors that aren't covered by IO errors
    #[error("Update cache error: {0}")]
    UpdateCache(String),

    /// Platform-specific update errors
    #[error("Update not supported on this platform: {0}")]
    UnsupportedPlatform(String),

    /// Errors related to version parsing or comparison
    /// (beyond what semver provides)
    #[error("Version error: {0}")]
    VersionError(String),

    // --- Application-specific errors (unchanged) ---

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

    #[cfg(feature = "clipboard")]
    #[error("Clipboard not initialized")]
    ClipboardNotInitialized,

    /// A catch-all for other specific errors in your application logic
    #[error("Application logic error: {0}")]
    LogicError(String),

    /// Feature not supported error (used when clipboard is disabled)
    #[cfg(not(feature = "clipboard"))]
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

// Generic implementation for any PoisonError
// We can't use thiserror since we need a generic for PoisonError
// (could make a struct and do weird conversions, but it's not worth)
impl<T> From<PoisonError<T>> for AppError {
    fn from(err: PoisonError<T>) -> Self {
        AppError::SyncError(format!("Mutex poisoned: {}", err))
    }
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

    /// Helper to check if this is a network-related error
    /// Useful for update operations to provide better user feedback
    pub fn is_network_error(&self) -> bool {
        match self {
            AppError::SelfUpdate(self_update::errors::Error::Network(_)) => true,
            AppError::SelfUpdate(self_update::errors::Error::Reqwest(_)) => true,
            AppError::Io(e) if e.kind() == io::ErrorKind::NetworkUnreachable => true,
            AppError::Io(e) if e.kind() == io::ErrorKind::ConnectionRefused => true,
            _ => false,
        }
    }

    /// Helper to determine if the error is related to permissions
    /// Common when trying to update the executable
    pub fn is_permission_error(&self) -> bool {
        match self {
            AppError::Io(e) if e.kind() == io::ErrorKind::PermissionDenied => true,
            AppError::SelfUpdate(self_update::errors::Error::Io(e)) 
                if e.kind() == io::ErrorKind::PermissionDenied => true,
            _ => false,
        }
    }

    /// Convert update errors to user-friendly messages
    /// This helps maintain consistent messaging across the app
    pub fn user_friendly_message(&self) -> String {
        match self {
            AppError::SelfUpdate(e) => match e {
                self_update::errors::Error::Network(_) => 
                    "Unable to check for updates: No internet connection".to_string(),
                self_update::errors::Error::Release(msg) => 
                    format!("Update check failed: {}", msg),
                self_update::errors::Error::Io(io_err) if io_err.kind() == io::ErrorKind::PermissionDenied => 
                    "Update failed: Permission denied. Try running with elevated permissions".to_string(),
                _ => format!("Update error: {}", e),
            },
            AppError::UpdateCancelled => "Update cancelled".to_string(),
            AppError::NoUpdateAvailable => "You're already running the latest version!".to_string(),
            AppError::UnsupportedPlatform(platform) => 
                format!("Updates not available for {} - please build from source", platform),
            AppError::SyncError(msg) => 
                format!("Internal synchronization error: {}. This is likely a bug - please restart the application", msg),
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

    /// Helper to determine if this is a temporary clipboard error that could be retried
    #[cfg(feature = "clipboard")]
    pub fn is_clipboard_busy(&self) -> bool {
        matches!(self, AppError::Clipboard(arboard::Error::ClipboardOccupied))
    }

    /// Helper to check if clipboard operation failed due to unsupported feature
    #[cfg(feature = "clipboard")]
    pub fn is_clipboard_unsupported(&self) -> bool {
        matches!(self, AppError::Clipboard(arboard::Error::ClipboardNotSupported))
    }

    /// Helper to check if this is a synchronization error
    pub fn is_sync_error(&self) -> bool {
        matches!(self, AppError::SyncError(_))
    }
}