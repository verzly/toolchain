use crate::cli::{
    CheckArgs, DoctorArgs, InitArgs, JsRunnerArg, PlanArgs, ReleaseArgs, ReleaseCommand, TuiArgs,
    UpdateArgs,
};
use crate::commands;
use crate::project::ProjectProfile;
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

const ACTIONS: &[TuiAction] = &[
    TuiAction {
        command: TuiCommand::Plan,
        slash: "/plan",
        title: "Plan",
        summary: "Show generated quality files and release graph.",
        plan_cli: "repository plan --root .",
        act_cli: "repository plan --root .",
        safety: "Read-only.",
    },
    TuiAction {
        command: TuiCommand::Check,
        slash: "/check",
        title: "Check",
        summary: "Validate config, distributions, README/action docs, and workflows.",
        plan_cli: "repository check --root .",
        act_cli: "repository check --root .",
        safety: "Read-only.",
    },
    TuiAction {
        command: TuiCommand::Doctor,
        slash: "/doctor",
        title: "Doctor",
        summary: "Inspect local quality tooling readiness.",
        plan_cli: "repository doctor --root .",
        act_cli: "repository doctor --root .",
        safety: "Read-only diagnostics.",
    },
    TuiAction {
        command: TuiCommand::Update,
        slash: "/update",
        title: "Update",
        summary: "Refresh repository-managed quality files.",
        plan_cli: "repository update --dry-run --skip-mise-use --skip-hk-install",
        act_cli: "repository update",
        safety: "PLAN runs a dry-run. ACT asks before overwriting files or running installers.",
    },
    TuiAction {
        command: TuiCommand::Init,
        slash: "/init",
        title: "Init",
        summary: "Bootstrap repository quality files.",
        plan_cli: "repository init --dry-run --skip-mise-use --skip-hk-install",
        act_cli: "repository init",
        safety: "PLAN runs a dry-run. ACT asks before writing files or installing hooks.",
    },
    TuiAction {
        command: TuiCommand::Release,
        slash: "/release",
        title: "Release",
        summary: "Open release target management.",
        plan_cli: "repository release",
        act_cli: "repository release",
        safety: "Uses the same release editor as the CLI.",
    },
];

pub fn run(args: TuiArgs) -> Result<()> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        run_fullscreen(args)
    } else {
        run_plain(args)
    }
}

fn run_fullscreen(args: TuiArgs) -> Result<()> {
    let mut terminal = FullscreenTerminal::enter()?;
    let mut app = TuiApp::default();

    loop {
        let profile = ProjectProfile::detect(
            args.root.clone(),
            args.config.clone(),
            None,
            &[],
            JsRunnerArg::Auto,
        )?;
        terminal.draw(|frame| render_fullscreen(frame, &profile, &app))?;

        let Some(command) = read_command(&mut app)? else {
            continue;
        };

        match command {
            TuiCommand::Quit => return Ok(()),
            TuiCommand::Mode(mode) => app.mode = mode,
            TuiCommand::Help => app.show_help = !app.show_help,
            TuiCommand::Noop => {}
            TuiCommand::Unknown(value) => app.status = format!("Unknown command: {value}"),
            command => {
                terminal.suspend()?;
                let result = execute_command(&args, app.mode, command);
                wait_for_enter()?;
                terminal.resume()?;
                app.status = match result {
                    Ok(()) => format!("Completed {}", command.label()),
                    Err(error) => format!("Error: {error:#}"),
                };
            }
        }
    }
}

fn run_plain(args: TuiArgs) -> Result<()> {
    let mut mode = TuiMode::Plan;

    loop {
        let profile = ProjectProfile::detect(
            args.root.clone(),
            args.config.clone(),
            None,
            &[],
            JsRunnerArg::Auto,
        )?;
        print!("{}", render_plain_dashboard(&profile, mode, false));

        match TuiCommand::parse(&prompt(mode.prompt())?) {
            TuiCommand::Mode(next_mode) => mode = next_mode,
            TuiCommand::Help => {
                print_plain_help(mode);
                pause()?;
            }
            TuiCommand::Quit => return Ok(()),
            TuiCommand::Unknown(value) => {
                println!("Unknown command `{value}`. Type /help for available commands.");
                pause()?;
            }
            TuiCommand::Noop => {}
            command => run_and_pause(|| execute_command(&args, mode, command))?,
        }
    }
}

struct FullscreenTerminal {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl FullscreenTerminal {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable terminal raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("failed to initialize terminal UI")?;
        Ok(Self { terminal })
    }

    fn draw(&mut self, render: impl FnOnce(&mut Frame<'_>)) -> Result<()> {
        self.terminal
            .draw(render)
            .context("failed to render terminal UI")?;
        Ok(())
    }

    fn suspend(&mut self) -> Result<()> {
        disable_raw_mode().context("failed to disable terminal raw mode")?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("failed to leave alternate screen")?;
        self.terminal.show_cursor().ok();
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        enable_raw_mode().context("failed to enable terminal raw mode")?;
        execute!(self.terminal.backend_mut(), EnterAlternateScreen)
            .context("failed to enter alternate screen")?;
        Ok(())
    }
}

impl Drop for FullscreenTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[derive(Debug)]
struct TuiApp {
    mode: TuiMode,
    selected: usize,
    command_input: Option<String>,
    show_help: bool,
    status: String,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            mode: TuiMode::Plan,
            selected: 0,
            command_input: None,
            show_help: false,
            status: "Use arrows, j/k, number keys, Enter, or slash commands.".into(),
        }
    }
}

fn read_command(app: &mut TuiApp) -> Result<Option<TuiCommand>> {
    let Event::Key(key) = event::read().context("failed to read terminal event")? else {
        return Ok(None);
    };
    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }

    if app.command_input.is_some() {
        return handle_command_input(app, key.code);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Ok(Some(TuiCommand::Quit)),
        KeyCode::Char('?') => Ok(Some(TuiCommand::Help)),
        KeyCode::Char('/') => {
            app.command_input = Some("/".into());
            Ok(None)
        }
        KeyCode::Char('p') => Ok(Some(TuiCommand::Mode(TuiMode::Plan))),
        KeyCode::Char('a') => Ok(Some(TuiCommand::Mode(TuiMode::Act))),
        KeyCode::Tab => Ok(Some(TuiCommand::Mode(app.mode.toggle()))),
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected = app.selected.saturating_sub(1);
            Ok(None)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.selected = (app.selected + 1).min(ACTIONS.len().saturating_sub(1));
            Ok(None)
        }
        KeyCode::Enter => Ok(Some(ACTIONS[app.selected].command)),
        KeyCode::Char(value) if value.is_ascii_digit() => {
            let Some(index) = value.to_digit(10).and_then(|value| value.checked_sub(1)) else {
                return Ok(None);
            };
            let index = index as usize;
            if index < ACTIONS.len() {
                app.selected = index;
                Ok(Some(ACTIONS[index].command))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn handle_command_input(app: &mut TuiApp, code: KeyCode) -> Result<Option<TuiCommand>> {
    match code {
        KeyCode::Enter => {
            let value = app.command_input.take().unwrap_or_default();
            Ok(Some(TuiCommand::parse(&value)))
        }
        KeyCode::Esc => {
            app.command_input = None;
            Ok(None)
        }
        KeyCode::Backspace => {
            if let Some(input) = &mut app.command_input {
                input.pop();
                if input.is_empty() {
                    app.command_input = None;
                }
            }
            Ok(None)
        }
        KeyCode::Char(value) => {
            if let Some(input) = &mut app.command_input {
                input.push(value);
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn execute_command(args: &TuiArgs, mode: TuiMode, command: TuiCommand) -> Result<()> {
    match command {
        TuiCommand::Plan => commands::plan::run(PlanArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            languages: Vec::new(),
            js_runner: JsRunnerArg::Auto,
            workspace: None,
        }),
        TuiCommand::Init => run_init(args, mode),
        TuiCommand::Update => run_update(args, mode),
        TuiCommand::Release => commands::release::run(ReleaseArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            command: Some(ReleaseCommand::Tui),
        }),
        TuiCommand::Doctor => commands::doctor::run(DoctorArgs {
            root: args.root.clone(),
            config: args.config.clone(),
        }),
        TuiCommand::Check => commands::check::run(CheckArgs {
            root: args.root.clone(),
            config: args.config.clone(),
        }),
        TuiCommand::Mode(_) | TuiCommand::Help | TuiCommand::Quit | TuiCommand::Noop => Ok(()),
        TuiCommand::Unknown(value) => {
            println!("Unknown command `{value}`.");
            Ok(())
        }
    }
}

fn run_init(args: &TuiArgs, mode: TuiMode) -> Result<()> {
    let workspace = prompt_optional_path("Workspace override")?;
    match mode {
        TuiMode::Plan => commands::init::run(InitArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            force: false,
            dry_run: true,
            skip_mise_use: true,
            skip_hk_install: true,
            languages: Vec::new(),
            js_runner: JsRunnerArg::Auto,
            workspace,
            skip_style_configs: false,
            skip_actions: false,
        }),
        TuiMode::Act => {
            let force = confirm("Overwrite existing managed files")?;
            let run_mise = confirm("Run mise use for missing tools")?;
            let install_hooks = confirm("Run hk install after writing files")?;
            commands::init::run(InitArgs {
                root: args.root.clone(),
                config: args.config.clone(),
                force,
                dry_run: false,
                skip_mise_use: !run_mise,
                skip_hk_install: !install_hooks,
                languages: Vec::new(),
                js_runner: JsRunnerArg::Auto,
                workspace,
                skip_style_configs: false,
                skip_actions: false,
            })
        }
    }
}

fn run_update(args: &TuiArgs, mode: TuiMode) -> Result<()> {
    match mode {
        TuiMode::Plan => commands::init::run_update(UpdateArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            force: false,
            dry_run: true,
            skip_mise_use: true,
            skip_hk_install: true,
            skip_style_configs: false,
            skip_actions: false,
        }),
        TuiMode::Act => {
            let force = confirm("Overwrite existing managed files")?;
            let run_mise = confirm("Run mise use for missing tools")?;
            let install_hooks = confirm("Run hk install after writing files")?;
            commands::init::run_update(UpdateArgs {
                root: args.root.clone(),
                config: args.config.clone(),
                force,
                dry_run: false,
                skip_mise_use: !run_mise,
                skip_hk_install: !install_hooks,
                skip_style_configs: false,
                skip_actions: false,
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TuiMode {
    Plan,
    Act,
}

impl TuiMode {
    fn prompt(self) -> &'static str {
        match self {
            Self::Plan => "repository plan>",
            Self::Act => "repository act>",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Plan => "PLAN",
            Self::Act => "ACT",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Plan => "preview-only commands; write operations run as dry-runs",
            Self::Act => "write-capable commands; every mutation still asks for confirmation",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Plan => Color::Blue,
            Self::Act => Color::Yellow,
        }
    }

    fn ansi_color(self) -> &'static str {
        match self {
            Self::Plan => "\x1b[1;34m",
            Self::Act => "\x1b[1;33m",
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::Plan => Self::Act,
            Self::Act => Self::Plan,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TuiCommand {
    Plan,
    Init,
    Update,
    Release,
    Doctor,
    Check,
    Mode(TuiMode),
    Help,
    Quit,
    Noop,
    Unknown(&'static str),
}

impl TuiCommand {
    fn parse(value: &str) -> Self {
        let normalized = value.trim().to_ascii_lowercase();
        let normalized = normalized.strip_prefix('/').unwrap_or(&normalized);

        match normalized {
            "" => Self::Noop,
            "1" | "plan" | "graph" => Self::Plan,
            "2" | "check" | "contracts" => Self::Check,
            "3" | "doctor" | "health" => Self::Doctor,
            "4" | "update" | "update preview" | "update dry-run" | "update-dry-run"
            | "update apply" | "refresh" => Self::Update,
            "5" | "init" | "init preview" | "init dry-run" | "init-dry-run" | "init apply"
            | "bootstrap" => Self::Init,
            "6" | "release" | "release targets" | "targets" => Self::Release,
            "mode plan" | "plan mode" | "safe" => Self::Mode(TuiMode::Plan),
            "mode act" | "act mode" | "apply" => Self::Mode(TuiMode::Act),
            "help" | "?" => Self::Help,
            "q" | "quit" | "exit" => Self::Quit,
            _ => Self::Unknown("unrecognized command"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Init => "init",
            Self::Update => "update",
            Self::Release => "release",
            Self::Doctor => "doctor",
            Self::Check => "check",
            Self::Mode(_) => "mode switch",
            Self::Help => "help",
            Self::Quit => "quit",
            Self::Noop => "noop",
            Self::Unknown(_) => "unknown command",
        }
    }
}

#[derive(Clone, Copy)]
struct TuiAction {
    command: TuiCommand,
    slash: &'static str,
    title: &'static str,
    summary: &'static str,
    plan_cli: &'static str,
    act_cli: &'static str,
    safety: &'static str,
}

fn render_fullscreen(frame: &mut Frame<'_>, profile: &ProjectProfile, app: &TuiApp) {
    let area = frame.area();
    let outer = Block::default()
        .title(Line::from(vec![
            Span::styled(" repository ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("standards command center"),
        ]))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    frame.render_widget(outer, area);

    let main = inner(area, 1);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(12),
            Constraint::Length(4),
        ])
        .split(main);
    render_header(frame, rows[0], profile, app);
    render_body(frame, rows[1], profile, app);
    render_footer(frame, rows[2], app);

    if app.show_help {
        render_help_overlay(frame, centered_rect(68, 70, area));
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, profile: &ProjectProfile, app: &TuiApp) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(area);

    render_metric(
        frame,
        columns[0],
        "Workspace",
        &profile.workspace_display(),
        Color::Cyan,
    );
    render_metric(
        frame,
        columns[1],
        "Mode",
        app.mode.label(),
        app.mode.color(),
    );
    render_metric(
        frame,
        columns[2],
        "Languages",
        &language_summary(profile),
        Color::Green,
    );
    render_metric(
        frame,
        columns[3],
        "Targets",
        &profile.stored_config.release.targets.len().to_string(),
        Color::Magenta,
    );
}

fn render_metric(frame: &mut Frame<'_>, area: Rect, label: &str, value: &str, color: Color) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let text = Text::from(vec![
        Line::from(Span::styled(
            label,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            value,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ]);
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn render_body(frame: &mut Frame<'_>, area: Rect, profile: &ProjectProfile, app: &TuiApp) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    render_actions(frame, columns[0], app);
    render_preview(frame, columns[1], profile, app);
}

fn render_actions(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let items = ACTIONS
        .iter()
        .enumerate()
        .map(|(index, action)| {
            let number = format!("{} ", index + 1);
            ListItem::new(Line::from(vec![
                Span::styled(number, Style::default().fg(Color::DarkGray)),
                Span::styled(action.slash, Style::default().fg(Color::Cyan).bold()),
                Span::raw("  "),
                Span::styled(action.title, Style::default().fg(Color::White).bold()),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(app.selected));
    let list = List::new(items)
        .block(
            Block::default()
                .title(" command palette ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_preview(frame: &mut Frame<'_>, area: Rect, profile: &ProjectProfile, app: &TuiApp) {
    let action = &ACTIONS[app.selected];
    let cli = match app.mode {
        TuiMode::Plan => action.plan_cli,
        TuiMode::Act => action.act_cli,
    };
    let text = Text::from(vec![
        Line::from(Span::styled(
            action.title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        Line::from(action.summary),
        Line::raw(""),
        Line::from(vec![
            Span::styled("CLI: ", Style::default().fg(Color::DarkGray)),
            Span::styled(cli, Style::default().fg(Color::Green)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Safety: ", Style::default().fg(Color::DarkGray)),
            Span::raw(action.safety),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Root: ", Style::default().fg(Color::DarkGray)),
            Span::raw(profile.root.display().to_string()),
        ]),
        Line::from(vec![
            Span::styled("Config: ", Style::default().fg(Color::DarkGray)),
            Span::raw(profile.config_display()),
        ]),
        Line::from(vec![
            Span::styled("Release workflows: ", Style::default().fg(Color::DarkGray)),
            Span::raw(release_workflow_summary(profile)),
        ]),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title(" selected action ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let input = app
        .command_input
        .as_deref()
        .map(|value| format!(" command: {value}"))
        .unwrap_or_else(|| {
            " enter run  arrows/j/k move  / command  tab mode  p plan  a act  ? help  q quit".into()
        });
    let text = Text::from(vec![
        Line::from(input),
        Line::from(vec![
            Span::styled(" status: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&app.status),
        ]),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_help_overlay(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(Clear, area);
    let text = Text::from(vec![
        Line::from(Span::styled(
            "Keyboard",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::raw(""),
        Line::raw("Enter       run selected action"),
        Line::raw("1-6         run action directly"),
        Line::raw("Up/Down     move selection"),
        Line::raw("j/k         move selection"),
        Line::raw("/           type a slash command"),
        Line::raw("Tab         toggle PLAN/ACT"),
        Line::raw("p / a       switch to PLAN or ACT"),
        Line::raw("?           close this help"),
        Line::raw("q / Esc     quit"),
    ]);
    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Left).block(
            Block::default()
                .title(" help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        area,
    );
}

fn render_plain_dashboard(profile: &ProjectProfile, mode: TuiMode, color: bool) -> String {
    let language_display = language_summary(profile);
    let js_runner = profile
        .js_runner
        .as_ref()
        .map(|runner| runner.as_str())
        .unwrap_or("-");
    let release_status = if profile.release_enabled() {
        "enabled"
    } else {
        "disabled"
    };

    let mut out = String::new();
    out.push('\n');
    out.push_str(&paint(color, "\x1b[1;36m", "repository"));
    out.push('\n');
    out.push_str("==========\n");
    out.push_str("Repository standards command center\n");
    out.push('\n');
    out.push_str(&format!(
        "Mode: {} - {}\n",
        paint(color, mode.ansi_color(), mode.label()),
        mode.description()
    ));
    out.push_str("Switch modes with /mode plan or /mode act. Type /help for details.\n");
    out.push('\n');
    out.push_str("Workspace\n");
    out.push_str("---------\n");
    out.push_str(&format!("Root:        {}\n", profile.root.display()));
    out.push_str(&format!("Config:      {}\n", profile.config_display()));
    out.push_str(&format!("Quality:     {}\n", profile.workspace_display()));
    out.push_str(&format!("Languages:   {language_display}\n"));
    out.push_str(&format!("JS runner:   {js_runner}\n"));
    out.push('\n');
    out.push_str("Release\n");
    out.push_str("-------\n");
    out.push_str(&format!("Status:      {release_status}\n"));
    out.push_str(&format!(
        "Targets:     {}\n",
        profile.stored_config.release.targets.len()
    ));
    out.push_str(&format!(
        "Workflows:   {}\n",
        release_workflow_summary(profile)
    ));
    out.push('\n');
    out.push_str("Command palette\n");
    out.push_str("---------------\n");
    for (index, action) in ACTIONS.iter().enumerate() {
        out.push_str(&format!(
            "{}  {:<10} {}\n",
            index + 1,
            action.slash,
            action.summary
        ));
    }
    out.push_str("   /mode plan  Use preview-only behavior\n");
    out.push_str("   /mode act   Enable write-capable behavior with confirmations\n");
    out.push_str("   /quit       Exit\n");
    out.push('\n');
    out.push_str("CLI equivalents\n");
    out.push_str("---------------\n");
    for action in ACTIONS {
        let cli = match mode {
            TuiMode::Plan => action.plan_cli,
            TuiMode::Act => action.act_cli,
        };
        out.push_str(cli);
        out.push('\n');
    }
    out.push('\n');
    out
}

fn print_plain_help(mode: TuiMode) {
    println!();
    println!("TUI help");
    println!("--------");
    println!("The TUI is a command palette over the normal CLI. Every action shown here");
    println!("has a direct CLI equivalent and keeps the same safety defaults.");
    println!();
    println!("Current mode: {}", mode.label());
    println!();
    println!("/plan        Read-only repository plan and release graph");
    println!("/check       Read-only policy validation");
    println!("/doctor      Read-only environment readiness inspection");
    println!("/update      PLAN: dry-run, ACT: writes managed files after prompts");
    println!("/init        PLAN: dry-run, ACT: writes bootstrap files after prompts");
    println!("/release     Opens release target management");
    println!("/mode plan   Preview-only mode");
    println!("/mode act    Write-capable mode with confirmations");
    println!("/quit        Exit the TUI");
}

fn release_workflow_summary(profile: &ProjectProfile) -> String {
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

fn language_summary(profile: &ProjectProfile) -> String {
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn inner(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(margin),
        y: area.y.saturating_add(margin),
        width: area.width.saturating_sub(margin * 2),
        height: area.height.saturating_sub(margin * 2),
    }
}

fn paint(enabled: bool, code: &str, value: &str) -> String {
    if enabled {
        format!("{code}{value}\x1b[0m")
    } else {
        value.to_string()
    }
}

fn run_and_pause(action: impl FnOnce() -> Result<()>) -> Result<()> {
    if let Err(error) = action() {
        eprintln!("Error: {error:#}");
    }
    pause()
}

fn prompt(label: &str) -> Result<String> {
    if label.ends_with('>') {
        print!("{label} ");
    } else {
        print!("{label}: ");
    }
    io::stdout().flush().context("failed to flush stdout")?;
    let mut value = String::new();
    io::stdin()
        .read_line(&mut value)
        .context("failed to read stdin")?;
    Ok(value.trim().to_string())
}

fn prompt_optional_path(label: &str) -> Result<Option<PathBuf>> {
    let value = prompt(&format!("{label} [use detected]"))?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

fn confirm(label: &str) -> Result<bool> {
    let value = prompt(&format!("{label} [y/N]"))?;
    Ok(matches!(value.as_str(), "y" | "Y" | "yes" | "YES"))
}

fn pause() -> Result<()> {
    let _ = prompt("Press Enter to continue")?;
    Ok(())
}

fn wait_for_enter() -> Result<()> {
    println!();
    let _ = prompt("Press Enter to return to repository")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_commands_and_legacy_shortcuts() {
        assert_eq!(TuiCommand::parse("/plan"), TuiCommand::Plan);
        assert_eq!(TuiCommand::parse("2"), TuiCommand::Check);
        assert_eq!(
            TuiCommand::parse("/mode act"),
            TuiCommand::Mode(TuiMode::Act)
        );
        assert_eq!(
            TuiCommand::parse("mode plan"),
            TuiCommand::Mode(TuiMode::Plan)
        );
        assert_eq!(TuiCommand::parse("/quit"), TuiCommand::Quit);
    }
}
