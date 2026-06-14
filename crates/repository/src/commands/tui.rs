use crate::cli::{
    CheckArgs, DoctorArgs, InitArgs, JsRunnerArg, LanguageArg, PlanArgs, ProjectsArgs, ReleaseArgs,
    ReleaseCommand, TuiArgs, UpdateArgs,
};
use crate::commands;
use crate::commands::interactive::{
    confirm, pause, prompt, prompt_default, prompt_optional_path,
    prompt_optional_path_with_default, wait_for_enter,
};
use crate::commands::release_flow::{self, ReleaseFlowMode};
use crate::output::{empty_as_dash, js_runner_summary, language_summary, release_workflow_summary};
use crate::project::ProjectProfile;
use anyhow::{bail, Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
use std::io::{self, IsTerminal};

const ACTIONS: &[TuiAction] = &[
    TuiAction {
        command: TuiCommand::Projects,
        slash: "/projects",
        title: "Projects",
        summary: "Inspect detected languages, Cargo packages, and release target coverage.",
        plan_cli: "repository projects --root .",
        act_cli: "repository projects --root .",
        safety: "Read-only.",
    },
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
        command: TuiCommand::Customize,
        slash: "/customize",
        title: "Customize",
        summary: "Choose workspace, languages, and JavaScript runner from the TUI.",
        plan_cli: "repository init --dry-run --workspace <path> --language <lang>",
        act_cli: "repository init --force --workspace <path> --language <lang>",
        safety: "PLAN previews generated files. ACT asks before overwriting or installing hooks.",
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
        command: TuiCommand::Update,
        slash: "/update",
        title: "Update",
        summary: "Refresh repository-managed quality files.",
        plan_cli: "repository update --dry-run --skip-mise-use --skip-hk-install",
        act_cli: "repository update",
        safety: "PLAN runs a dry-run. ACT asks before overwriting files or running installers.",
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
        command: TuiCommand::ReleaseTargets,
        slash: "/targets",
        title: "Targets",
        summary: "Add, edit, view, and remove release targets.",
        plan_cli: "repository release",
        act_cli: "repository release",
        safety: "Uses the same release target editor as the CLI.",
    },
    TuiAction {
        command: TuiCommand::ReleaseFlow,
        slash: "/release",
        title: "Release cockpit",
        summary: "Pick version and target, then plan or run release build commands.",
        plan_cli: "github-release plan && cargo-release build --dry-run",
        act_cli: "github-release plan | cargo-release build | workflow publish",
        safety: "PLAN prints commands only. ACT asks which release step to run.",
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
            TuiCommand::Mode(mode) => {
                app.mode = mode;
                app.status = format!("Switched to {} mode", mode.label());
            }
            TuiCommand::Help => {
                app.show_help = true;
                app.status = "Opened keybindings".into();
            }
            TuiCommand::Refresh => app.status = "Refreshed repository profile".into(),
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
            TuiCommand::Refresh => {
                println!("Refreshed repository profile.");
                pause()?;
            }
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

    handle_key(app, key.code, key.modifiers)
}

fn handle_key(
    app: &mut TuiApp,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<Option<TuiCommand>> {
    if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
        app.status = "Interrupted with Ctrl+C".into();
        return Ok(Some(TuiCommand::Quit));
    }

    if app.command_input.is_some() {
        return handle_command_input(app, code, modifiers);
    }

    if app.show_help {
        return handle_help_modal(app, code);
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => Ok(Some(TuiCommand::Quit)),
        KeyCode::Char('?') => Ok(Some(TuiCommand::Help)),
        KeyCode::Char('R') => Ok(Some(TuiCommand::Refresh)),
        KeyCode::Char('/') => {
            app.command_input = Some("/".into());
            app.status = "Typing slash command".into();
            Ok(None)
        }
        KeyCode::Char('p') => Ok(Some(TuiCommand::Mode(TuiMode::Plan))),
        KeyCode::Char('a') => Ok(Some(TuiCommand::Mode(TuiMode::Act))),
        KeyCode::Tab => Ok(Some(TuiCommand::Mode(app.mode.toggle()))),
        KeyCode::PageUp => {
            app.selected = app.selected.saturating_sub(5);
            Ok(None)
        }
        KeyCode::PageDown => {
            app.selected = (app.selected + 5).min(ACTIONS.len().saturating_sub(1));
            Ok(None)
        }
        KeyCode::Home | KeyCode::Char('g') | KeyCode::Char('<') => {
            app.selected = 0;
            Ok(None)
        }
        KeyCode::End | KeyCode::Char('G') | KeyCode::Char('>') => {
            app.selected = ACTIONS.len().saturating_sub(1);
            Ok(None)
        }
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

fn handle_help_modal(app: &mut TuiApp, code: KeyCode) -> Result<Option<TuiCommand>> {
    match code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
            app.show_help = false;
            app.status = "Closed keybindings".into();
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn handle_command_input(
    app: &mut TuiApp,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<Option<TuiCommand>> {
    match code {
        KeyCode::Enter => {
            let value = app.command_input.take().unwrap_or_default();
            Ok(Some(TuiCommand::parse(&value)))
        }
        KeyCode::Esc => {
            app.command_input = None;
            app.status = "Cancelled slash command".into();
            Ok(None)
        }
        KeyCode::Backspace => {
            if let Some(input) = &mut app.command_input {
                input.pop();
                if input.is_empty() {
                    app.command_input = None;
                    app.status = "Cancelled slash command".into();
                }
            }
            Ok(None)
        }
        KeyCode::Char(value) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
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
        TuiCommand::Projects => commands::projects::run(ProjectsArgs {
            root: args.root.clone(),
            config: args.config.clone(),
        }),
        TuiCommand::Plan => commands::plan::run(PlanArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            languages: Vec::new(),
            js_runner: JsRunnerArg::Auto,
            workspace: None,
        }),
        TuiCommand::Init => run_init(args, mode),
        TuiCommand::Customize => run_customize(args, mode),
        TuiCommand::Update => run_update(args, mode),
        TuiCommand::ReleaseTargets => commands::release::run(ReleaseArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            command: Some(ReleaseCommand::Tui),
        }),
        TuiCommand::ReleaseFlow => release_flow::run(args, mode.into()),
        TuiCommand::Doctor => commands::doctor::run(DoctorArgs {
            root: args.root.clone(),
            config: args.config.clone(),
        }),
        TuiCommand::Check => commands::check::run(CheckArgs {
            root: args.root.clone(),
            config: args.config.clone(),
        }),
        TuiCommand::Mode(_)
        | TuiCommand::Help
        | TuiCommand::Refresh
        | TuiCommand::Quit
        | TuiCommand::Noop => Ok(()),
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

fn run_customize(args: &TuiArgs, mode: TuiMode) -> Result<()> {
    let profile = ProjectProfile::detect(
        args.root.clone(),
        args.config.clone(),
        None,
        &[],
        JsRunnerArg::Auto,
    )?;
    println!("Customize repository standards");
    println!("------------------------------");
    println!("Leave a field empty to keep detected or configured values.");
    let workspace =
        prompt_optional_path_with_default("Quality workspace", &profile.workspace_display())?;
    let languages = prompt_default(
        "Languages [rust,js,php]",
        &profile
            .languages
            .iter()
            .map(|language| language.as_str())
            .collect::<Vec<_>>()
            .join(","),
    )?;
    let js_runner = prompt_default("JavaScript runner [auto|aube|npm|pnpm|yarn|bun]", "auto")?;
    let languages = parse_language_overrides(&languages)?;
    let js_runner = parse_js_runner(&js_runner)?;

    match mode {
        TuiMode::Plan => commands::init::run(InitArgs {
            root: args.root.clone(),
            config: args.config.clone(),
            force: false,
            dry_run: true,
            skip_mise_use: true,
            skip_hk_install: true,
            languages,
            js_runner,
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
                languages,
                js_runner,
                workspace,
                skip_style_configs: false,
                skip_actions: false,
            })
        }
    }
}

fn parse_language_overrides(value: &str) -> Result<Vec<LanguageArg>> {
    let mut languages = Vec::new();
    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let language = match item.to_ascii_lowercase().as_str() {
            "rust" => LanguageArg::Rust,
            "js" | "javascript" | "typescript" | "ts" | "vue" => LanguageArg::Js,
            "php" | "laravel" => LanguageArg::Php,
            _ => bail!("unsupported language `{item}`; use rust, js, or php"),
        };
        if !languages.contains(&language) {
            languages.push(language);
        }
    }
    Ok(languages)
}

fn parse_js_runner(value: &str) -> Result<JsRunnerArg> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "auto" => Ok(JsRunnerArg::Auto),
        "aube" => Ok(JsRunnerArg::Aube),
        "npm" => Ok(JsRunnerArg::Npm),
        "pnpm" => Ok(JsRunnerArg::Pnpm),
        "yarn" => Ok(JsRunnerArg::Yarn),
        "bun" => Ok(JsRunnerArg::Bun),
        other => bail!("unsupported JavaScript runner `{other}`"),
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

impl From<TuiMode> for ReleaseFlowMode {
    fn from(value: TuiMode) -> Self {
        match value {
            TuiMode::Plan => Self::Plan,
            TuiMode::Act => Self::Act,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TuiCommand {
    Projects,
    Plan,
    Init,
    Update,
    Customize,
    ReleaseTargets,
    ReleaseFlow,
    Doctor,
    Check,
    Mode(TuiMode),
    Help,
    Refresh,
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
            "1" | "projects" | "inventory" => Self::Projects,
            "2" | "plan" | "graph" => Self::Plan,
            "3" | "customize" | "customise" | "configure" | "config" => Self::Customize,
            "4" | "init" | "init preview" | "init dry-run" | "init-dry-run" | "init apply"
            | "bootstrap" => Self::Init,
            "5" | "update" | "update preview" | "update dry-run" | "update-dry-run"
            | "update apply" => Self::Update,
            "6" | "check" | "contracts" => Self::Check,
            "7" | "doctor" | "health" => Self::Doctor,
            "8" | "targets" | "release targets" | "release-targets" => Self::ReleaseTargets,
            "9" | "release" | "release cockpit" | "publish" | "build release" => Self::ReleaseFlow,
            "mode plan" | "plan mode" | "safe" => Self::Mode(TuiMode::Plan),
            "mode act" | "act mode" | "apply" => Self::Mode(TuiMode::Act),
            "help" | "?" => Self::Help,
            "r" | "refresh" => Self::Refresh,
            "q" | "quit" | "exit" => Self::Quit,
            _ => Self::Unknown("unrecognized command"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Projects => "projects",
            Self::Plan => "plan",
            Self::Init => "init",
            Self::Update => "update",
            Self::Customize => "customize",
            Self::ReleaseTargets => "release targets",
            Self::ReleaseFlow => "release",
            Self::Doctor => "doctor",
            Self::Check => "check",
            Self::Mode(_) => "mode switch",
            Self::Help => "help",
            Self::Refresh => "refresh",
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
    let mut lines = vec![
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
    ];

    append_context_preview(profile, action.command, &mut lines);

    let text = Text::from(lines);
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

fn append_context_preview(
    profile: &ProjectProfile,
    command: TuiCommand,
    lines: &mut Vec<Line<'static>>,
) {
    match command {
        TuiCommand::Projects => {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Detected project surface",
                Style::default().fg(Color::Cyan).bold(),
            )));
            lines.push(Line::from(format!(
                "languages: {}",
                language_summary(profile)
            )));
            lines.push(Line::from(format!(
                "js runner: {}",
                js_runner_summary(profile)
            )));
            lines.push(Line::from(format!(
                "release targets: {}",
                profile.stored_config.release.targets.len()
            )));
        }
        TuiCommand::Customize => {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Current quality config",
                Style::default().fg(Color::Cyan).bold(),
            )));
            lines.push(Line::from(format!(
                "workspace: {}",
                profile.workspace_display()
            )));
            lines.push(Line::from(format!(
                "languages: {}",
                language_summary(profile)
            )));
            lines.push(Line::from(format!(
                "js runner: {}",
                js_runner_summary(profile)
            )));
        }
        TuiCommand::ReleaseTargets | TuiCommand::ReleaseFlow => {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "Release targets",
                Style::default().fg(Color::Cyan).bold(),
            )));
            if profile.stored_config.release.targets.is_empty() {
                lines.push(Line::from("-"));
            } else {
                for target in profile.stored_config.release.targets.iter().take(8) {
                    lines.push(Line::from(format!(
                        "{} -> {} ({})",
                        target.name,
                        empty_as_dash(&target.repository),
                        empty_as_dash(&target.cargo_binary)
                    )));
                }
                if profile.stored_config.release.targets.len() > 8 {
                    lines.push(Line::from(format!(
                        "... {} more",
                        profile.stored_config.release.targets.len() - 8
                    )));
                }
            }
        }
        _ => {}
    }
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &TuiApp) {
    let input = app
        .command_input
        .as_deref()
        .map(|value| format!(" command: {value}"))
        .unwrap_or_else(|| {
            " enter run  j/k move  g/G edge  / command  ? keys  R refresh  Esc/q back/quit  Ctrl+C quit".into()
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
            "Keybindings",
            Style::default().fg(Color::Cyan).bold(),
        )),
        Line::raw(""),
        Line::raw("Enter          run selected action"),
        Line::raw("1-9            run action directly"),
        Line::raw("Up/Down j/k    move selection"),
        Line::raw("PgUp/PgDn      move five actions"),
        Line::raw("g/G < >        jump to first or last action"),
        Line::raw("/              type a slash command"),
        Line::raw("Tab p a        toggle or set PLAN/ACT"),
        Line::raw("R              refresh dashboard state"),
        Line::raw("?              open/close this keybindings modal"),
        Line::raw("Esc/q          close modal first, quit at top level"),
        Line::raw("Ctrl+C         quit immediately"),
    ]);
    frame.render_widget(
        Paragraph::new(text).alignment(Alignment::Left).block(
            Block::default()
                .title(" keybindings ")
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
    out.push_str("   R / /refresh Refresh dashboard state\n");
    out.push_str("   Esc/q       Close modal first, then exit from the top level\n");
    out.push_str("   Ctrl+C      Exit immediately in fullscreen mode\n");
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
    println!("/projects    Read-only project and release target inventory");
    println!("/plan        Read-only repository plan and release graph");
    println!("/customize   Configure workspace, languages, and JS runner");
    println!("/update      PLAN: dry-run, ACT: writes managed files after prompts");
    println!("/check       Read-only policy validation");
    println!("/doctor      Read-only environment readiness inspection");
    println!("/targets     Opens release target management");
    println!("/release     Opens release cockpit for version/target/build flow");
    println!("/refresh     Read-only dashboard refresh");
    println!("/mode plan   Preview-only mode");
    println!("/mode act    Write-capable mode with confirmations");
    println!("Esc/q        Close modal first, then exit from the top level");
    println!("Ctrl+C       Exit immediately in fullscreen mode");
    println!("/quit        Exit the TUI from slash command mode");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_commands_and_legacy_shortcuts() {
        assert_eq!(TuiCommand::parse("/projects"), TuiCommand::Projects);
        assert_eq!(TuiCommand::parse("/plan"), TuiCommand::Plan);
        assert_eq!(TuiCommand::parse("2"), TuiCommand::Plan);
        assert_eq!(TuiCommand::parse("/check"), TuiCommand::Check);
        assert_eq!(TuiCommand::parse("/customize"), TuiCommand::Customize);
        assert_eq!(TuiCommand::parse("/targets"), TuiCommand::ReleaseTargets);
        assert_eq!(TuiCommand::parse("/release"), TuiCommand::ReleaseFlow);
        assert_eq!(TuiCommand::parse("/refresh"), TuiCommand::Refresh);
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

    #[test]
    fn escape_and_q_close_help_before_quitting() {
        let mut app = TuiApp {
            show_help: true,
            ..TuiApp::default()
        };

        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE).unwrap(),
            None
        );
        assert!(!app.show_help);
        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE).unwrap(),
            Some(TuiCommand::Quit)
        );

        app.show_help = true;
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('q'), KeyModifiers::NONE).unwrap(),
            None
        );
        assert!(!app.show_help);
        assert_eq!(
            handle_key(&mut app, KeyCode::Char('q'), KeyModifiers::NONE).unwrap(),
            Some(TuiCommand::Quit)
        );
    }

    #[test]
    fn escape_cancels_command_input_before_quitting() {
        let mut app = TuiApp {
            command_input: Some("/rel".into()),
            ..TuiApp::default()
        };

        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE).unwrap(),
            None
        );
        assert!(app.command_input.is_none());
        assert_eq!(
            handle_key(&mut app, KeyCode::Esc, KeyModifiers::NONE).unwrap(),
            Some(TuiCommand::Quit)
        );
    }

    #[test]
    fn ctrl_c_quits_immediately_even_when_modal_is_open() {
        let mut app = TuiApp {
            show_help: true,
            ..TuiApp::default()
        };

        assert_eq!(
            handle_key(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL).unwrap(),
            Some(TuiCommand::Quit)
        );
        assert!(app.show_help);
    }

    #[test]
    fn lazygit_style_navigation_shortcuts_work() {
        let mut app = TuiApp {
            selected: 4,
            ..TuiApp::default()
        };

        assert_eq!(
            handle_key(&mut app, KeyCode::Char('g'), KeyModifiers::NONE).unwrap(),
            None
        );
        assert_eq!(app.selected, 0);

        assert_eq!(
            handle_key(&mut app, KeyCode::Char('G'), KeyModifiers::SHIFT).unwrap(),
            None
        );
        assert_eq!(app.selected, ACTIONS.len() - 1);

        assert_eq!(
            handle_key(&mut app, KeyCode::Char('R'), KeyModifiers::SHIFT).unwrap(),
            Some(TuiCommand::Refresh)
        );
    }
}
