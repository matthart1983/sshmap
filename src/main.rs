mod app;
mod health;
mod host;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io;
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<()> {
    // Create sample config if none exists
    host::create_sample_config()?;

    let hosts = host::load_hosts();
    if hosts.is_empty() {
        eprintln!("No hosts found. Add hosts to ~/.ssh/config or ~/.config/sshmap/hosts.json");
        std::process::exit(1);
    }

    eprintln!("Loaded {} hosts", hosts.len());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(hosts);

    // Initial health check
    health::check_all(Arc::clone(&app.hosts));

    loop {
        terminal.draw(|f| {
            ui::render(f, &mut app);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if app.filter_mode {
                    match key.code {
                        KeyCode::Esc => {
                            app.filter_mode = false;
                        }
                        KeyCode::Enter => {
                            app.filter_mode = false;
                        }
                        KeyCode::Backspace => {
                            app.filter.pop();
                            app.selected = 0;
                            app.scroll_offset = 0;
                        }
                        KeyCode::Char(c) => {
                            app.filter.push(c);
                            app.selected = 0;
                            app.scroll_offset = 0;
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Char('c')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.should_quit = true;
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.select_up(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_down(),
                    KeyCode::PageUp => app.page_up(10),
                    KeyCode::PageDown => app.page_down(10),
                    KeyCode::Enter => {
                        app.connect_selected();
                    }
                    KeyCode::Char('/') => {
                        app.filter_mode = true;
                        app.message = None;
                    }
                    KeyCode::Esc => {
                        app.filter.clear();
                        app.selected = 0;
                        app.scroll_offset = 0;
                    }
                    KeyCode::Char('p') => {
                        // Ping selected host
                        if let Some(idx) = app.selected_host_index() {
                            health::check_one(Arc::clone(&app.hosts), idx);
                        }
                    }
                    KeyCode::Char('P') => {
                        // Ping all
                        health::check_all(Arc::clone(&app.hosts));
                        app.message = Some("Pinging all hosts...".into());
                    }
                    KeyCode::Char('g') => {
                        app.show_groups = !app.show_groups;
                    }
                    _ => {}
                }
            }
        }

        // Handle connection
        if let Some(idx) = app.connect_index.take() {
            let cmd = {
                let hosts = app.hosts.lock().unwrap();
                hosts[idx].ssh_command()
            };

            // Restore terminal
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            // Launch SSH
            let status = std::process::Command::new(&cmd[0])
                .args(&cmd[1..])
                .status();

            match status {
                Ok(s) => {
                    if !s.success() {
                        eprintln!("SSH exited with: {}", s);
                    }
                }
                Err(e) => eprintln!("Failed to launch ssh: {}", e),
            }

            // Re-enter TUI
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
            terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
            app.message = Some("Returned from SSH session".into());
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
