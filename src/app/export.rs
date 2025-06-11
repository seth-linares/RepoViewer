//! Export and output functionality for the RepoViewer application.
//! 
//! This module handles all operations related to exporting collected files
//! and directory structures. It includes markdown generation, file saving,
//! clipboard operations, and tree visualization.

use super::App;
use crate::app_error::AppError;
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

impl App {
    /// Generate a markdown document from all collected files
    /// 
    /// This creates a structured markdown document that's optimized for LLM consumption.
    /// Each file is presented with a clear header showing its path, followed by a
    /// syntax-highlighted code block containing the file's content.
    pub fn generate_markdown(&self) -> String {
        let mut output = String::new();
        
        // Add a header explaining what this document contains
        // This context helps both humans and LLMs understand the document's purpose
        output.push_str("# Code Context\n\n");
        output.push_str(&format!("Generated from: {}\n\n", 
            self.git_root.as_ref().unwrap_or(&self.start_dir).display()));
        
        // For each collected file, create a section with proper formatting
        for file in &self.collected_files {
            // Add file header - using ## makes it easy to navigate between files
            output.push_str(&format!("\n## {}\n\n", file.relative_path));
            
            // Add code block with syntax highlighting
            // The language hint helps both markdown renderers and LLMs understand the code
            output.push_str(&format!("```{}\n", file.language));
            output.push_str(&file.content);
            // Ensure the code block is properly closed even if file doesn't end with newline
            if !file.content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("```\n");
        }
        
        output
    }

    /// Save the collection to a markdown file in the current directory
    /// 
    /// If no filename is provided, generates one with a timestamp to avoid
    /// overwriting existing files. This makes it safe to export multiple times.
    pub fn save_collection_to_file(&mut self, filename: Option<String>) -> Result<(), AppError> {
        // Check if we have anything to save
        if self.collected_files.is_empty() {
            self.set_error_message("Collection is empty".to_string());
            return Ok(());
        }

        // Generate the markdown content
        let markdown = self.generate_markdown();
        
        // Create filename with timestamp if not provided
        // This ensures we never accidentally overwrite previous exports
        let filename = filename.unwrap_or_else(|| {
            let now = SystemTime::now();
            // Handle potential system time before UNIX_EPOCH gracefully
            let since_epoch = now.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0));
            format!("code_context_{}.md", since_epoch.as_secs())
        });

        // Save to the current directory where the user is browsing
        let output_path = self.current_dir.join(&filename);
        fs::write(&output_path, markdown)?;

        // Provide feedback with the full path so users know where to find it
        self.set_success_message(format!("Collection saved to {}", output_path.display()));
        Ok(())
    }

    /// Copy the markdown collection to the system clipboard
    /// 
    /// This is often the most convenient way to share code with LLMs,
    /// as you can paste directly into a chat interface.
    pub fn copy_collection_to_clipboard(&mut self) -> Result<(), AppError> {
        // Check if we have anything to copy
        if self.collected_files.is_empty() {
            self.set_error_message("Collection is empty".to_string());
            return Ok(());
        }

        let markdown = self.generate_markdown();
        
        // Handle clipboard operations with feature flag
        // This allows the crate to compile without clipboard support if needed
        #[cfg(feature = "clipboard")]
        {
            use arboard::Clipboard;
            // Create a new clipboard context for this operation
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

    /// Generate a tree structure string of the current directory
    /// 
    /// This creates a visual representation of the directory structure,
    /// similar to the Unix 'tree' command. It's useful for giving LLMs
    /// an overview of project structure without including file contents.
    pub fn generate_tree(&self, max_depth: Option<usize>) -> Result<String, AppError> {
        let mut output = String::new();
        
        // Start with the absolute path as context
        // This helps users understand exactly what directory structure they're looking at
        output.push_str(&format!("{}\n", self.current_dir.display()));
        
        // Recursively build the tree structure
        self.generate_tree_recursive(&self.current_dir, &mut output, "", 0, max_depth)?;

        Ok(output)
    }

    /// Recursive helper function to build the tree structure
    /// 
    /// This function does the heavy lifting of creating the ASCII art tree.
    /// It handles:
    /// - Proper indentation with tree drawing characters
    /// - Respecting visibility settings (hidden files, gitignored files)
    /// - Depth limiting to prevent extremely deep trees
    /// - Sorting entries (directories first, then alphabetically)
    fn generate_tree_recursive(
        &self,
        dir: &Path,
        output: &mut String,
        prefix: &str, 
        depth: usize,
        max_depth: Option<usize>,
    ) -> Result<(), AppError> {
        // Check if we've reached the maximum depth
        if let Some(max) = max_depth {
            if depth >= max {
                return Ok(());
            }
        }

        // Read directory entries and filter based on visibility settings
        let mut entries = fs::read_dir(dir)?
            .filter_map(|entry_result| entry_result.ok())
            .filter(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                // Respect the same visibility rules as the main file list
                name != ".git" && self.should_include_file(&path, &name, path.is_dir())
            })
            .collect::<Vec<fs::DirEntry>>();

        // Sort entries: directories first, then alphabetically by name
        // This consistent ordering makes the tree easier to read
        entries.sort_by_key(|entry| (!entry.path().is_dir(), entry.file_name()));

        let total_entries = entries.len();

        // Draw each entry with appropriate tree characters
        for (i, entry) in entries.iter().enumerate() {
            let is_last_entry = i == total_entries - 1;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Tree drawing characters:
            // ├── for entries that have siblings below them
            // └── for the last entry in a directory
            let connector = if is_last_entry { "└── " } else { "├── " };
            // Continue the vertical line for non-last entries
            let extension = if is_last_entry { "    " } else { "│   " };

            // Build the line with proper prefix
            output.push_str(prefix);
            output.push_str(connector);

            if path.is_dir() {
                // Add directory name with trailing slash for clarity
                output.push_str(&name);
                output.push('/');
                output.push('\n');

                // Recurse into subdirectory with extended prefix
                // The prefix maintains the tree structure visually
                let new_prefix = format!("{}{}", prefix, extension);
                self.generate_tree_recursive(
                    &path, 
                    output, 
                    &new_prefix, 
                    depth + 1, 
                    max_depth
                )?;
            } else {
                // Just add the filename for regular files
                output.push_str(&name);
                output.push('\n');
            }
        }
        
        Ok(())
    }

    /// Copy the directory tree to clipboard
    /// 
    /// This provides a quick way to share project structure without
    /// having to save it to a file first.
    #[cfg(feature = "clipboard")]
    pub fn copy_tree_to_clipboard(&self) -> Result<(), AppError> {
        use arboard::Clipboard;

        let tree = self.generate_tree(None)?;
        
        Clipboard::new()?
            .set_text(tree)?;

        Ok(())
    }

    /// Fallback implementation when clipboard feature is disabled
    #[cfg(not(feature = "clipboard"))]
    pub fn copy_tree_to_clipboard(&self) -> Result<(), AppError> {
        Err(AppError::UnsupportedOperation(
            "Clipboard support not compiled. Use --features clipboard".to_string(),
        ))
    }
}