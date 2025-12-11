use colored::Colorize;

/// App branding and styling - clean, minimalist design
pub struct Styles;

impl Styles {
    /// Print the app banner
    pub fn print_banner() {
        println!();
        println!("{}", "  ios-sim".bright_white().bold());
        println!("{}", "  iOS Development from Linux".dimmed());
        println!();
    }

    /// Print a section header
    pub fn header(text: &str) {
        println!();
        println!("{}", text.bright_white().bold());
        println!("{}", "─".repeat(40).dimmed());
    }

    /// Print a success message
    pub fn success(text: &str) {
        println!("{} {}", "[ok]".bright_green(), text);
    }

    /// Print an error message
    pub fn error(text: &str) {
        println!("{} {}", "[error]".bright_red(), text);
    }

    /// Print a warning message
    pub fn warning(text: &str) {
        println!("{} {}", "[warn]".bright_yellow(), text);
    }

    /// Print an info message
    pub fn info(text: &str) {
        println!("{} {}", "->".dimmed(), text);
    }

    /// Print a dimmed/secondary message
    pub fn dimmed(text: &str) {
        println!("   {}", text.dimmed());
    }

    /// Print a key-value pair
    pub fn kv(key: &str, value: &str) {
        println!(
            "   {:<16} {}",
            format!("{}:", key).dimmed(),
            value.bright_white()
        );
    }

    /// Print a status line with label
    pub fn status(icon: &str, label: &str, value: &str, color: StatusColor) {
        let colored_value = match color {
            StatusColor::Green => value.bright_green(),
            StatusColor::Yellow => value.bright_yellow(),
            StatusColor::Red => value.bright_red(),
            StatusColor::Blue => value.bright_blue(),
            StatusColor::White => value.bright_white(),
        };
        println!(
            "   {} {:<12} {}",
            icon.dimmed(),
            format!("{}:", label).dimmed(),
            colored_value
        );
    }

    /// Format bytes to human readable
    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Format duration to human readable
    pub fn format_duration(secs: f64) -> String {
        if secs < 1.0 {
            format!("{:.0}ms", secs * 1000.0)
        } else if secs < 60.0 {
            format!("{:.1}s", secs)
        } else {
            let mins = (secs / 60.0).floor();
            let remaining_secs = secs % 60.0;
            format!("{}m {:.0}s", mins, remaining_secs)
        }
    }
}

pub enum StatusColor {
    Green,
    Yellow,
    Red,
    Blue,
    White,
}
