use colored::Colorize;
use std::io::{self, Write};

/// Screen utilities for better UX
pub struct Screen;

impl Screen {
    /// Clear the terminal screen
    pub fn clear() {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
    }

    /// Print the header with optional breadcrumb path
    pub fn header(path: &[&str]) {
        println!();
        println!(
            "  {}  {}  {}",
            "xscape".bright_white().bold(),
            "—".dimmed(),
            "Be grateful you don't have to develop on Mac".dimmed()
        );

        if !path.is_empty() {
            let breadcrumb = path.join(" > ");
            println!("  {}", breadcrumb.dimmed());
        }

        println!("{}", "  ─".repeat(38).dimmed());
        println!();
    }

    /// Print a status bar showing connection info
    pub fn status_bar(connected: bool, xcode: Option<&str>, simulators: Option<usize>) {
        let conn_status = if connected {
            "connected".bright_green()
        } else {
            "disconnected".bright_red()
        };

        let mut parts = vec![format!("agent: {}", conn_status)];

        if let Some(xc) = xcode {
            parts.push(format!("xcode: {}", xc));
        }

        if let Some(count) = simulators {
            parts.push(format!("simulators: {}", count));
        }

        println!("  {}", parts.join("  |  ").dimmed());
        println!();
    }

    /// Show a "press enter to continue" prompt
    pub fn pause() {
        print!("\n  Press Enter to continue...");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
    }

    /// Print a divider line
    pub fn divider() {
        println!("{}", "  ─".repeat(38).dimmed());
    }

    /// Print section title
    pub fn section(title: &str) {
        println!();
        println!("  {}", title.bright_white());
        println!();
    }

    /// Print an info line with proper indentation
    pub fn info(text: &str) {
        println!("  {}", text);
    }

    /// Print a key-value pair
    pub fn kv(key: &str, value: &str) {
        println!("  {:<16} {}", format!("{}:", key).dimmed(), value.bright_white());
    }

    /// Print success message
    pub fn success(text: &str) {
        println!("  {} {}", "ok".bright_green(), text);
    }

    /// Print error message
    pub fn error(text: &str) {
        println!("  {} {}", "error".bright_red(), text);
    }

    /// Print warning message
    pub fn warning(text: &str) {
        println!("  {} {}", "warn".bright_yellow(), text);
    }
}
