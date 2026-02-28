use crate::app::App;
use crate::host::HostStatus;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),   // host table
            Constraint::Length(3), // detail
            Constraint::Length(2), // footer/help
        ])
        .split(area);

    render_header(f, app, chunks[0]);
    render_host_table(f, app, chunks[1]);
    render_detail(f, app, chunks[2]);
    render_footer(f, app, chunks[3]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let hosts = app.hosts.lock().unwrap();
    let total = hosts.len();
    let up = hosts
        .iter()
        .filter(|h| matches!(h.status, HostStatus::Up(_)))
        .count();
    let down = hosts
        .iter()
        .filter(|h| matches!(h.status, HostStatus::Down))
        .count();
    drop(hosts);

    let mut spans = vec![
        Span::styled(" sshmap ", Style::default().fg(Color::Cyan).bold()),
        Span::raw("│ "),
        Span::styled(format!("{} hosts", total), Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(format!("▲{}", up), Style::default().fg(Color::Green)),
        Span::raw(" "),
        Span::styled(format!("▼{}", down), Style::default().fg(Color::Red)),
    ];

    if app.filter_mode || !app.filter.is_empty() {
        spans.push(Span::raw("  │ "));
        spans.push(Span::styled("filter: ", Style::default().fg(Color::Yellow)));
        spans.push(Span::styled(
            &app.filter,
            Style::default().fg(Color::White).bold(),
        ));
        if app.filter_mode {
            spans.push(Span::styled("▌", Style::default().fg(Color::Yellow)));
        }
    }

    if let Some(ref msg) = app.message {
        spans.push(Span::raw("  │ "));
        spans.push(Span::styled(msg.as_str(), Style::default().fg(Color::Yellow)));
    }

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(header, area);
}

fn render_host_table(f: &mut Frame, app: &mut App, area: Rect) {
    let inner_height = area.height.saturating_sub(2) as usize;
    let filtered = app.filtered_indices();
    let total = filtered.len();

    // Adjust scroll
    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    }
    if app.selected >= app.scroll_offset + inner_height {
        app.scroll_offset = app.selected - inner_height + 1;
    }

    let hosts = app.hosts.lock().unwrap();

    let header = Row::new(vec![
        Cell::from(" ").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Alias").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Host").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("User").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Port").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Group").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("Status").style(Style::default().fg(Color::Cyan).bold()),
        Cell::from("RTT").style(Style::default().fg(Color::Cyan).bold()),
    ])
    .height(1);

    let mut last_group = String::new();
    let mut rows: Vec<Row> = Vec::new();

    for (display_idx, &real_idx) in filtered
        .iter()
        .enumerate()
        .skip(app.scroll_offset)
        .take(inner_height)
    {
        let host = &hosts[real_idx];
        let is_selected = display_idx == app.selected;

        // Group separator
        if app.show_groups && host.group != last_group {
            if !last_group.is_empty() {
                rows.push(Row::new(vec![Cell::from("")]));
            }
            last_group = host.group.clone();
        }

        let status_icon = match &host.status {
            HostStatus::Unknown => Span::styled("?", Style::default().fg(Color::DarkGray)),
            HostStatus::Checking => Span::styled("◌", Style::default().fg(Color::Yellow)),
            HostStatus::Up(_) => Span::styled("●", Style::default().fg(Color::Green)),
            HostStatus::Down => Span::styled("●", Style::default().fg(Color::Red)),
        };

        let (status_text, status_style) = match &host.status {
            HostStatus::Unknown => ("—", Style::default().fg(Color::DarkGray)),
            HostStatus::Checking => ("...", Style::default().fg(Color::Yellow)),
            HostStatus::Up(_) => ("UP", Style::default().fg(Color::Green)),
            HostStatus::Down => ("DOWN", Style::default().fg(Color::Red)),
        };

        let rtt = host.rtt_label();

        let group_color = group_color(&host.group);

        let row_style = if is_selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        let port_str = if host.port != 22 {
            format!("{}", host.port)
        } else {
            "22".to_string()
        };

        rows.push(
            Row::new(vec![
                Cell::from(status_icon),
                Cell::from(host.alias.clone()).style(Style::default().fg(Color::White).bold()),
                Cell::from(host.hostname.clone()).style(Style::default().fg(Color::DarkGray)),
                Cell::from(host.user.clone()).style(Style::default().fg(Color::Cyan)),
                Cell::from(port_str),
                Cell::from(host.group.clone()).style(Style::default().fg(group_color)),
                Cell::from(status_text).style(status_style),
                Cell::from(rtt).style(Style::default().fg(Color::DarkGray)),
            ])
            .style(row_style),
        );
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),  // status icon
            Constraint::Length(18), // alias
            Constraint::Length(20), // hostname
            Constraint::Length(12), // user
            Constraint::Length(6),  // port
            Constraint::Length(14), // group
            Constraint::Length(6),  // status
            Constraint::Length(8),  // rtt
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(" {} hosts ", total))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(table, area);
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_indices();
    let hosts = app.hosts.lock().unwrap();

    let content = if let Some(&real_idx) = filtered.get(app.selected) {
        let host = &hosts[real_idx];
        let cmd = host.ssh_command().join(" ");
        Line::from(vec![
            Span::raw(" → "),
            Span::styled(cmd, Style::default().fg(Color::Green).bold()),
            if let Some(ref key) = host.identity_file {
                Span::styled(
                    format!("  │  key: {}", key),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::raw("")
            },
        ])
    } else {
        Line::from(Span::styled(
            " No host selected",
            Style::default().fg(Color::DarkGray),
        ))
    };

    let detail = Paragraph::new(content).block(
        Block::default()
            .title(" Command ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(detail, area);
}

fn render_footer(f: &mut Frame, _app: &App, area: Rect) {
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑↓", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":Nav  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":Connect  "),
        Span::styled("/", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":Filter  "),
        Span::styled("p", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":Ping  "),
        Span::styled("P", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":PingAll  "),
        Span::styled("g", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":Groups  "),
        Span::styled("q", Style::default().fg(Color::Yellow).bold()),
        Span::raw(":Quit"),
    ]));
    f.render_widget(help, area);
}

fn group_color(group: &str) -> Color {
    match group.to_lowercase().as_str() {
        "production" | "prod" => Color::Red,
        "staging" | "stage" => Color::Yellow,
        "dev" | "development" => Color::Green,
        "test" | "testing" => Color::Cyan,
        _ => Color::Magenta,
    }
}
