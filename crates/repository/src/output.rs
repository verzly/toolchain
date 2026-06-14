//! User-visible output helpers for repository inspection commands.

use crate::project::ProjectProfile;
use comfy_table::{presets::NOTHING, Attribute, Cell, Color, Table};
use std::io::IsTerminal;

const ELLIPSIS: &str = "...";

pub fn compact_table() -> Table {
    let mut table = Table::new();
    table.load_preset(NOTHING);
    table.set_truncation_indicator(ELLIPSIS);
    table.style_text_only();
    table
}

pub fn style_compact_columns(table: &mut Table) {
    for column in table.column_iter_mut() {
        column.set_padding((0, 2));
    }
}

pub fn section(title: &str) {
    println!("{}", paint(Tone::Title, title));
    println!("{}", paint(Tone::Muted, &"-".repeat(title.len())));
}

pub fn field(label: &str, value: impl AsRef<str>) {
    println!(
        "{} {}",
        paint(Tone::Muted, &format!("{label}:")),
        paint(Tone::Value, value.as_ref())
    );
}

pub fn note(message: impl AsRef<str>) {
    println!("{}", paint(Tone::Accent, message.as_ref()));
}

pub fn warning(message: impl AsRef<str>) {
    println!("{}", paint(Tone::Warning, message.as_ref()));
}

pub fn command(message: impl AsRef<str>) -> String {
    paint(Tone::Command, message.as_ref())
}

pub fn header_cell(value: &str) -> Cell {
    Cell::new(compact_text(value, 24))
        .fg(Color::Cyan)
        .add_attribute(Attribute::Bold)
}

pub fn key_cell(value: &str) -> Cell {
    Cell::new(compact_text(value, 16)).fg(Color::DarkGrey)
}

pub fn plain_cell(value: impl AsRef<str>, max_width: usize) -> Cell {
    Cell::new(compact_text(value.as_ref(), max_width))
}

pub fn value_cell(value: impl AsRef<str>, max_width: usize) -> Cell {
    Cell::new(compact_text(value.as_ref(), max_width)).fg(Color::Green)
}

pub fn command_cell(value: impl AsRef<str>, max_width: usize) -> Cell {
    Cell::new(compact_text(value.as_ref(), max_width)).fg(Color::Green)
}

pub fn warning_cell(value: impl AsRef<str>, max_width: usize) -> Cell {
    Cell::new(compact_text(value.as_ref(), max_width)).fg(Color::Yellow)
}

pub fn print_release_target_table(profile: &ProjectProfile) {
    if profile.stored_config.release.targets.is_empty() {
        println!("{} -", paint(Tone::Muted, "Release targets:"));
        return;
    }

    println!("{}", paint(Tone::Accent, "Release targets:"));
    let mut table = compact_table();
    table.set_header([
        header_cell("Target"),
        header_cell("Source"),
        header_cell("Repo"),
        header_cell("Bin"),
        header_cell("WF"),
        header_cell("Dist"),
    ]);
    style_compact_columns(&mut table);
    for target in &profile.stored_config.release.targets {
        table.add_row([
            value_cell(&target.name, 20),
            plain_cell(empty_as_dash(&target.path), 22),
            plain_cell(empty_as_dash(&target.repository), 24),
            plain_cell(empty_as_dash(&target.cargo_binary), 18),
            workflow_cell(&target.workflow),
            plain_cell(empty_as_dash(&target.distribution_path), 34),
        ]);
    }
    println!("{table}");
}

pub fn release_workflow_summary(profile: &ProjectProfile) -> String {
    if profile.stored_config.release.targets.is_empty() {
        return "-".into();
    }

    let mut managed = 0;
    let mut preserve = 0;
    let mut custom = 0;

    for target in &profile.stored_config.release.targets {
        match target.workflow.as_str() {
            "managed" => managed += 1,
            "preserve" => preserve += 1,
            _ => custom += 1,
        }
    }

    format!("managed {managed}, preserve {preserve}, custom {custom}")
}

pub fn language_summary(profile: &ProjectProfile) -> String {
    let languages = profile
        .languages
        .iter()
        .map(|language| language.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if languages.is_empty() {
        "-".into()
    } else {
        languages
    }
}

pub fn js_runner_summary(profile: &ProjectProfile) -> &str {
    profile
        .js_runner
        .as_ref()
        .map(|runner| runner.as_str())
        .unwrap_or("-")
}

pub fn empty_as_dash(value: &str) -> &str {
    if value.trim().is_empty() {
        "-"
    } else {
        value
    }
}

fn workflow_cell(value: &str) -> Cell {
    match value {
        "managed" => value_cell(value, 9),
        "preserve" => warning_cell(value, 9),
        "" => plain_cell("-", 9),
        _ => Cell::new(compact_text(value, 9)).fg(Color::Magenta),
    }
}

fn compact_text(value: &str, max_width: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = if normalized.is_empty() {
        "-"
    } else {
        normalized.as_str()
    };

    if max_width == 0 {
        return String::new();
    }

    let length = normalized.chars().count();
    if length <= max_width {
        return normalized.to_string();
    }

    if max_width <= ELLIPSIS.len() {
        return ".".repeat(max_width);
    }

    let keep = max_width - ELLIPSIS.len();
    let mut compacted = normalized.chars().take(keep).collect::<String>();
    compacted.push_str(ELLIPSIS);
    compacted
}

enum Tone {
    Title,
    Accent,
    Value,
    Command,
    Warning,
    Muted,
}

fn paint(tone: Tone, value: &str) -> String {
    if !std::io::stdout().is_terminal() {
        return value.to_string();
    }

    let code = match tone {
        Tone::Title => "\x1b[1;36m",
        Tone::Accent => "\x1b[36m",
        Tone::Value => "\x1b[32m",
        Tone::Command => "\x1b[1;32m",
        Tone::Warning => "\x1b[33m",
        Tone::Muted => "\x1b[90m",
    };
    format!("{code}{value}\x1b[0m")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_text_keeps_table_cells_single_line() {
        assert_eq!(compact_text("alpha\nbeta\tgamma", 20), "alpha beta gamma");
    }

    #[test]
    fn compact_text_truncates_with_three_dot_ellipsis() {
        assert_eq!(compact_text("abcdefghijklmnopqrstuvwxyz", 10), "abcdefg...");
    }
}
