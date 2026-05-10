use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap},
};

use crate::{
    app::{App, FocusPane, display_category_name},
    models::{BlackArchTool, ToolStatus},
};

const BG: Color = Color::Rgb(24, 24, 37);
const SURFACE: Color = Color::Rgb(49, 50, 68);
const OVERLAY: Color = Color::Rgb(108, 112, 134);
const TEXT: Color = Color::Rgb(205, 214, 244);
const MUTED: Color = Color::Rgb(127, 132, 156);
const MAUVE: Color = Color::Rgb(203, 166, 247);
const CYAN: Color = Color::Rgb(137, 220, 235);
const GREEN: Color = Color::Rgb(166, 227, 161);
const AMBER: Color = Color::Rgb(249, 226, 175);
const WHITE: Color = Color::Rgb(245, 245, 255);

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let page = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(area);

    render_header(frame, page[0], app);
    render_body(frame, page[1], app);
    render_status_bar(frame, page[2], app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("󰣇 ", Style::default().fg(MAUVE)),
            Span::styled(
                "BlackArch Hypr TUI",
                Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "BlackArch Repository Manager",
            Style::default().fg(MUTED),
        )),
    ])
    .style(Style::default().bg(BG))
    .block(base_block().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM));

    let repo_status = if app.error_message.is_some() {
        Span::styled("error", Style::default().fg(AMBER))
    } else if app.loading {
        Span::styled("loading", Style::default().fg(AMBER))
    } else {
        Span::styled("synced", Style::default().fg(GREEN))
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled("Repo: ", Style::default().fg(MUTED)),
        repo_status,
        Span::raw("  "),
        Span::styled("Terminal: ", Style::default().fg(MUTED)),
        Span::styled("Kitty", Style::default().fg(CYAN)),
        Span::raw("  "),
        Span::styled("Mode: ", Style::default().fg(MUTED)),
        Span::styled("Hyprland", Style::default().fg(MAUVE)),
    ]))
    .alignment(Alignment::Right)
    .style(Style::default().bg(BG))
    .block(base_block().borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM));

    frame.render_widget(title, chunks[0]);
    frame.render_widget(status, chunks[1]);
}

fn render_body(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(54),
            Constraint::Length(38),
        ])
        .split(area);

    render_categories(frame, chunks[0], app);
    render_tools(frame, chunks[1], app);
    render_details(frame, chunks[2], app);

    if app.loading {
        render_center_message(frame, area, "Loading BlackArch tools...", None);
    } else if let Some(error) = &app.error_message {
        render_center_message(frame, area, "Backend Error", Some(error));
    }
}

fn render_categories(frame: &mut Frame, area: Rect, app: &App) {
    let list_items = app.categories.iter().enumerate().map(|(index, category)| {
        let label = display_category_name(category);
        if index == app.selected_category_index {
            ListItem::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(WHITE).bg(MAUVE)),
                Span::styled(
                    label,
                    Style::default()
                        .fg(WHITE)
                        .bg(MAUVE)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .style(Style::default().bg(MAUVE))
        } else {
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default().fg(MUTED)),
                Span::styled(label, Style::default().fg(TEXT)),
            ]))
        }
    });

    let title = if app.focus == FocusPane::Categories {
        "Categories *"
    } else {
        "Categories"
    };
    let list = List::new(list_items)
        .style(Style::default().bg(BG))
        .block(titled_block(title));

    frame.render_widget(list, area);
}

fn render_tools(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    let cursor = if app.focus == FocusPane::Search {
        "_"
    } else {
        ""
    };
    let title = if matches!(app.focus, FocusPane::Tools | FocusPane::Search) {
        "Tools *"
    } else {
        "Tools"
    };
    let search = Paragraph::new(Line::from(vec![
        Span::styled("Search: ", Style::default().fg(MUTED)),
        Span::styled(
            app.search_query.as_str(),
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(cursor, Style::default().fg(MAUVE)),
    ]))
    .style(Style::default().bg(BG))
    .block(titled_block(title));

    let header = Row::new(["", "Name", "Category", "Version", "Status"])
        .style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    if app.filtered_tools.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No tools match the current filter",
            Style::default().fg(MUTED),
        )))
        .alignment(Alignment::Center)
        .style(Style::default().bg(BG))
        .block(base_block().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
        frame.render_widget(search, chunks[0]);
        frame.render_widget(empty, chunks[1]);
        return;
    }

    let rows = app
        .filtered_tools
        .iter()
        .enumerate()
        .map(|(index, tool)| tool_row(tool, index == app.selected_tool_index));

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Min(13),
        ],
    )
    .header(header)
    .style(Style::default().bg(BG))
    .block(base_block().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));

    frame.render_widget(search, chunks[0]);
    frame.render_widget(table, chunks[1]);
}

fn tool_row(tool: &BlackArchTool, selected: bool) -> Row<'static> {
    let base = if selected {
        Style::default().fg(WHITE).bg(MAUVE)
    } else {
        Style::default().fg(TEXT).bg(BG)
    };
    let status = status_label(&tool.status).to_string();

    Row::new([
        Cell::from(if selected { ">" } else { " " }).style(base),
        Cell::from(tool.name.clone()).style(if selected {
            base.add_modifier(Modifier::BOLD)
        } else {
            base
        }),
        Cell::from(display_tool_category(tool)).style(base),
        Cell::from(tool.version.clone().unwrap_or_else(|| "-".to_string())).style(base),
        Cell::from(status).style(if selected {
            base.add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(status_color(&tool.status)).bg(BG)
        }),
    ])
    .height(1)
}

fn render_details(frame: &mut Frame, area: Rect, app: &App) {
    let title = if app.focus == FocusPane::Details {
        "Details *"
    } else {
        "Details"
    };

    let text = match app.selected_tool() {
        Some(tool) => details_lines(tool),
        None => vec![Line::from(Span::styled(
            "No tool selected",
            Style::default().fg(MUTED),
        ))],
    };

    let details = Paragraph::new(text)
        .style(Style::default().bg(BG))
        .wrap(Wrap { trim: true })
        .block(titled_block(title));

    frame.render_widget(details, area);
}

fn details_lines(tool: &BlackArchTool) -> Vec<Line<'static>> {
    let executable = tool.executable.as_deref().unwrap_or("-");
    let version = tool.version.as_deref().unwrap_or("-");
    let description = tool.description.as_deref().unwrap_or_else(|| {
        if tool.version.is_none() {
            "Loading package details..."
        } else {
            "No description available."
        }
    });
    let executables = if tool.executables.is_empty() {
        "-".to_string()
    } else {
        tool.executables.join(", ")
    };

    vec![
        Line::from(Span::styled(
            tool.name.clone(),
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        detail_line("Name", &tool.name, TEXT),
        detail_line("Package", &tool.package_name, CYAN),
        detail_line("Category", &display_tool_category(tool), TEXT),
        detail_line("Version", version, TEXT),
        detail_line("Executable", executable, TEXT),
        detail_line(
            "Status",
            status_label(&tool.status),
            status_color(&tool.status),
        ),
        Line::raw(""),
        Line::from(Span::styled(
            "Description",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            description.to_string(),
            Style::default().fg(TEXT),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            "Executables",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(executables, Style::default().fg(TEXT))),
        Line::raw(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        action_line("R", "Run"),
        action_line("I", "Install / Update"),
        action_line("X", "Remove"),
        action_line("F", "Favorite"),
        action_line("C", "Copy Command"),
        action_line("Enter", "Action Menu"),
    ]
}

fn detail_line(label: &str, value: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<11}"), Style::default().fg(MUTED)),
        Span::styled(value.to_string(), Style::default().fg(color)),
    ])
}

fn display_tool_category(tool: &BlackArchTool) -> String {
    tool.category
        .as_deref()
        .or_else(|| tool.categories.first().map(String::as_str))
        .map(display_category_name)
        .unwrap_or_else(|| "-".to_string())
}

fn action_line(key: &str, label: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("[{key}]"),
            Style::default()
                .fg(BG)
                .bg(CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(label.to_string(), Style::default().fg(TEXT)),
    ])
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(70), Constraint::Length(43)])
        .split(area);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            "↑↓",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" move  ", Style::default().fg(TEXT)),
        Span::styled(
            "Tab",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" switch pane  ", Style::default().fg(TEXT)),
        Span::styled("/", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" search  ", Style::default().fg(TEXT)),
        Span::styled(
            "Enter",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" actions  ", Style::default().fg(TEXT)),
        Span::styled("s", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" sync  ", Style::default().fg(TEXT)),
        Span::styled("f", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" favorite  ", Style::default().fg(TEXT)),
        Span::styled("q", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" quit", Style::default().fg(TEXT)),
    ]))
    .style(Style::default().bg(SURFACE));

    let message = app.error_message.as_deref().unwrap_or(&app.status_message);
    let activity = Paragraph::new(Line::from(Span::styled(
        truncate(message, 41),
        if app.error_message.is_some() {
            Style::default().fg(AMBER)
        } else {
            Style::default().fg(GREEN)
        },
    )))
    .alignment(Alignment::Right)
    .style(Style::default().bg(SURFACE));

    frame.render_widget(help, chunks[0]);
    frame.render_widget(activity, chunks[1]);
}

fn render_center_message(frame: &mut Frame, area: Rect, title: &str, message: Option<&str>) {
    let popup = centered_rect(64, 52, area);
    let mut lines = vec![
        Line::from(Span::styled(
            title.to_string(),
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];

    if let Some(message) = message {
        lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(TEXT),
        )));
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "Possible fixes:",
            Style::default().fg(CYAN),
        )));
        lines.push(Line::from(Span::styled(
            "- Make sure the BlackArch repository is installed",
            Style::default().fg(MUTED),
        )));
        lines.push(Line::from(Span::styled(
            "- Refresh sync databases after enabling the repository",
            Style::default().fg(MUTED),
        )));
        lines.push(Line::from(Span::styled(
            "- Press s to retry sync",
            Style::default().fg(MUTED),
        )));
        lines.push(Line::from(Span::styled(
            "- Press q to quit",
            Style::default().fg(MUTED),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(BG))
        .block(base_block());

    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn status_label(status: &ToolStatus) -> &'static str {
    match status {
        ToolStatus::Installed => "installed",
        ToolStatus::NotInstalled => "not installed",
        ToolStatus::UpdateAvailable => "update",
    }
}

fn status_color(status: &ToolStatus) -> Color {
    match status {
        ToolStatus::Installed => GREEN,
        ToolStatus::UpdateAvailable => AMBER,
        ToolStatus::NotInstalled => MUTED,
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut output = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        output.push('…');
    }
    output
}

fn titled_block(title: &'static str) -> Block<'static> {
    base_block().title(Span::styled(
        format!(" {title} "),
        Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
    ))
}

fn base_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(OVERLAY))
        .style(Style::default().bg(BG))
}
