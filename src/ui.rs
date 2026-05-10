use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
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

pub fn render(frame: &mut Frame) {
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

    render_header(frame, page[0]);
    render_body(frame, page[1]);
    render_status_bar(frame, page[2]);
}

fn render_header(frame: &mut Frame, area: Rect) {
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

    let status = Paragraph::new(Line::from(vec![
        Span::styled("Repo: ", Style::default().fg(MUTED)),
        Span::styled("synced", Style::default().fg(GREEN)),
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

fn render_body(frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(24),
            Constraint::Min(54),
            Constraint::Length(38),
        ])
        .split(area);

    render_categories(frame, chunks[0]);
    render_tools(frame, chunks[1]);
    render_details(frame, chunks[2]);
}

fn render_categories(frame: &mut Frame, area: Rect) {
    let items = [
        "All Tools",
        "Scanner",
        "Webapp",
        "Recon",
        "Exploitation",
        "Wireless",
        "Forensic",
        "Reversing",
        "Favorites",
        "Installed",
        "Recent",
    ];

    let list_items = items.into_iter().map(|item| {
        if item == "Webapp" {
            ListItem::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(WHITE).bg(MAUVE)),
                Span::styled(
                    item,
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
                Span::styled(item, Style::default().fg(TEXT)),
            ]))
        }
    });

    let list = List::new(list_items)
        .style(Style::default().bg(BG))
        .block(titled_block("Categories"));

    frame.render_widget(list, area);
}

fn render_tools(frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    let search = Paragraph::new(Line::from(vec![
        Span::styled("Search: ", Style::default().fg(MUTED)),
        Span::styled(
            "sql",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled("_", Style::default().fg(MAUVE)),
    ]))
    .style(Style::default().bg(BG))
    .block(titled_block("Tools"));

    let rows = [
        tool_row(" ", "nmap", "Scanner", "7.94", "installed", GREEN, false),
        tool_row(">", "sqlmap", "Webapp", "1.7.11", "update", AMBER, true),
        tool_row(" ", "amass", "Recon", "4.2.0", "installed", GREEN, false),
        tool_row(
            " ",
            "aircrack-ng",
            "Wireless",
            "1.7",
            "installed",
            GREEN,
            false,
        ),
        tool_row(
            " ",
            "binwalk",
            "Forensic",
            "2.3.4",
            "not installed",
            MUTED,
            false,
        ),
        tool_row(
            " ",
            "nikto",
            "Webapp",
            "2.5.0",
            "not installed",
            MUTED,
            false,
        ),
        tool_row(
            " ",
            "gobuster",
            "Webapp",
            "3.6.0",
            "installed",
            GREEN,
            false,
        ),
    ];

    let header = Row::new(["", "Name", "Category", "Version", "Status"])
        .style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
        .bottom_margin(1);

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

fn tool_row<'a>(
    indicator: &'a str,
    name: &'a str,
    category: &'a str,
    version: &'a str,
    status: &'a str,
    status_color: Color,
    selected: bool,
) -> Row<'a> {
    let base = if selected {
        Style::default().fg(WHITE).bg(MAUVE)
    } else {
        Style::default().fg(TEXT).bg(BG)
    };

    Row::new([
        Cell::from(indicator).style(base),
        Cell::from(name).style(if selected {
            base.add_modifier(Modifier::BOLD)
        } else {
            base
        }),
        Cell::from(category).style(base),
        Cell::from(version).style(base),
        Cell::from(status).style(if selected {
            base.add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(status_color).bg(BG)
        }),
    ])
    .height(1)
}

fn render_details(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(Span::styled(
            "sqlmap",
            Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        detail_line("Name", "sqlmap", TEXT),
        detail_line("Package", "blackarch/sqlmap", CYAN),
        detail_line("Category", "Webapp", TEXT),
        detail_line("Version", "1.7.11", TEXT),
        detail_line("Executable", "sqlmap", TEXT),
        detail_line("Status", "update available", AMBER),
        Line::raw(""),
        Line::from(Span::styled(
            "Description",
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "Automatic SQL injection and",
            Style::default().fg(TEXT),
        )),
        Line::from(Span::styled(
            "database takeover tool.",
            Style::default().fg(TEXT),
        )),
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
    ];

    let details = Paragraph::new(text)
        .style(Style::default().bg(BG))
        .block(titled_block("Details"));

    frame.render_widget(details, area);
}

fn detail_line<'a>(label: &'a str, value: &'a str, color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<11}"), Style::default().fg(MUTED)),
        Span::styled(value, Style::default().fg(color)),
    ])
}

fn action_line<'a>(key: &'a str, label: &'a str) -> Line<'a> {
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
        Span::styled(label, Style::default().fg(TEXT)),
    ])
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
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
        Span::styled("r", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" run  ", Style::default().fg(TEXT)),
        Span::styled("i", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" install  ", Style::default().fg(TEXT)),
        Span::styled("f", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" favorite  ", Style::default().fg(TEXT)),
        Span::styled("q", Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)),
        Span::styled(" quit", Style::default().fg(TEXT)),
    ]))
    .style(Style::default().bg(SURFACE));

    let activity = Paragraph::new(Line::from(vec![
        Span::styled("Ready", Style::default().fg(GREEN)),
        Span::styled(" • ", Style::default().fg(MUTED)),
        Span::styled("3 updates available", Style::default().fg(AMBER)),
        Span::styled(" • cache loaded", Style::default().fg(MUTED)),
    ]))
    .alignment(Alignment::Right)
    .style(Style::default().bg(SURFACE));

    frame.render_widget(help, chunks[0]);
    frame.render_widget(activity, chunks[1]);
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
