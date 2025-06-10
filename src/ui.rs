use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect}, 
    style::{Color, Modifier, Style}, 
    text::{Line, Span}, 
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap}, 
    Frame
};

use crate::{app::App, utils::get_file_display_info};

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
        // Create the main layout -- this divides our terminal into three sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header (fixed height)
                Constraint::Min(5),    // File list (takes remaining space)
                Constraint::Length(6), // Status bar (fixed height)
            ])
            .split(frame.area());

        // Now render each section
        // Notice how we pass the specific area (chunk) for each component
        Self::render_header(frame, app, chunks[0]);
        Self::render_file_list(frame, app, chunks[1]);
        Self::render_status_bar(frame, app, chunks[2]);

        // Render message popup if there is an active message
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
            let size_str = app.format_size(app.get_collection_size());
            lines.push(
                Line::from(format!(
                    "📦 Collection: {} files ({})",
                    app.collected_files.len(),
                    size_str
                ))
                .style(Style::default().fg(Color::Yellow)),
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
                let display_name = format!("{} {}", icon, item.name);

                let final_style = if is_collected {
                    style.bg(Color::Rgb(50, 50, 50)) // Dark gray background for collected
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
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        // For stateful widgets, we need to clone the state
        // This is because render_stateful_widget needs mutable access to the state
        // but we only have an immutable reference to app
        let mut list_state = app.state.clone();
        frame.render_stateful_widget(list, area, &mut list_state);
    }

    /// Renders the status bar with keyboard shortcuts and toggle states
    fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
        // Build the control information lines
        // Each line shows different categories of controls
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
            // Quit control
            Line::from(vec![
                Span::styled("Exit:", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled("q/Esc", Style::default().fg(Color::Red)),
                Span::raw(" Quit"),
            ]),
        ];

        // Update the layout constraints to accommodate the additional line
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Line 1
                Constraint::Length(1), // Line 2
                Constraint::Length(1), // Line 3
                Constraint::Length(1), // Line 4
                Constraint::Length(1), // Line 5
            ])
            .split(area);

        // Create the status block
        let status_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        frame.render_widget(status_block, area);

        // Render each line of controls in its own area within the block
        for (i, line) in controls.iter().enumerate() {
            if i < chunks.len() {
                let inner_area = Rect {
                    x: chunks[i].x + 1, // Add 1 to avoid drawing on the border
                    y: chunks[i].y,
                    width: chunks[i].width.saturating_sub(2), // Adjust width for both borders
                    height: chunks[i].height,
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
}