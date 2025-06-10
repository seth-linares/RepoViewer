mod app;
mod utils;
mod app_error;
mod ui;

use std::{
    io::stdout,
    path::PathBuf,
    time::Duration,
};

use app::App;
use app_error::AppError;
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::{ui::UI, utils::parse_target_dir};





/// RepoViewer - A TUI file explorer for generating directory trees for LLMs
#[derive(Parser, Debug)]
#[command(name = "repoviewer")]
#[command(version, about, long_about = None)]
struct Args {
    /// Target directory (default: current directory)
    path: Option<String>,

    /// Generate tree output immediately and exit
    #[arg(short, long)]
    tree: bool,

    /// Maximum depth for tree generation
    #[arg(short, long, default_value = "10")]
    depth: Option<usize>,

    /// Show hidden files in tree output
    #[arg(long)]
    hidden: bool,

    /// Show gitignored files in tree output
    #[arg(long)]
    all: bool,
}

fn main() -> Result<(), AppError> {
    let args = Args::parse();

    let target_dir = parse_target_dir(args.path)?;

    // If tree flag is set generate and exit
    if args.tree {
        let mut app = App::new(target_dir)?;
        if args.hidden {
            app.show_hidden = true;
        }

        if args.all {
            app.show_gitignored = true;
        }
        app.refresh_files()?;

        let tree = app.generate_tree(args.depth)?;
        println!("{}", tree);
        return Ok(());
    }

    // Otherwise run tui
    run_tui(target_dir)
}


fn run_tui(target_dir: PathBuf) -> Result<(), AppError> {
    // Setup terminal 
    enable_raw_mode()?;

    let mut stdout = stdout();
    
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize the app
    let mut app = App::new(target_dir)?;

    // Run the app
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<(), AppError> {
    loop {
        // Update message state (clear if timeout elapsed)
        app.update_message();
        
        // Draw UI
        terminal.draw(|frame| UI::render(frame, app))?;

        // Handle events
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // Quit
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),

                        // Toggle hidden files
                        KeyCode::Char('h') => {
                            app.show_hidden = !app.show_hidden;
                            app.refresh_files()?;
                        }

                        // Toggle gitignored files (only if in git repo)
                        KeyCode::Char('g') if app.git_root.is_some() => {
                            app.show_gitignored = !app.show_gitignored;
                            app.refresh_files()?;
                        }

                        // Save tree to file
                        KeyCode::Char('t') => {
                            let tree = app.generate_tree(None)?;
                            let output_file = app.current_dir.join("tree.txt");
                            std::fs::write(&output_file, tree)?;
                            app.set_success_message(format!("Tree saved to {}", output_file.display()));
                        }
                        
                        // Copy tree to clipboard
                        KeyCode::Char('c') => {
                            match app.copy_tree_to_clipboard() {
                                Ok(_) => app.set_success_message("Tree copied to clipboard!".to_string()),
                                Err(e) => app.set_error_message(format!("Clipboard error: {}", e)),
                            }
                        }

                        // --- Collection Controls ---
                        
                        // Add current file to collection
                        KeyCode::Char('a') => app.add_current_file()?,

                        // Add all files in current directory to collection
                        KeyCode::Char('A') => app.add_all_files_in_dir()?,

                        // Remove current file from collection
                        KeyCode::Char('d') => app.remove_current_file()?,
                        
                        // Clear entire collection
                        KeyCode::Char('D') => app.clear_collection()?,

                        // Save collection to markdown file
                        KeyCode::Char('S') => {
                            if let Err(e) = app.save_collection_to_file(None) {
                                app.set_error_message(format!("Failed to save file: {}", e));
                            }
                        },

                        // Refresh collected files to sync with filesystem changes
                        KeyCode::Char('r') => {
                            // First, check if we even have files to refresh
                            if app.collected_files.is_empty() {
                                app.set_error_message("No files in collection to refresh".to_string());
                            } else {
                                // Store the initial count to detect net changes
                                let initial_count = app.collected_files.len();
                                
                                // Perform the refresh operation
                                let summary = app.refresh_all_collected();
                                
                                // Now we need to craft a meaningful message
                                // Let's think about what users care about:
                                // 1. Did anything change?
                                // 2. What specifically changed?
                                // 3. Were there any problems?
                                
                                // Case 1: Nothing changed at all - reassure the user
                                if summary.updated == 0 && summary.deleted == 0 && 
                                summary.failed == 0 && summary.inaccessible == 0 {
                                    app.set_success_message(format!(
                                        "✓ Collection is up to date ({} files checked)", 
                                        summary.unchanged
                                    ));
                                } 
                                // Case 2: Changes occurred - detail what happened
                                else {
                                    // Build a list of what changed
                                    let mut changes = Vec::new();
                                    
                                    if summary.updated > 0 {
                                        changes.push(format!("{} updated", summary.updated));
                                    }
                                    if summary.deleted > 0 {
                                        changes.push(format!("{} deleted", summary.deleted));
                                    }
                                    if summary.inaccessible > 0 {
                                        changes.push(format!("{} inaccessible", summary.inaccessible));
                                    }
                                    if summary.failed > 0 {
                                        changes.push(format!("{} failed", summary.failed));
                                    }
                                    
                                    // Determine if this is a success or partial failure
                                    let has_problems = summary.failed > 0 || summary.inaccessible > 0;
                                    let final_count = app.collected_files.len();
                                    
                                    // Craft the message with appropriate tone
                                    let message = format!(
                                        "Refresh complete: {} | {} → {} files",
                                        changes.join(", "),
                                        initial_count,
                                        final_count
                                    );
                                    
                                    if has_problems {
                                        app.set_error_message(message);
                                    } else {
                                        app.set_success_message(message);
                                    }
                                }
                            }
                        }

                        // Copy collection to clipboard
                        KeyCode::Char('C') => {
                            if let Err(e) = app.copy_collection_to_clipboard() {
                                app.set_error_message(format!("Clipboard error: {}", e));
                            }
                        },

                        // Navigation
                        KeyCode::Up => {
                            if let Some(selected) = app.state.selected() {
                                if selected > 0 {
                                    app.state.select(Some(selected - 1));
                                }
                            }
                        }

                        KeyCode::Down => {
                            if let Some(selected) = app.state.selected() {
                                if selected < app.items.len().saturating_sub(1) {
                                    app.state.select(Some(selected + 1));
                                }
                            }
                        }

                        KeyCode::Left => app.navigate_up()?,
                        KeyCode::Right | KeyCode::Enter => app.navigate_into()?,

                        // Home/PageUp - go to the first item
                        KeyCode::Home | KeyCode::PageUp => {
                            if !app.items.is_empty() {
                                app.state.select(Some(0));
                            }
                        }

                        // End/PageDown - go to last item
                        KeyCode::End | KeyCode::PageDown => {
                            if !app.items.is_empty() {
                                app.state.select(Some(app.items.len() - 1));
                            }
                        }

                        _ => {}
                    }
                }
            }
        }
    }
}

