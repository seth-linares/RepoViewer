use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect}, 
    style::{Color, Modifier, Style}, 
    text::{Line, Span}, 
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap}, 
    Frame
};

use crate::{app::App, utils::{get_file_display_info, MEGABYTE}};

/// UI is now a stateless renderer - it doesn't hold any data, just contains
/// methods for drawing different parts of the interface
pub struct UI;

impl UI {
    /// Main rendering entry point - this orchestrates drawing all UI components
    ///
    /// This method:
    /// 1. Creates the layout (dividing the screen into sections)
    /// 2. Calls individual render methods for each section
    /// 3. Doesn't store any state - it just draws and returns
    pub fn render(frame: &mut Frame, app: &App) {
        // If help is shown, render it on top of everything
        if app.show_help {
            // First render the normal UI (so it's visible in the background)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(5),
                    Constraint::Length(7), // Increased for contextual hints
                ])
                .split(frame.area());

            Self::render_header(frame, app, chunks[0]);
            Self::render_file_list(frame, app, chunks[1]);
            Self::render_status_bar_with_hints(frame, app, chunks[2]);
            
            // Then render the help overlay on top
            Self::render_help_overlay(frame);
        } else {
            // Normal rendering
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4),
                    Constraint::Min(5),
                    Constraint::Length(7), // Increased for contextual hints
                ])
                .split(frame.area());

            Self::render_header(frame, app, chunks[0]);
            Self::render_file_list(frame, app, chunks[1]);
            Self::render_status_bar_with_hints(frame, app, chunks[2]);
        }

        // Always render message popup if there is one
        if let Some(message) = &app.message {
            Self::render_message(frame, message);
        }
    }

    /// Renders the header section showing current directory and git info
    fn render_header(frame: &mut Frame, app: &App, area: Rect) {
        // Create the header block with borders and title
        let header_block = Block::default()
            .borders(Borders::ALL)
            .title("  RepoViewer  ")
            .title_alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));

        // Build the content lines
        let path_display = format!("📁 {}", app.current_dir.display());
        let mut lines: Vec<Line> = vec![Line::from(path_display)];

        // Add git root info if we're in a git repository
        if let Some(git_root) = &app.git_root {
            lines.push(
                Line::from(format!("🔧 Git root: {}", git_root.display()))
                    .style(Style::default().fg(Color::Green).add_modifier(Modifier::DIM)),
            );
        }

        // Add collection status
        if !app.collected_files.is_empty() {
            let size = app.get_collection_size();
            let size_str = app.format_size(size);
            
            // Determine collection health with visual indicators
            let (indicator, style) = match size {
                s if s > 50 * MEGABYTE => ("⚠️ ", Style::default().fg(Color::Red)),
                s if s > 25 * MEGABYTE => ("⚠️ ", Style::default().fg(Color::Yellow)),
                _ => ("📦 ", Style::default().fg(Color::Green))
            };
            
            lines.push(
                Line::from(vec![
                    Span::styled(indicator, style),
                    Span::raw(format!("Collection: {} files ({})", 
                        app.collected_files.len(), 
                        size_str
                    )),
                ])
            );
        }

        // Create the paragraph widget and render it
        let paragraph = Paragraph::new(lines)
            .block(header_block)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Renders the main file list with selection highlighting
    fn render_file_list(frame: &mut Frame, app: &App, area: Rect) {
        // Convert each file item into a styled list item
        let items: Vec<ListItem> = app
            .items
            .iter()
            .map(|item| {
                let (icon, style) = get_file_display_info(item);
                let is_collected = app.is_collected(&item.path);
                
                // Create the display name with collection indicator
                // We use [+] for collected files and spaces for alignment
                let collection_marker = if is_collected { "[+]" } else { "   " };
                let display_name = format!("{} {} {}", collection_marker, icon, item.name);

                // Keep the background color as a secondary indicator
                // This provides redundancy - users can rely on either visual cue
                let final_style = if is_collected {
                    style.bg(Color::Rgb(50, 50, 50))
                } else {
                    style
                };

                ListItem::new(display_name).style(final_style)
            })
            .collect();

        // Create the file list block
        let title = if app.collected_files.is_empty() {
            format!(" Files [{}] ", app.items.len())
        } else {
            format!(
                " Files [{}] | Collected [{}] ",
                app.items.len(),
                app.collected_files.len()
            )
        };
        let files_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_alignment(Alignment::Center);

        // Create the list widget with highlighting
        let list = List::new(items)
            .block(files_block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(80, 80, 80))  // Lighter than collection background
                    .add_modifier(Modifier::BOLD)
            )
            .highlight_symbol("▶ ");

        // For stateful widgets, we need to clone the state
        // This is because render_stateful_widget needs mutable access to the state
        // but we only have an immutable reference to app
        let mut list_state = app.state.clone();
        frame.render_stateful_widget(list, area, &mut list_state);
    }


    
    /// Renders the status bar with keyboard shortcuts and toggle states
    fn render_status_bar_with_hints(frame: &mut Frame, app: &App, area: Rect) {
        // Split the status area into hint and controls sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Contextual hint
                Constraint::Length(6), // Control information
            ])
            .split(area);
        
        // Render contextual hint if available
        if let Some(hint) = app.get_contextual_hint() {
            let hint_block = Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(Color::DarkGray));
                
            let hint_text = Line::from(vec![
                Span::styled(" 💡 ", Style::default().fg(Color::Yellow)),
                Span::styled(hint, Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)),
            ]);
            
            let hint_paragraph = Paragraph::new(hint_text)
                .block(hint_block)
                .alignment(Alignment::Center);
                
            frame.render_widget(hint_paragraph, chunks[0]);
        }
        
        // Render the regular status bar in the remaining space
        // Update the controls to include help
        let controls = vec![
            // Navigation controls
            Line::from(vec![
                Span::styled("Navigate:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("↑↓", Style::default().fg(Color::Yellow)),
                Span::raw(" Select  "),
                Span::styled("←", Style::default().fg(Color::Yellow)),
                Span::raw(" Back  "),
                Span::styled("→/Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" Open  "),
                Span::styled("Home/PgUp", Style::default().fg(Color::Yellow)),
                Span::raw(" Top  "),
                Span::styled("End/PgDn", Style::default().fg(Color::Yellow)),
                Span::raw(" Bottom"),
            ]),
            // Toggle controls
            Line::from(vec![
                Span::styled("Toggle:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("h", Style::default().fg(Color::Yellow)),
                Span::raw(" Hidden["),
                if app.show_hidden {
                    Span::styled("ON", Style::default().fg(Color::Green))
                } else {
                    Span::styled("OFF", Style::default().fg(Color::Red))
                },
                Span::raw("]  "),
                // Git ignore toggle (grayed out if not in a git repo)
                if app.git_root.is_some() {
                    Span::styled("g", Style::default().fg(Color::Yellow))
                } else {
                    Span::styled("g", Style::default().fg(Color::DarkGray))
                },
                Span::raw(" Gitignore["),
                if app.show_gitignored {
                    Span::styled("SHOW", Style::default().fg(Color::Green))
                } else {
                    Span::styled("HIDE", Style::default().fg(Color::Red))
                },
                Span::raw("]"),
            ]),
            // Tree Export controls
            Line::from(vec![
                Span::styled("Tree Export:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("t", Style::default().fg(Color::Green)),
                Span::raw(" Save tree  "),
                Span::styled("c", Style::default().fg(Color::Green)),
                Span::raw(" Copy tree"),
            ]),
            // Collection controls
            Line::from(vec![
                Span::styled("Collection:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("a", Style::default().fg(Color::Yellow)),
                Span::raw("/"),
                Span::styled("A", Style::default().fg(Color::Yellow)),
                Span::raw(" Add, "),
                Span::styled("d", Style::default().fg(Color::Red)),
                Span::raw("/"),
                Span::styled("D", Style::default().fg(Color::Red)),
                Span::raw(" Remove, "),
                Span::styled("r", Style::default().fg(Color::Cyan)),
                Span::raw(" Refresh | "),
                Span::styled("S", Style::default().fg(Color::Green)),
                Span::raw("/"),
                Span::styled("C", Style::default().fg(Color::Green)),
                Span::raw(" Export"),
            ]),
            // Exit and help control
            Line::from(vec![
                Span::styled("Other:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("?", Style::default().fg(Color::Cyan)),
                Span::raw(" Help  "),
                Span::styled("q/Esc", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]),
        ];
        
        // Render the status block
        let status_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(status_block, chunks[1]);
        
        // Layout for control lines
        let control_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Line 1
                Constraint::Length(1), // Line 2
                Constraint::Length(1), // Line 3
                Constraint::Length(1), // Line 4
                Constraint::Length(1), // Line 5
            ])
            .split(chunks[1]);

        // Render each line of controls
        for (i, line) in controls.iter().enumerate() {
            if i < control_chunks.len() {
                let inner_area = Rect {
                    x: control_chunks[i].x + 1,
                    y: control_chunks[i].y,
                    width: control_chunks[i].width.saturating_sub(2),
                    height: control_chunks[i].height,
                };
                let paragraph = Paragraph::new(line.clone()).style(Style::default());
                frame.render_widget(paragraph, inner_area);
            }
        }
    }

    /// Renders a popup message (success or error)
    fn render_message(frame: &mut Frame, message: &crate::app::Message) {
        let width = message.text.len().min(50) as u16 + 4; // Max width of 50 chars + padding
        let height = 3;

        // Center the popup on screen
        let area = frame.area();
        let popup_area = Rect::new(
            (area.width.saturating_sub(width)) / 2,
            area.height / 2 - 1,
            width,
            height,
        );

        // Create the message block with appropriate color
        let color = if message.success {
            Color::Green
        } else {
            Color::Red
        };
        let block = Block::default()
            .title(if message.success {
                " Success "
            } else {
                " Notice "
            })
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));

        let text = Paragraph::new(message.text.clone())
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color));

        frame.render_widget(text, popup_area);
    }

    /// Renders the help overlay
    pub fn render_help_overlay(frame: &mut Frame) {
        let area = frame.area();
        
        // Create a centered area for the help content (80% width, 90% height)
        let help_area = Rect::new(
            area.width / 10,
            area.height / 20,
            area.width * 8 / 10,
            area.height * 9 / 10,
        );
        
        // Create the help content
        let help_text = vec![
            Line::from(vec![
                Span::styled("RepoViewer Help", Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigation", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  ↑/↓      Navigate through files and directories"),
            Line::from("  ←        Go back to parent directory"),
            Line::from("  →/Enter  Open selected directory"),
            Line::from("  PgUp     Jump to first item"),
            Line::from("  PgDn     Jump to last item"),
            Line::from(""),
            Line::from(vec![
                Span::styled("File Collection", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  a        Add current file to collection"),
            Line::from("  A        Add all files in current directory"),
            Line::from("  d        Remove current file from collection"),
            Line::from("  D        Clear entire collection"),
            Line::from("  r        Refresh collected files (sync with changes)"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Export Options", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  S        Save collection to markdown file"),
            Line::from("  C        Copy collection to clipboard"),
            Line::from("  t        Save directory tree to file"),
            Line::from("  c        Copy directory tree to clipboard"),
            Line::from(""),
            Line::from(vec![
                Span::styled("View Options", Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  h        Toggle hidden files visibility"),
            Line::from("  g        Toggle gitignored files (in git repos)"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Tips", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            ]),
            Line::from("  • Files marked with [+] are in your collection"),
            Line::from("  • Collection size is shown in the header with health indicators"),
            Line::from("  • Yellow warning at 25MB, red warning at 50MB"),
            Line::from("  • Refresh (r) updates modified files and removes deleted ones"),
            Line::from("  • The tree export includes directory structure for AI context"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press '?' or ESC to close this help", 
                    Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC))
            ]),
        ];
        
        // Create the help block
        let help_block = Block::default()
            .title("  Help - RepoViewer  ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));
        
        // Create the paragraph
        let help_paragraph = Paragraph::new(help_text)
            .block(help_block)
            .wrap(Wrap { trim: false })
            .scroll((0, 0));
        
        // Clear the help area background first
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Black)),
            help_area
        );
        
        // Render the help content
        frame.render_widget(help_paragraph, help_area);
    }
}