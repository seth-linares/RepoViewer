//! Clipboard handling with proper lifetime management
//! 
//! This module provides a wrapper around arboard that handles platform-specific
//! clipboard behavior correctly avoiding the debug warnings about clipboard
//! lifetime while maintaining efficiency.


#[cfg(feature = "clipboard")]
pub use enabled::*;

#[cfg(not(feature = "clipboard"))]
pub use disabled::*;



#[cfg(feature = "clipboard")]
pub mod enabled {
    use arboard::Clipboard;
    use crate::app_error::AppError;
    #[cfg(target_os = "linux")]
    use std::thread;


    /// Platform-aware clipboard operations that handle lifetime correctly
    pub struct ClipboardManager;

    impl ClipboardManager {
        
        /// Copy text to clipboard with proper lifetime handling
        /// 
        /// On Linux/X11 the clipboard works differently than Windows/macOS.
        /// We need to keep the clipboard "alive" long enough for clipboard
        /// managers to grab the content. This function handles that efficiently
        /// based on the platform.
        pub fn set_text(text: String) -> Result<(), AppError> {
            // Determine platform at compile time for zero-cost abstraction
            #[cfg(target_os = "linux")]
            {
                // See if it's a linux that needs to have a background thread wait to use clipboard
                if Self::detect_linux_environment_needs_wait() {
                    Self::set_text_linux(text)
                } else {
                    // if it's fine just use this
                    Self::set_text_direct(text)
                }
            }
            
            // Could use the cfg! macro BUT it would essentially become an if-else and increase binary size and complexity
            // for basically no reason. Might as well just not compile useless code blocks if on a different OS.
            #[cfg(not(target_os = "linux"))]
            {
                // Windows and macOS have centralized clipboards so no need to worry
                Self::set_text_direct(text)
            }
        }
        
        /// Direct clipboard write for platforms with centralized clipboard
        fn set_text_direct(text: String) -> Result<(), AppError> {

            let mut clipboard = Clipboard::new()?;
            
            clipboard.set_text(text)?;
            
            Ok(())
        }
        

        /// Detect if we're in a Linux environment that needs special clipboard handling
        #[cfg(target_os = "linux")]
        fn detect_linux_environment_needs_wait() -> bool {
            // Check environment variables to determine display backend
            let xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
            let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
            let x11_display = std::env::var("DISPLAY").ok();
            let is_wslg = std::env::var("WSL_DISTRO_NAME").is_ok() || 
                        std::env::var("WSL_INTEROP").is_ok();
            
            // return the actual bools
            match xdg_session.as_deref() {
                Some("x11") => true,
                Some("wayland") => true,
                _ => {
                    // if any of these are true then we also need special handling
                    is_wslg || wayland_display.is_some() || x11_display.is_some()
                }
            }
        }
        /// Linux specific clipboard handling with proper lifetime management
        #[cfg(target_os = "linux")]
        fn set_text_linux(text: String) -> Result<(), AppError> {
            // spawn thread
            thread::spawn(move || {

                // Create a new clipboard instance for this thread
                if let Ok(mut clipboard) = Clipboard::new() {
                    use arboard::SetExtLinux;

                    let _ = clipboard
                        .set()
                        .wait()
                        .text(text);
                }
            });

            // exit thread and drop clipboard
            Ok(())
        }
    }
}

/// Fallback implementation when clipboard feature is disabled
#[cfg(not(feature = "clipboard"))]
pub mod disabled {
    pub struct ClipboardManager;

    impl ClipboardManager {
        pub fn set_text(_text: String) -> Result<(), AppError> {
            Err(AppError::UnsupportedOperation(
                "Clipboard support not compiled. Use --features clipboard".to_string(),
            ))
        }
    }

}


