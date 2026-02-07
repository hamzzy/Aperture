//! Output formatting utilities for CLI commands

use colored::Colorize;

/// Print success message
pub fn success(msg: &str) {
    println!("{} {}", "✓".green(), msg);
}

/// Print error message
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red(), msg);
}

/// Print info message
pub fn info(msg: &str) {
    println!("{} {}", "ℹ".blue(), msg);
}

/// Print warning message
pub fn warning(msg: &str) {
    println!("{} {}", "⚠".yellow(), msg);
}
