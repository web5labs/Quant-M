use crate::config::{ChannelPreference, Config, ModelPreference};
use crate::cost_ledger;
use crate::domain;
use crate::execution_runtime::{self, WorkflowRunResult};
use crate::sessions::{self, SessionId};
use crate::shared_state;
use crate::state_review;
use crate::workflow_registry::{self, WorkflowId};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::path::Path;
use std::time::Duration;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_VISIBLE_ITEMS: usize = 8;
const MOCK_RESEARCH_WORKFLOW: &str = "workflow:mock-research-brief";
const CHAT_INPUT_HINT: &str =
    "/help /state [domain] /cost [session] /replay <session> /ask <question> /refresh /quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiView {
    Overview,
    Doctor,
    Workflows,
    Sessions,
    SharedState,
}

#[derive(Debug, Clone)]
struct TuiSnapshot {
    workspace_path: String,
    runtime_profile: String,
    external_network_posture: String,
    preferred_local_model: String,
    preferred_openrouter_model: String,
    session_count: usize,
    shared_state_count: usize,
    available_domains: Vec<String>,
    available_workflows: Vec<String>,
    sessions_preview: Vec<String>,
    shared_state_preview: Vec<String>,
}

#[derive(Debug, Clone)]
struct DoctorSnapshot {
    config_exists: bool,
    workspace_exists: bool,
    state_path_exists: bool,
    session_path_exists: bool,
    workflow_run_ok: bool,
    shared_state_list_ok: bool,
    session_list_ok: bool,
    generated_session_id: Option<String>,
}

#[derive(Debug, Clone)]
struct TuiApp {
    snapshot: TuiSnapshot,
    doctor: Option<DoctorSnapshot>,
    last_run: Option<WorkflowRunResult>,
    notice: String,
    view: TuiView,
}

pub fn run(cfg: &Config, config_path: &Path) -> Result<()> {
    let snapshot = collect_snapshot(cfg)?;
    let mut app = TuiApp {
        snapshot,
        doctor: None,
        last_run: None,
        notice: "✨ Ready. Press r to run mock-research, d for doctor, q to quit.".to_string(),
        view: TuiView::Overview,
    };

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, cfg, config_path, &mut app);
    ratatui::restore();
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TuiAction {
    Help,
    Quit,
    Refresh,
    ShowState { domain: Option<String> },
    ShowCost { session_id: Option<String> },
    Replay { session_id: String },
    AskInspect { question: String },
    ConsensusDryRun { prompt: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiStorageMode {
    InspectOnly,
    DryRunWritesAllowed,
    StateWritesAllowed,
    RequiresApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatMessageKind {
    HumanInput,
    DisplayOnlyNote,
    DryRunResult,
    ReplaySummary,
    PolicyDecision,
    StateRecord,
    CostRecord,
    WorkerProposal,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatLayoutMode {
    Compact,
    Wide,
}

#[derive(Debug, Clone)]
struct ChatMessage {
    kind: ChatMessageKind,
    storage_mode: TuiStorageMode,
    body: String,
}

#[derive(Debug, Clone)]
struct ChatApp {
    inspect_only: bool,
    messages: Vec<ChatMessage>,
    input: String,
    notice: String,
    snapshot: TuiSnapshot,
}

pub fn run_chat(cfg: &Config, _config_path: &Path, inspect: bool) -> Result<()> {
    let mut app = ChatApp {
        inspect_only: inspect,
        messages: initial_chat_messages(inspect),
        input: String::new(),
        notice: "Inspect-first chat cockpit. No provider calls, worker writes, or hidden session mutation.".to_string(),
        snapshot: collect_snapshot(cfg)?,
    };

    let mut terminal = ratatui::init();
    let result = run_chat_app(&mut terminal, cfg, &mut app);
    ratatui::restore();
    result
}

fn initial_chat_messages(inspect: bool) -> Vec<ChatMessage> {
    vec![ChatMessage {
        kind: ChatMessageKind::DisplayOnlyNote,
        storage_mode: TuiStorageMode::InspectOnly,
        body: format!(
            "Quant-M TUI chat is an evidence cockpit, not an authority surface. mode={}",
            if inspect {
                "inspect"
            } else {
                "inspect-default"
            }
        ),
    }]
}

fn run_chat_app(terminal: &mut DefaultTerminal, cfg: &Config, app: &mut ChatApp) -> Result<()> {
    loop {
        terminal.draw(|frame| render_chat(frame, app))?;
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Esc => break Ok(()),
                    KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.input.push('\n');
                    }
                    KeyCode::Enter => {
                        let input = app.input.trim().to_string();
                        app.input.clear();
                        if input.is_empty() {
                            continue;
                        }
                        app.messages.push(ChatMessage {
                            kind: ChatMessageKind::HumanInput,
                            storage_mode: TuiStorageMode::InspectOnly,
                            body: input.clone(),
                        });
                        let action = parse_tui_action(&input);
                        if action == TuiAction::Quit {
                            break Ok(());
                        }
                        handle_tui_action(cfg, app, action);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break Ok(());
                    }
                    KeyCode::Char(ch) => {
                        app.input.push(ch);
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

fn parse_tui_action(input: &str) -> TuiAction {
    let trimmed = input.trim();
    let (command, rest) = trimmed
        .split_once(char::is_whitespace)
        .unwrap_or((trimmed, ""));
    match command.to_ascii_lowercase().as_str() {
        "/quit" | "/exit" => return TuiAction::Quit,
        "/help" | "?" => return TuiAction::Help,
        "/refresh" => return TuiAction::Refresh,
        "/state" => {
            let domain = rest.trim();
            return TuiAction::ShowState {
                domain: (!domain.is_empty()).then(|| domain.to_string()),
            };
        }
        "/cost" => {
            let session = rest.trim();
            return TuiAction::ShowCost {
                session_id: (!session.is_empty()).then(|| session.to_string()),
            };
        }
        "/replay" => {
            return TuiAction::Replay {
                session_id: rest.trim().to_string(),
            };
        }
        "/consensus" => {
            return TuiAction::ConsensusDryRun {
                prompt: rest.trim().to_string(),
            };
        }
        "/ask" => {
            return TuiAction::AskInspect {
                question: rest.trim().to_string(),
            };
        }
        _ => {}
    }
    TuiAction::AskInspect {
        question: trimmed.to_string(),
    }
}

fn storage_mode_for_action(action: &TuiAction) -> TuiStorageMode {
    match action {
        TuiAction::Help
        | TuiAction::Quit
        | TuiAction::Refresh
        | TuiAction::ShowState { .. }
        | TuiAction::ShowCost { .. }
        | TuiAction::Replay { .. }
        | TuiAction::AskInspect { .. } => TuiStorageMode::InspectOnly,
        TuiAction::ConsensusDryRun { .. } => TuiStorageMode::DryRunWritesAllowed,
    }
}

fn handle_tui_action(cfg: &Config, app: &mut ChatApp, action: TuiAction) {
    let storage_mode = storage_mode_for_action(&action);
    if app.inspect_only && storage_mode != TuiStorageMode::InspectOnly {
        app.messages.push(ChatMessage {
            kind: ChatMessageKind::PolicyDecision,
            storage_mode,
            body: format!(
                "Blocked in inspect mode. Action requires {:?}; no artifacts were written.",
                storage_mode
            ),
        });
        return;
    }

    let message = match action {
        TuiAction::Help => ChatMessage {
            kind: ChatMessageKind::DisplayOnlyNote,
            storage_mode,
            body: format!(
                "Typed actions only. {CHAT_INPUT_HINT}. Chat text is display/navigation input, not runtime authority.\nstorage modes: {}\nmessage provenance: {}",
                known_storage_modes_label(),
                known_message_kinds_label()
            ),
        },
        TuiAction::Refresh => match collect_snapshot(cfg) {
            Ok(snapshot) => {
                app.snapshot = snapshot;
                ChatMessage {
                    kind: ChatMessageKind::DisplayOnlyNote,
                    storage_mode,
                    body: "Refreshed inspect snapshot from config, sessions, and shared state."
                        .to_string(),
                }
            }
            Err(err) => ChatMessage {
                kind: ChatMessageKind::Error,
                storage_mode,
                body: format!("Refresh failed: {err}"),
            },
        },
        TuiAction::ShowState { domain } => match state_review::review_state(cfg, domain.as_deref())
        {
            Ok(report) => ChatMessage {
                kind: ChatMessageKind::StateRecord,
                storage_mode,
                body: state_review::render_state_review(&report),
            },
            Err(err) => ChatMessage {
                kind: ChatMessageKind::Error,
                storage_mode,
                body: format!("State review failed: {err}"),
            },
        },
        TuiAction::ShowCost { session_id } => {
            match cost_ledger::summarize_costs(cfg, None, session_id.as_deref()) {
                Ok(summary) => ChatMessage {
                    kind: ChatMessageKind::CostRecord,
                    storage_mode,
                    body: cost_ledger::render_cost_summary(&summary),
                },
                Err(err) => ChatMessage {
                    kind: ChatMessageKind::Error,
                    storage_mode,
                    body: format!("Cost summary failed: {err}"),
                },
            }
        }
        TuiAction::Replay { session_id } if session_id.trim().is_empty() => ChatMessage {
            kind: ChatMessageKind::Error,
            storage_mode,
            body: "Usage: /replay <session_id>".to_string(),
        },
        TuiAction::Replay { session_id } => match session_id.parse::<SessionId>() {
            Ok(session_id) => match sessions::replay_session(cfg, &session_id) {
                Ok(replay) => ChatMessage {
                    kind: ChatMessageKind::ReplaySummary,
                    storage_mode,
                    body: render_session_replay_for_chat(&replay),
                },
                Err(err) => ChatMessage {
                    kind: ChatMessageKind::Error,
                    storage_mode,
                    body: format!("Replay failed: {err}"),
                },
            },
            Err(err) => ChatMessage {
                kind: ChatMessageKind::Error,
                storage_mode,
                body: format!("Invalid session id: {err}"),
            },
        },
        TuiAction::AskInspect { question } if question.trim().is_empty() => ChatMessage {
            kind: ChatMessageKind::DisplayOnlyNote,
            storage_mode,
            body:
                "Ask inspect needs text. Example: /ask what evidence exists for the last decision?"
                    .to_string(),
        },
        TuiAction::AskInspect { question } => ChatMessage {
            kind: ChatMessageKind::DisplayOnlyNote,
            storage_mode,
            body: format!(
                "Inspect question recorded as display-only navigation text. No provider call was made.\nquestion: {question}\nnext: use /state, /cost, or /replay <session_id> to inspect structured truth."
            ),
        },
        TuiAction::ConsensusDryRun { .. } => ChatMessage {
            kind: ChatMessageKind::PolicyDecision,
            storage_mode,
            body: "Consensus dry-run is not enabled in this inspect MVP.".to_string(),
        },
        TuiAction::Quit => ChatMessage {
            kind: ChatMessageKind::DisplayOnlyNote,
            storage_mode,
            body: "Quit.".to_string(),
        },
    };
    app.notice = format!("last={:?} storage={:?}", message.kind, message.storage_mode);
    app.messages.push(message);
}

fn known_storage_modes_label() -> String {
    [
        TuiStorageMode::InspectOnly,
        TuiStorageMode::DryRunWritesAllowed,
        TuiStorageMode::StateWritesAllowed,
        TuiStorageMode::RequiresApproval,
    ]
    .into_iter()
    .map(|mode| format!("{mode:?}"))
    .collect::<Vec<_>>()
    .join(", ")
}

fn known_message_kinds_label() -> String {
    [
        ChatMessageKind::HumanInput,
        ChatMessageKind::DisplayOnlyNote,
        ChatMessageKind::DryRunResult,
        ChatMessageKind::ReplaySummary,
        ChatMessageKind::PolicyDecision,
        ChatMessageKind::StateRecord,
        ChatMessageKind::CostRecord,
        ChatMessageKind::WorkerProposal,
        ChatMessageKind::Error,
    ]
    .into_iter()
    .map(|kind| format!("{kind:?}"))
    .collect::<Vec<_>>()
    .join(", ")
}

fn render_session_replay_for_chat(replay: &sessions::SessionReplay) -> String {
    format!(
        "Session replay\nsession_id: {}\nstatus: {}\ndomain: {}\nevents: {}\npolicy_decisions: {}\npolicy_denials: {}\nside_effects_replayed: {}\nnext: session replay is inspect-only; no side effects executed.",
        replay.summary.session_id,
        replay.summary.final_status,
        replay.summary.domain_id,
        replay.summary.event_count,
        replay.state.policy_decisions,
        replay.state.policy_denials,
        replay.state.side_effects_replayed,
    )
}

fn render_chat(frame: &mut Frame, app: &ChatApp) {
    let area = frame.area();
    let layout_mode = chat_layout_mode(area.width);
    let compact = layout_mode == ChatLayoutMode::Compact;
    let [header, body, input] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(5),
    ])
    .areas(area);
    render_chat_header(frame, header, app, compact);
    if compact {
        render_chat_messages(frame, body, app, true);
    } else {
        let [chat_area, rail_area] =
            Layout::horizontal([Constraint::Percentage(72), Constraint::Percentage(28)])
                .areas(body);
        render_chat_messages(frame, chat_area, app, false);
        render_chat_evidence_rail(frame, rail_area, app);
    }
    render_chat_input(frame, input, app);
}

fn render_chat_header(frame: &mut Frame, area: Rect, app: &ChatApp, compact: bool) {
    let mode = if app.inspect_only {
        "inspect"
    } else {
        "inspect-default"
    };
    let text = Line::from(vec![
        Span::styled(
            "Quant-M TUI Chat ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "mode={mode} sessions={} state={} local_model={} openrouter={} layout={}",
            app.snapshot.session_count,
            app.snapshot.shared_state_count,
            app.snapshot.preferred_local_model,
            app.snapshot.preferred_openrouter_model,
            if compact { "compact" } else { "wide" }
        )),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(panel_block(" Evidence Cockpit ", Color::Cyan))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_chat_messages(frame: &mut Frame, area: Rect, app: &ChatApp, compact: bool) {
    let max_messages = area.height.saturating_sub(2).max(1) as usize;
    let visible = app
        .messages
        .iter()
        .rev()
        .take(max_messages)
        .collect::<Vec<_>>()
        .into_iter()
        .rev();
    let mut lines = Vec::new();
    for message in visible {
        let style = chat_message_style(message.kind);
        let alignment = if compact || message.kind != ChatMessageKind::HumanInput {
            Alignment::Left
        } else {
            Alignment::Right
        };
        let rendered = render_chat_message_text(message);
        lines.push(Line::from(Span::styled(rendered, style)).alignment(alignment));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block(" Chat-Shaped Evidence ", Color::Magenta))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn chat_layout_mode(width: u16) -> ChatLayoutMode {
    if width < 96 {
        ChatLayoutMode::Compact
    } else {
        ChatLayoutMode::Wide
    }
}

fn render_chat_message_text(message: &ChatMessage) -> String {
    let prefix = chat_message_prefix(message.kind);
    let body = message.body.lines().next().unwrap_or("");
    format!(
        "{prefix} [{:?}/{:?}] {body}",
        message.kind, message.storage_mode
    )
}

fn render_chat_evidence_rail(frame: &mut Frame, area: Rect, app: &ChatApp) {
    let last = app.messages.last();
    let items = vec![
        ListItem::new(format!("workspace: {}", app.snapshot.workspace_path)),
        ListItem::new(format!(
            "network: {}",
            app.snapshot.external_network_posture
        )),
        ListItem::new(format!("sessions: {}", app.snapshot.session_count)),
        ListItem::new(format!("state rows: {}", app.snapshot.shared_state_count)),
        ListItem::new(format!(
            "last kind: {}",
            last.map(|message| format!("{:?}", message.kind))
                .unwrap_or_else(|| "none".to_string())
        )),
        ListItem::new(format!(
            "last storage: {}",
            last.map(|message| format!("{:?}", message.storage_mode))
                .unwrap_or_else(|| "none".to_string())
        )),
        ListItem::new("truth: sessions/state/replay/cost/policy"),
    ];
    frame.render_widget(
        List::new(items).block(panel_block(" Evidence Rail ", Color::Green)),
        area,
    );
}

fn render_chat_input(frame: &mut Frame, area: Rect, app: &ChatApp) {
    let text = Text::from(vec![
        Line::from(app.input.clone()),
        Line::from(Span::styled(
            format!("{CHAT_INPUT_HINT} | Ctrl+Enter newline | Esc quit"),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            app.notice.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(panel_block(" Input ", Color::Blue))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn chat_message_prefix(kind: ChatMessageKind) -> &'static str {
    match kind {
        ChatMessageKind::HumanInput => "You:",
        ChatMessageKind::DisplayOnlyNote => "Note:",
        ChatMessageKind::DryRunResult => "Dry-run:",
        ChatMessageKind::ReplaySummary => "Replay:",
        ChatMessageKind::PolicyDecision => "Policy:",
        ChatMessageKind::StateRecord => "State:",
        ChatMessageKind::CostRecord => "Cost:",
        ChatMessageKind::WorkerProposal => "Proposal:",
        ChatMessageKind::Error => "Error:",
    }
}

fn chat_message_style(kind: ChatMessageKind) -> Style {
    match kind {
        ChatMessageKind::HumanInput => Style::default().fg(Color::LightBlue),
        ChatMessageKind::DisplayOnlyNote => Style::default().fg(Color::Gray),
        ChatMessageKind::DryRunResult => Style::default().fg(Color::Yellow),
        ChatMessageKind::ReplaySummary => Style::default().fg(Color::Green),
        ChatMessageKind::PolicyDecision => Style::default().fg(Color::LightRed),
        ChatMessageKind::StateRecord => Style::default().fg(Color::Cyan),
        ChatMessageKind::CostRecord => Style::default().fg(Color::LightMagenta),
        ChatMessageKind::WorkerProposal => Style::default().fg(Color::Magenta),
        ChatMessageKind::Error => Style::default().fg(Color::Red),
    }
}

fn run_app(
    terminal: &mut DefaultTerminal,
    cfg: &Config,
    config_path: &Path,
    app: &mut TuiApp,
) -> Result<()> {
    loop {
        terminal.draw(|frame| render(frame, app))?;
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Char('d') => {
                        app.doctor = Some(run_doctor_snapshot(cfg, config_path));
                        app.view = TuiView::Doctor;
                        app.notice = "🩺 Doctor refreshed.".to_string();
                    }
                    KeyCode::Char('w') => {
                        app.snapshot = collect_snapshot(cfg)?;
                        app.view = TuiView::Workflows;
                        app.notice = "⚙ Workflow list refreshed.".to_string();
                    }
                    KeyCode::Char('s') => {
                        app.snapshot = collect_snapshot(cfg)?;
                        app.view = TuiView::Sessions;
                        app.notice = "📜 Session list refreshed.".to_string();
                    }
                    KeyCode::Char('t') => {
                        app.snapshot = collect_snapshot(cfg)?;
                        app.view = TuiView::SharedState;
                        app.notice = "🧠 Shared-state list refreshed.".to_string();
                    }
                    KeyCode::Char('o') => {
                        app.snapshot = collect_snapshot(cfg)?;
                        app.view = TuiView::Overview;
                        app.notice = "✨ Overview refreshed.".to_string();
                    }
                    KeyCode::Char('r') => {
                        let workflow_id = WorkflowId::new(MOCK_RESEARCH_WORKFLOW);
                        match execution_runtime::run_workflow(cfg, &workflow_id) {
                            Ok(result) => {
                                app.last_run = Some(result.clone());
                                app.snapshot = collect_snapshot(cfg)?;
                                app.view = TuiView::Overview;
                                app.notice = format!(
                                    "✓ Workflow ok: {} session={}",
                                    result.workflow_id, result.session_id
                                );
                            }
                            Err(err) => {
                                app.notice = format!("Workflow failed: {err}");
                            }
                        }
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

fn collect_snapshot(cfg: &Config) -> Result<TuiSnapshot> {
    let domains = domain::builtin_registry()?
        .list()
        .into_iter()
        .map(|pack| format!("{} ({})", pack.domain_id, pack.name))
        .collect::<Vec<_>>();
    let workflows = workflow_registry::builtin_registry()?
        .list(None)
        .into_iter()
        .map(|workflow| workflow.workflow_id.to_string())
        .collect::<Vec<_>>();
    let sessions = sessions::list_sessions(cfg)?;
    let shared_state_rows = shared_state::list_state(cfg, None)?;

    Ok(TuiSnapshot {
        workspace_path: cfg.workspace_dir.display().to_string(),
        runtime_profile: format!("{:?}", cfg.runtime.profile).to_lowercase(),
        external_network_posture: if cfg.runtime.external_network_enabled {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        },
        preferred_local_model: format_model_preference(
            cfg.preferences.preferred_local_model.as_ref(),
        ),
        preferred_openrouter_model: cfg
            .preferences
            .preferred_openrouter_model
            .clone()
            .unwrap_or_else(|| "unset".to_string()),
        session_count: sessions.len(),
        shared_state_count: shared_state_rows.len(),
        available_domains: domains,
        available_workflows: workflows,
        sessions_preview: sessions
            .into_iter()
            .take(MAX_VISIBLE_ITEMS)
            .map(|session| {
                format!(
                    "{} [{}] {}",
                    session.session_id, session.final_status, session.domain_id
                )
            })
            .collect(),
        shared_state_preview: shared_state_rows
            .into_iter()
            .take(MAX_VISIBLE_ITEMS)
            .map(|record| format!("{} [{}]", record.key, record.domain_id))
            .collect(),
    })
}

fn run_doctor_snapshot(cfg: &Config, config_path: &Path) -> DoctorSnapshot {
    let workflow_run =
        execution_runtime::run_workflow(cfg, &WorkflowId::new(MOCK_RESEARCH_WORKFLOW));
    DoctorSnapshot {
        config_exists: config_path.exists(),
        workspace_exists: cfg.workspace_dir.exists(),
        state_path_exists: cfg
            .state_sql
            .sqlite_path
            .parent()
            .is_some_and(|parent| parent.exists()),
        session_path_exists: cfg.runtime.session_dir.exists(),
        workflow_run_ok: workflow_run.is_ok(),
        shared_state_list_ok: shared_state::list_state(cfg, None).is_ok(),
        session_list_ok: sessions::list_sessions(cfg).is_ok(),
        generated_session_id: workflow_run
            .ok()
            .map(|result| result.session_id.to_string()),
    }
}

fn render(frame: &mut Frame, app: &TuiApp) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(frame.area());

    render_header(frame, header, app);
    render_body(frame, body, app);
    render_footer(frame, footer, app);
}

fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let title = Paragraph::new(Text::from(vec![
        Line::from(vec![
            Span::styled(
                "🧠 Quant-M ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("v{APP_VERSION}"),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
        Line::from(vec![
            Span::styled("Workspace: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.snapshot.workspace_path.clone(),
                Style::default().fg(Color::White),
            ),
            Span::raw("  "),
            Span::styled("Profile: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.snapshot.runtime_profile.clone(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Network: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                app.snapshot.external_network_posture.clone(),
                if app.snapshot.external_network_posture == "enabled" {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::LightRed)
                },
            ),
        ]),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue))
            .title(" Runtime "),
    );
    frame.render_widget(title, area);
}

fn render_body(frame: &mut Frame, area: Rect, app: &TuiApp) {
    match app.view {
        TuiView::Overview => render_overview(frame, area, app),
        TuiView::Doctor => render_doctor(frame, area, app),
        TuiView::Workflows => render_list_view(
            frame,
            area,
            "Workflows",
            &app.snapshot.available_workflows,
            "Registered workflow descriptors; use r to run mock-research.",
        ),
        TuiView::Sessions => render_list_view(
            frame,
            area,
            "Sessions",
            &app.snapshot.sessions_preview,
            "Recent sessions from append-only logs.",
        ),
        TuiView::SharedState => render_list_view(
            frame,
            area,
            "Shared State",
            &app.snapshot.shared_state_preview,
            "Current normalized shared-state records.",
        ),
    }
}

fn render_overview(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let [left, right, bottom] = Layout::vertical([
        Constraint::Length(11),
        Constraint::Length(8),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [top_left, top_right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(left);

    let overview = Paragraph::new(Text::from(vec![
        metric_line(
            "🧩 Preferred local model",
            &app.snapshot.preferred_local_model,
        ),
        metric_line(
            "🌐 Preferred OpenRouter model",
            &app.snapshot.preferred_openrouter_model,
        ),
        metric_line("📜 Session count", &app.snapshot.session_count.to_string()),
        metric_line(
            "🧠 Shared-state count",
            &app.snapshot.shared_state_count.to_string(),
        ),
        Line::from("CLI remains primary for Staff OS and cmux.").style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
    ]))
    .block(panel_block(" Overview ", Color::Cyan))
    .wrap(Wrap { trim: true });
    frame.render_widget(overview, top_left);

    let domains = list_widget(
        "Domains",
        &app.snapshot.available_domains,
        "No domains registered.",
    );
    frame.render_widget(domains, top_right);

    let workflows = list_widget(
        "Available Workflows",
        &app.snapshot.available_workflows,
        "No workflows registered.",
    );
    frame.render_widget(workflows, right);

    let mut lines = vec![Line::from(app.notice.clone())];
    if let Some(last_run) = &app.last_run {
        lines.push(Line::from(format!(
            "Last run: {} status={} session={}",
            last_run.workflow_id, last_run.status, last_run.session_id
        )));
    } else {
        lines.push(Line::from("Last run: none yet"));
    }
    let status = Paragraph::new(Text::from(lines))
        .block(panel_block(" Last Action ", Color::Green))
        .wrap(Wrap { trim: true });
    frame.render_widget(status, bottom);
}

fn render_doctor(frame: &mut Frame, area: Rect, app: &TuiApp) {
    frame.render_widget(Clear, area);
    let lines = match &app.doctor {
        Some(doctor) => vec![
            check_line("config_exists", doctor.config_exists),
            check_line("workspace_exists", doctor.workspace_exists),
            check_line("state_path_exists", doctor.state_path_exists),
            check_line("session_path_exists", doctor.session_path_exists),
            check_line("workflow_run_ok", doctor.workflow_run_ok),
            check_line("shared_state_list_ok", doctor.shared_state_list_ok),
            check_line("session_list_ok", doctor.session_list_ok),
            metric_line(
                "generated_session_id",
                doctor.generated_session_id.as_deref().unwrap_or("none"),
            ),
        ],
        None => vec![
            Line::from("Doctor has not been run in this TUI session.")
                .style(Style::default().fg(Color::Yellow)),
            Line::from("Press d to run the local doctor checks.").style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ],
    };
    let paragraph = Paragraph::new(Text::from(lines))
        .block(panel_block(" 🩺 Doctor ", Color::Yellow))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn render_list_view(frame: &mut Frame, area: Rect, title: &str, items: &[String], hint: &str) {
    let [top, bottom] = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(area);
    frame.render_widget(
        list_widget(title, items, &format!("No {title} available.")),
        top,
    );
    frame.render_widget(
        Paragraph::new(hint)
            .style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )
            .block(panel_block(" Note ", Color::DarkGray))
            .wrap(Wrap { trim: true }),
        bottom,
    );
}

fn list_widget<'a>(title: &'a str, items: &'a [String], empty: &'a str) -> List<'a> {
    let list_items = if items.is_empty() {
        vec![ListItem::new(empty.to_string()).style(Style::default().fg(Color::DarkGray))]
    } else {
        items
            .iter()
            .map(|item| ListItem::new(format!("• {item}")).style(Style::default().fg(Color::White)))
            .collect()
    };
    List::new(list_items).block(panel_block(title, Color::Magenta))
}

fn panel_block(title: &str, color: Color) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title.to_string())
}

fn metric_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value.to_string(), Style::default().fg(Color::White)),
    ])
}

fn check_line(label: &str, ok: bool) -> Line<'static> {
    let (icon, color, value) = if ok {
        ("✓", Color::Green, "ok")
    } else {
        ("!", Color::LightRed, "needs attention")
    };
    Line::from(vec![
        Span::styled(
            format!("{icon} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{label}: "), Style::default().fg(Color::White)),
        Span::styled(value.to_string(), Style::default().fg(color)),
    ])
}

fn key_span(value: &str) -> Span<'static> {
    Span::styled(
        value.to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )
}

fn render_footer(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let help = Paragraph::new(Text::from(vec![
        Line::from(vec![
            key_span("q"),
            Span::raw(" quit  "),
            key_span("d"),
            Span::raw(" doctor  "),
            key_span("w"),
            Span::raw(" workflows  "),
            key_span("s"),
            Span::raw(" sessions  "),
            key_span("t"),
            Span::raw(" state  "),
            key_span("r"),
            Span::raw(" run demo  "),
            key_span("o"),
            Span::raw(" overview"),
        ]),
        Line::from(app.notice.clone()).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        ),
    ]))
    .block(panel_block(" Shortcuts ", Color::Blue))
    .wrap(Wrap { trim: true });
    frame.render_widget(help, area);
}

fn format_model_preference(preference: Option<&ModelPreference>) -> String {
    preference
        .map(|model| format!("{} {}", model.provider, model.model))
        .unwrap_or_else(|| "unset".to_string())
}

#[allow(dead_code)]
fn format_channel_preference(preference: &ChannelPreference) -> String {
    match &preference.value {
        Some(value) => format!("{:?}: {value}", preference.channel).to_lowercase(),
        None => format!("{:?}", preference.channel).to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_snapshot() -> TuiSnapshot {
        TuiSnapshot {
            workspace_path: "workspace".to_string(),
            runtime_profile: "edge".to_string(),
            external_network_posture: "disabled".to_string(),
            preferred_local_model: "unset".to_string(),
            preferred_openrouter_model: "unset".to_string(),
            session_count: 0,
            shared_state_count: 0,
            available_domains: Vec::new(),
            available_workflows: Vec::new(),
            sessions_preview: Vec::new(),
            shared_state_preview: Vec::new(),
        }
    }

    fn test_chat_app(inspect_only: bool) -> ChatApp {
        ChatApp {
            inspect_only,
            messages: Vec::new(),
            input: String::new(),
            notice: String::new(),
            snapshot: test_snapshot(),
        }
    }

    fn test_config(tmp: &TempDir) -> Config {
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().join("workspace");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg
    }

    #[test]
    fn chat_parser_maps_slash_commands_to_typed_actions() {
        assert_eq!(parse_tui_action("/help"), TuiAction::Help);
        assert_eq!(
            parse_tui_action("/state domain:consensus"),
            TuiAction::ShowState {
                domain: Some("domain:consensus".to_string())
            }
        );
        assert_eq!(
            parse_tui_action("/cost session-1"),
            TuiAction::ShowCost {
                session_id: Some("session-1".to_string())
            }
        );
        assert_eq!(
            parse_tui_action("/replay session-1"),
            TuiAction::Replay {
                session_id: "session-1".to_string()
            }
        );
        assert_eq!(
            parse_tui_action("/ask what evidence exists?"),
            TuiAction::AskInspect {
                question: "what evidence exists?".to_string()
            }
        );
    }

    #[test]
    fn chat_parser_does_not_prefix_match_unknown_commands() {
        assert_eq!(
            parse_tui_action("/stateful thinking"),
            TuiAction::AskInspect {
                question: "/stateful thinking".to_string()
            }
        );
    }

    #[test]
    fn chat_action_storage_modes_are_explicit() {
        assert_eq!(
            storage_mode_for_action(&TuiAction::ShowState { domain: None }),
            TuiStorageMode::InspectOnly
        );
        assert_eq!(
            storage_mode_for_action(&TuiAction::ConsensusDryRun {
                prompt: "test".to_string()
            }),
            TuiStorageMode::DryRunWritesAllowed
        );
        assert!(known_storage_modes_label().contains("RequiresApproval"));
        assert!(known_message_kinds_label().contains("WorkerProposal"));
    }

    #[test]
    fn consensus_is_blocked_in_inspect_mode_without_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let mut app = test_chat_app(true);

        handle_tui_action(
            &cfg,
            &mut app,
            TuiAction::ConsensusDryRun {
                prompt: "should this write?".to_string(),
            },
        );

        let message = app.messages.last().expect("policy message");
        assert_eq!(message.kind, ChatMessageKind::PolicyDecision);
        assert_eq!(message.storage_mode, TuiStorageMode::DryRunWritesAllowed);
        assert!(message.body.contains("Blocked in inspect mode"));
        assert!(!cost_ledger::cost_ledger_path(&cfg).exists());
        assert!(!cfg.runtime.session_dir.exists());
    }

    #[test]
    fn compact_layout_threshold_is_explicit() {
        assert_eq!(chat_layout_mode(95), ChatLayoutMode::Compact);
        assert_eq!(chat_layout_mode(96), ChatLayoutMode::Wide);
    }

    #[test]
    fn rendered_chat_messages_always_include_provenance() {
        let message = ChatMessage {
            kind: ChatMessageKind::StateRecord,
            storage_mode: TuiStorageMode::InspectOnly,
            body: "State review".to_string(),
        };
        let rendered = render_chat_message_text(&message);

        assert!(rendered.contains("StateRecord"));
        assert!(rendered.contains("InspectOnly"));
        assert!(rendered.contains("State review"));
    }

    #[test]
    fn ask_inspect_never_calls_provider_or_writes() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        cfg.llm.enabled = true;
        cfg.llm.api_key = Some("test-key".to_string());
        cfg.runtime.external_network_enabled = true;
        let mut app = test_chat_app(true);

        handle_tui_action(
            &cfg,
            &mut app,
            TuiAction::AskInspect {
                question: "summarize evidence".to_string(),
            },
        );

        let message = app.messages.last().expect("ask response");
        assert_eq!(message.kind, ChatMessageKind::DisplayOnlyNote);
        assert_eq!(message.storage_mode, TuiStorageMode::InspectOnly);
        assert!(message.body.contains("No provider call was made"));
        assert!(!cost_ledger::cost_ledger_path(&cfg).exists());
        assert!(!cfg.runtime.session_dir.exists());
    }
}
