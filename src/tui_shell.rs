use crate::agent_shell;
use crate::config::{ChannelPreference, Config, ModelPreference};
use crate::cost_ledger;
use crate::domain;
use crate::execution_runtime::{self, WorkflowRunResult};
use crate::llm;
use crate::sessions::{self, SessionId};
use crate::shared_state;
use crate::state_review;
use crate::workflow_registry::{self, WorkflowId};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_VISIBLE_ITEMS: usize = 8;
const MOCK_RESEARCH_WORKFLOW: &str = "workflow:mock-research-brief";
const CHAT_INPUT_HINT: &str = "/help /read /write /add-dir <path> /state [domain] /cost [session] /replay <session> /ask <question> /refresh /quit";
const CHAT_FOOTER_HINT: &str =
    "/help /read /write /state /cost /quit | Ctrl+Enter newline | Esc quit";

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
    SetCliSandbox { sandbox: agent_shell::CliSandbox },
    AddWriteDir { path: String },
    ShowWriteDirs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiStorageMode {
    InspectOnly,
    DryRunWritesAllowed,
    StateWritesAllowed,
    RequiresApproval,
    ReadOnlyToolCall,
    WorkspaceWriteToolCall,
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
    ToolCliResponse,
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

#[derive(Debug)]
struct ChatApp {
    inspect_only: bool,
    chat_tool: Option<String>,
    cli_sandbox: agent_shell::CliSandbox,
    project_root: PathBuf,
    add_dirs: Vec<PathBuf>,
    pending: Option<PendingChat>,
    messages: Vec<ChatMessage>,
    input: String,
    notice: String,
    snapshot: TuiSnapshot,
}

#[derive(Debug)]
struct PendingChat {
    tool_id: String,
    storage_mode: TuiStorageMode,
    started_at: Instant,
    receiver: Receiver<Result<String, String>>,
}

pub fn run_chat(cfg: &Config, config_path: &Path, inspect: bool) -> Result<()> {
    let chat_tool = (!inspect).then(|| selected_chat_route(cfg)).flatten();
    let inspect_only = inspect || chat_tool.is_none();
    let cli_sandbox = if chat_tool.as_deref() == Some("codex") {
        agent_shell::CliSandbox::WorkspaceWrite
    } else {
        agent_shell::CliSandbox::ReadOnly
    };
    let project_root = project_root_from_config_path(config_path)?;
    let mut app = ChatApp {
        inspect_only,
        chat_tool: chat_tool.clone(),
        cli_sandbox,
        project_root,
        add_dirs: Vec::new(),
        pending: None,
        messages: initial_chat_messages(inspect_only, chat_tool.as_deref(), cli_sandbox),
        input: String::new(),
        notice: initial_chat_notice(inspect_only, chat_tool.as_deref(), cli_sandbox),
        snapshot: collect_snapshot(cfg)?,
    };

    let mut terminal = ratatui::init();
    let result = run_chat_app(&mut terminal, cfg, &mut app);
    ratatui::restore();
    result
}

fn project_root_from_config_path(config_path: &Path) -> Result<PathBuf> {
    config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .context("failed to resolve Quant-M project root")
}

fn initial_chat_messages(
    inspect: bool,
    chat_tool: Option<&str>,
    sandbox: agent_shell::CliSandbox,
) -> Vec<ChatMessage> {
    vec![ChatMessage {
        kind: ChatMessageKind::DisplayOnlyNote,
        storage_mode: TuiStorageMode::InspectOnly,
        body: format!(
            "Route: {}. Permission scope: {} inside the current project only. Quant-M records evidence; it does not grant execution authority.",
            chat_mode_label(inspect, chat_tool),
            sandbox.label()
        ),
    }]
}

fn initial_chat_notice(
    inspect: bool,
    chat_tool: Option<&str>,
    sandbox: agent_shell::CliSandbox,
) -> String {
    if inspect {
        "Inspect-only cockpit. /ask records navigation text; no provider or CLI call is made."
            .to_string()
    } else {
        format!(
            "{} ready. {} is confined to the project root. Use /read for inspect-only file access.",
            chat_mode_label(false, chat_tool),
            sandbox.label()
        )
    }
}

fn chat_mode_label(inspect: bool, chat_tool: Option<&str>) -> String {
    if inspect {
        "inspect".to_string()
    } else {
        format!("{}-cli", chat_tool.unwrap_or("tool"))
    }
}

fn run_chat_app(terminal: &mut DefaultTerminal, cfg: &Config, app: &mut ChatApp) -> Result<()> {
    loop {
        drain_pending_chat(app);
        terminal.draw(|frame| render_chat(frame, app))?;
        let poll_delay = if app.pending.is_some() {
            Duration::from_millis(40)
        } else {
            Duration::from_millis(80)
        };
        if event::poll(poll_delay)? {
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
                        let action = parse_tui_action(&input);
                        let input_storage_mode = storage_mode_for_action(&action, app.cli_sandbox);
                        app.messages.push(ChatMessage {
                            kind: ChatMessageKind::HumanInput,
                            storage_mode: input_storage_mode,
                            body: input.clone(),
                        });
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
        "/read" | "/readonly" | "/read-only" => {
            return TuiAction::SetCliSandbox {
                sandbox: agent_shell::CliSandbox::ReadOnly,
            };
        }
        "/write" | "/read-write" | "/workspace-write" => {
            return TuiAction::SetCliSandbox {
                sandbox: agent_shell::CliSandbox::WorkspaceWrite,
            };
        }
        "/add-dir" | "/adddir" => {
            return TuiAction::AddWriteDir {
                path: rest.trim().to_string(),
            };
        }
        "/dirs" | "/write-dirs" => return TuiAction::ShowWriteDirs,
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

fn storage_mode_for_action(action: &TuiAction, sandbox: agent_shell::CliSandbox) -> TuiStorageMode {
    match action {
        TuiAction::Help
        | TuiAction::Quit
        | TuiAction::Refresh
        | TuiAction::SetCliSandbox { .. }
        | TuiAction::AddWriteDir { .. }
        | TuiAction::ShowWriteDirs
        | TuiAction::ShowState { .. }
        | TuiAction::ShowCost { .. }
        | TuiAction::Replay { .. } => TuiStorageMode::InspectOnly,
        TuiAction::AskInspect { .. } => match sandbox {
            agent_shell::CliSandbox::ReadOnly => TuiStorageMode::ReadOnlyToolCall,
            agent_shell::CliSandbox::WorkspaceWrite => TuiStorageMode::WorkspaceWriteToolCall,
        },
        TuiAction::ConsensusDryRun { .. } => TuiStorageMode::DryRunWritesAllowed,
    }
}

fn handle_tui_action(cfg: &Config, app: &mut ChatApp, action: TuiAction) {
    let storage_mode = storage_mode_for_action(&action, app.cli_sandbox);
    if app.inspect_only && storage_mode != TuiStorageMode::InspectOnly {
        if let TuiAction::AskInspect { question } = action {
            app.messages.push(ChatMessage {
                kind: ChatMessageKind::DisplayOnlyNote,
                storage_mode: TuiStorageMode::InspectOnly,
                body: format!(
                    "Inspect question recorded as display-only navigation text. No provider or CLI call was made.\nquestion: {question}\nnext: use /state, /cost, or /replay <session_id> to inspect structured truth."
                ),
            });
            return;
        }
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
                "Typed actions only. {CHAT_INPUT_HINT}. Chat text is evidence input, not runtime authority. In CLI mode, /ask and plain text call the selected tool through a bounded adapter.\npermissions: /read uses Codex read-only; /write uses Codex workspace-write; /add-dir <path> grants an extra writable directory for this chat session.\nstorage modes: {}\nmessage provenance: {}",
                known_storage_modes_label(),
                known_message_kinds_label()
            ),
        },
        TuiAction::SetCliSandbox { sandbox } => {
            if sandbox == agent_shell::CliSandbox::WorkspaceWrite
                && app.chat_tool.as_deref() != Some("codex")
            {
                ChatMessage {
                    kind: ChatMessageKind::PolicyDecision,
                    storage_mode,
                    body: "Workspace writes are available only through the hardened Codex adapter. This route remains read-only.".to_string(),
                }
            } else {
                app.cli_sandbox = sandbox;
                ChatMessage {
                    kind: ChatMessageKind::PolicyDecision,
                    storage_mode,
                    body: match sandbox {
                        agent_shell::CliSandbox::ReadOnly => {
                            "Codex chat sandbox set to read-only. Codex can inspect and answer, but not create or edit files.".to_string()
                        }
                        agent_shell::CliSandbox::WorkspaceWrite => {
                            "Codex chat sandbox set to workspace-write. Codex may create and edit files inside this Quant-M project only.".to_string()
                        }
                    },
                }
            }
        }
        TuiAction::AddWriteDir { path } => match resolve_add_dir_path(&path, &app.project_root) {
            Ok(path) => {
                if !app.add_dirs.contains(&path) {
                    app.add_dirs.push(path.clone());
                }
                ChatMessage {
                    kind: ChatMessageKind::PolicyDecision,
                    storage_mode,
                    body: format!("Added project-local writable directory: {}", path.display()),
                }
            }
            Err(err) => ChatMessage {
                kind: ChatMessageKind::Error,
                storage_mode,
                body: err,
            },
        },
        TuiAction::ShowWriteDirs => ChatMessage {
            kind: ChatMessageKind::DisplayOnlyNote,
            storage_mode,
            body: format!(
                "Codex sandbox: {}\nExtra writable directories: {}",
                app.cli_sandbox.label(),
                format_chat_add_dirs(&app.add_dirs)
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
        TuiAction::AskInspect { question } => {
            if app.chat_tool.is_none() {
                ChatMessage {
                    kind: ChatMessageKind::DisplayOnlyNote,
                    storage_mode,
                    body: format!(
                        "No chat-capable CLI is enabled in this Quant-M profile, so no provider or CLI call was made.\nquestion: {question}\nnext: run `quant-m onboard`, select Codex, Claude, Gemini, or Antigravity, then validate with `quant-m tool validate <tool>`."
                    ),
                }
            } else if let Some(pending) = &app.pending {
                ChatMessage {
                    kind: ChatMessageKind::PolicyDecision,
                    storage_mode,
                    body: format!(
                        "{} is still working for {}. Wait for the response before sending another tool call.",
                        pending.tool_id,
                        format_elapsed(pending.started_at.elapsed())
                    ),
                }
            } else {
                let tool_id = app.chat_tool.as_deref().unwrap_or("codex");
                app.pending = Some(spawn_chat_call(cfg, app, tool_id, &question, storage_mode));
                ChatMessage {
                    kind: ChatMessageKind::DisplayOnlyNote,
                    storage_mode,
                    body: format!(
                        "Started {tool_id} chat call in the background. The TUI stays responsive while Quant-M waits for the tool response."
                    ),
                }
            }
        }
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

fn run_model_chat(cfg: &Config, prompt: &str) -> Result<String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create model chat runtime")?
        .block_on(llm::ask(cfg, prompt))
}

fn spawn_chat_call(
    cfg: &Config,
    app: &ChatApp,
    tool_id: &str,
    question: &str,
    storage_mode: TuiStorageMode,
) -> PendingChat {
    let (sender, receiver) = mpsc::channel();
    let cfg = cfg.clone();
    let tool_id = tool_id.to_string();
    let question = question.to_string();
    let options = agent_shell::CliChatOptions {
        sandbox: app.cli_sandbox,
        add_dirs: app.add_dirs.clone(),
        project_root: Some(app.project_root.clone()),
    };
    let worker_tool_id = tool_id.clone();
    std::thread::spawn(move || {
        let result = if worker_tool_id.starts_with("model:") {
            run_model_chat(&cfg, &question)
        } else {
            agent_shell::run_cli_chat_with_options(&cfg, &worker_tool_id, &question, &options)
        }
        .map_err(|err| err.to_string());
        let _ = sender.send(result);
    });
    PendingChat {
        tool_id,
        storage_mode,
        started_at: Instant::now(),
        receiver,
    }
}

fn drain_pending_chat(app: &mut ChatApp) {
    let Some(pending) = app.pending.as_ref() else {
        return;
    };
    match pending.receiver.try_recv() {
        Ok(result) => {
            let pending = app.pending.take().expect("pending chat");
            let elapsed = format_elapsed(pending.started_at.elapsed());
            match result {
                Ok(output) => {
                    app.notice = format!("{} completed in {elapsed}", pending.tool_id);
                    app.messages.push(ChatMessage {
                        kind: ChatMessageKind::ToolCliResponse,
                        storage_mode: pending.storage_mode,
                        body: output,
                    });
                }
                Err(err) => {
                    app.notice = format!("{} failed after {elapsed}", pending.tool_id);
                    app.messages.push(ChatMessage {
                        kind: ChatMessageKind::Error,
                        storage_mode: pending.storage_mode,
                        body: format!("{} CLI call failed: {err}", pending.tool_id),
                    });
                }
            }
        }
        Err(TryRecvError::Empty) => {
            app.notice = format!(
                "{} working... {} elapsed",
                pending.tool_id,
                format_elapsed(pending.started_at.elapsed())
            );
        }
        Err(TryRecvError::Disconnected) => {
            let pending = app.pending.take().expect("pending chat");
            app.notice = format!("{} worker disconnected", pending.tool_id);
            app.messages.push(ChatMessage {
                kind: ChatMessageKind::Error,
                storage_mode: pending.storage_mode,
                body: format!("{} CLI call failed: worker disconnected", pending.tool_id),
            });
        }
    }
}

fn format_elapsed(duration: Duration) -> String {
    if duration.as_secs() == 0 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{}s", duration.as_secs())
    }
}

fn known_storage_modes_label() -> String {
    [
        TuiStorageMode::InspectOnly,
        TuiStorageMode::DryRunWritesAllowed,
        TuiStorageMode::StateWritesAllowed,
        TuiStorageMode::RequiresApproval,
        TuiStorageMode::ReadOnlyToolCall,
        TuiStorageMode::WorkspaceWriteToolCall,
    ]
    .into_iter()
    .map(|mode| format!("{mode:?}"))
    .collect::<Vec<_>>()
    .join(", ")
}

fn resolve_add_dir_path(raw: &str, project_root: &Path) -> std::result::Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("Usage: /add-dir <existing-directory>".to_string());
    }
    let expanded = if raw == "~" {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "Could not resolve ~ because HOME is not set.".to_string())?
    } else if let Some(rest) = raw.strip_prefix("~/") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "Could not resolve ~/ because HOME is not set.".to_string())?
            .join(rest)
    } else {
        PathBuf::from(raw)
    };
    let path = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .map_err(|err| format!("Could not resolve current directory: {err}"))?
            .join(expanded)
    };
    if !path.is_dir() {
        return Err(format!(
            "Writable add-dir must already exist and be a directory: {}",
            path.display()
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|err| format!("Could not canonicalize {}: {err}", path.display()))?;
    let root = project_root
        .canonicalize()
        .map_err(|err| format!("Could not canonicalize project root: {err}"))?;
    if !canonical.starts_with(&root) {
        return Err(format!(
            "Writable paths must stay inside the project root: {}",
            root.display()
        ));
    }
    Ok(canonical)
}

fn format_chat_add_dirs(add_dirs: &[PathBuf]) -> String {
    if add_dirs.is_empty() {
        "none".to_string()
    } else {
        add_dirs
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
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
        ChatMessageKind::ToolCliResponse,
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
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(6),
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
        "inspect".to_string()
    } else {
        chat_mode_label(false, app.chat_tool.as_deref())
    };
    let route = app.chat_tool.as_deref().unwrap_or("inspect");
    let status_style = if app.inspect_only {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    };
    let text = Text::from(vec![
        Line::from(vec![
            Span::styled(
                "QUANT-M  ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if app.inspect_only { "INSPECT" } else { "READY" },
                status_style,
            ),
            Span::raw(format!(
                "   route={route}   scope={}   {}",
                app.cli_sandbox.label(),
                if compact { "compact" } else { "wide" }
            )),
        ]),
        Line::from(format!(
            "project={}   sessions={}   evidence={}",
            app.project_root.display(),
            app.snapshot.session_count,
            app.snapshot.shared_state_count
        ))
        .style(Style::default().fg(Color::Gray)),
    ]);
    let title = format!(" Chat / {mode} ");
    frame.render_widget(
        Paragraph::new(text)
            .block(panel_block(&title, Color::Cyan))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_chat_messages(frame: &mut Frame, area: Rect, app: &ChatApp, compact: bool) {
    let max_lines = area.height.saturating_sub(2).max(1) as usize;
    let mut lines = Vec::new();
    for message in &app.messages {
        let style = chat_message_style(message.kind);
        let alignment = if compact || message.kind != ChatMessageKind::HumanInput {
            Alignment::Left
        } else {
            Alignment::Right
        };
        for rendered in render_chat_message_lines(message) {
            lines.push(Line::from(Span::styled(rendered, style)).alignment(alignment));
        }
    }
    if lines.len() > max_lines {
        lines = lines.split_off(lines.len() - max_lines);
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(panel_block(" Conversation ", Color::Magenta))
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

#[cfg(test)]
fn render_chat_message_text(message: &ChatMessage) -> String {
    render_chat_message_lines(message).join("\n")
}

fn render_chat_message_lines(message: &ChatMessage) -> Vec<String> {
    let prefix = chat_message_prefix(message.kind);
    let mut body_lines = message.body.lines();
    let first = body_lines.next().unwrap_or("");
    let mut rendered = vec![format!(
        "{prefix} [{:?}/{:?}] {first}",
        message.kind, message.storage_mode
    )];
    rendered.extend(body_lines.map(|line| format!("    {line}")));
    rendered
}

fn render_chat_evidence_rail(frame: &mut Frame, area: Rect, app: &ChatApp) {
    let last = app.messages.last();
    let items = vec![
        ListItem::new(format!(
            "route: {}",
            app.chat_tool.as_deref().unwrap_or("inspect")
        )),
        ListItem::new(format!("project: {}", app.project_root.display())),
        ListItem::new(format!(
            "network: {}",
            app.snapshot.external_network_posture
        )),
        ListItem::new(format!("sandbox: {}", app.cli_sandbox.label())),
        ListItem::new(format!(
            "project dirs: {}",
            format_chat_add_dirs(&app.add_dirs)
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
        List::new(items).block(panel_block(" Route & Evidence ", Color::Green)),
        area,
    );
}

fn render_chat_input(frame: &mut Frame, area: Rect, app: &ChatApp) {
    let text = Text::from(vec![
        Line::from(app.input.clone()),
        Line::from(Span::styled(
            CHAT_FOOTER_HINT,
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
            .block(panel_block(" Message ", Color::LightBlue))
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
        ChatMessageKind::ToolCliResponse => "Tool:",
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
        ChatMessageKind::ToolCliResponse => Style::default().fg(Color::Cyan),
        ChatMessageKind::Error => Style::default().fg(Color::Red),
    }
}

pub(crate) fn selected_chat_tool(cfg: &Config) -> Option<String> {
    if let Some(preferred) = cfg
        .preferences
        .preferred_chat_tool
        .as_deref()
        .and_then(|tool| enabled_chat_tool_id(cfg, tool))
    {
        return Some(preferred);
    }

    [
        "codex",
        "claude",
        "anthropic",
        "gemini",
        "antigravity",
        "openai",
        "opencode",
    ]
    .into_iter()
    .find_map(|id| enabled_chat_tool_id(cfg, id))
}

pub(crate) fn selected_chat_route(cfg: &Config) -> Option<String> {
    selected_chat_tool(cfg).or_else(|| {
        let ready = cfg.llm.enabled
            && cfg.runtime.external_network_enabled
            && cfg.resolve_llm_api_key().is_some()
            && !cfg.llm.model.trim().is_empty();
        ready.then(|| format!("model:{}", cfg.llm.model))
    })
}

fn enabled_chat_tool_id(cfg: &Config, id: &str) -> Option<String> {
    let id = id.trim().to_ascii_lowercase();
    if agent_shell::chat_tool_readiness(cfg, &id).is_ready() {
        Some(id)
    } else {
        None
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
            chat_tool: None,
            cli_sandbox: agent_shell::CliSandbox::ReadOnly,
            project_root: std::env::current_dir().expect("current dir"),
            add_dirs: Vec::new(),
            pending: None,
            messages: Vec::new(),
            input: String::new(),
            notice: String::new(),
            snapshot: test_snapshot(),
        }
    }

    fn test_config(tmp: &TempDir) -> Config {
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
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
        assert_eq!(
            parse_tui_action("/write"),
            TuiAction::SetCliSandbox {
                sandbox: agent_shell::CliSandbox::WorkspaceWrite
            }
        );
        assert_eq!(
            parse_tui_action("/read"),
            TuiAction::SetCliSandbox {
                sandbox: agent_shell::CliSandbox::ReadOnly
            }
        );
        assert_eq!(
            parse_tui_action("/add-dir ~/Desktop"),
            TuiAction::AddWriteDir {
                path: "~/Desktop".to_string()
            }
        );
        assert_eq!(parse_tui_action("/dirs"), TuiAction::ShowWriteDirs);
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
            storage_mode_for_action(
                &TuiAction::ShowState { domain: None },
                agent_shell::CliSandbox::ReadOnly,
            ),
            TuiStorageMode::InspectOnly
        );
        assert_eq!(
            storage_mode_for_action(
                &TuiAction::ConsensusDryRun {
                    prompt: "test".to_string()
                },
                agent_shell::CliSandbox::ReadOnly,
            ),
            TuiStorageMode::DryRunWritesAllowed
        );
        assert_eq!(
            storage_mode_for_action(
                &TuiAction::AskInspect {
                    question: "what model are you?".to_string()
                },
                agent_shell::CliSandbox::ReadOnly,
            ),
            TuiStorageMode::ReadOnlyToolCall
        );
        assert_eq!(
            storage_mode_for_action(
                &TuiAction::AskInspect {
                    question: "create a file".to_string()
                },
                agent_shell::CliSandbox::WorkspaceWrite,
            ),
            TuiStorageMode::WorkspaceWriteToolCall
        );
        assert!(known_storage_modes_label().contains("RequiresApproval"));
        assert!(known_storage_modes_label().contains("ReadOnlyToolCall"));
        assert!(known_storage_modes_label().contains("WorkspaceWriteToolCall"));
        assert!(known_message_kinds_label().contains("WorkerProposal"));
        assert!(known_message_kinds_label().contains("ToolCliResponse"));
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
    fn write_command_switches_chat_to_workspace_write() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let mut app = test_chat_app(false);
        app.chat_tool = Some("codex".to_string());

        handle_tui_action(
            &cfg,
            &mut app,
            TuiAction::SetCliSandbox {
                sandbox: agent_shell::CliSandbox::WorkspaceWrite,
            },
        );

        assert_eq!(app.cli_sandbox, agent_shell::CliSandbox::WorkspaceWrite);
        let message = app.messages.last().expect("mode message");
        assert_eq!(message.kind, ChatMessageKind::PolicyDecision);
        assert!(message.body.contains("workspace-write"));
    }

    #[test]
    fn add_dir_command_registers_canonical_write_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let mut app = test_chat_app(false);
        app.project_root = tmp.path().canonicalize().expect("project root");
        let extra = tmp.path().join("extra");
        std::fs::create_dir_all(&extra).expect("extra dir");

        handle_tui_action(
            &cfg,
            &mut app,
            TuiAction::AddWriteDir {
                path: extra.display().to_string(),
            },
        );

        assert_eq!(app.add_dirs, vec![extra.canonicalize().expect("canon")]);
        let message = app.messages.last().expect("add-dir message");
        assert_eq!(message.kind, ChatMessageKind::PolicyDecision);
        assert!(
            message
                .body
                .contains("Added project-local writable directory")
        );
    }

    #[test]
    fn compact_layout_threshold_is_explicit() {
        assert_eq!(chat_layout_mode(95), ChatLayoutMode::Compact);
        assert_eq!(chat_layout_mode(96), ChatLayoutMode::Wide);
    }

    #[test]
    fn relative_config_path_resolves_current_project_root() {
        let expected = std::env::current_dir()
            .expect("current dir")
            .canonicalize()
            .expect("canonical current dir");
        let root =
            project_root_from_config_path(Path::new("quant-m.local.toml")).expect("project root");
        assert_eq!(root, expected);
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
    fn rendered_chat_messages_preserve_multiline_tool_answers() {
        let message = ChatMessage {
            kind: ChatMessageKind::ToolCliResponse,
            storage_mode: TuiStorageMode::ReadOnlyToolCall,
            body: "Codex\nActual useful answer".to_string(),
        };
        let rendered = render_chat_message_text(&message);

        assert!(rendered.contains("Codex"));
        assert!(rendered.contains("Actual useful answer"));
    }

    #[test]
    fn ask_inspect_mode_never_calls_provider_cli_or_writes() {
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
        assert!(message.body.contains("No provider or CLI call was made"));
        assert!(!cost_ledger::cost_ledger_path(&cfg).exists());
        assert!(!cfg.runtime.session_dir.exists());
    }

    #[test]
    fn ask_cli_mode_reports_missing_chat_tool_without_calling_cli() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let mut app = test_chat_app(false);

        handle_tui_action(
            &cfg,
            &mut app,
            TuiAction::AskInspect {
                question: "what model are you?".to_string(),
            },
        );

        let message = app.messages.last().expect("ask response");
        assert_eq!(message.kind, ChatMessageKind::DisplayOnlyNote);
        assert_eq!(message.storage_mode, TuiStorageMode::ReadOnlyToolCall);
        assert!(message.body.contains("No chat-capable CLI is enabled"));
        assert!(!cost_ledger::cost_ledger_path(&cfg).exists());
        assert!(!cfg.runtime.session_dir.exists());
    }

    #[test]
    fn pending_chat_completion_drains_without_blocking_input_state() {
        let mut app = test_chat_app(false);
        let (sender, receiver) = mpsc::channel();
        app.input = "still editable".to_string();
        app.pending = Some(PendingChat {
            tool_id: "codex".to_string(),
            storage_mode: TuiStorageMode::WorkspaceWriteToolCall,
            started_at: Instant::now(),
            receiver,
        });
        sender.send(Ok("done".to_string())).expect("send response");

        drain_pending_chat(&mut app);

        assert!(app.pending.is_none());
        assert_eq!(app.input, "still editable");
        let message = app.messages.last().expect("response");
        assert_eq!(message.kind, ChatMessageKind::ToolCliResponse);
        assert_eq!(message.storage_mode, TuiStorageMode::WorkspaceWriteToolCall);
        assert_eq!(message.body, "done");
        assert!(app.notice.contains("completed"));
    }

    #[test]
    fn selected_chat_tool_follows_enabled_fallback_priority() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);

        assert!(selected_chat_tool(&cfg).is_none());
        let claude = cfg.tools.get_mut("claude").expect("claude tool");
        claude.enabled = true;
        claude.command = "sh".to_string();
        assert_eq!(selected_chat_tool(&cfg).as_deref(), Some("claude"));
        let codex = cfg.tools.get_mut("codex").expect("codex tool");
        codex.enabled = true;
        codex.command = "true".to_string();
        assert_eq!(selected_chat_tool(&cfg).as_deref(), Some("codex"));
    }

    #[test]
    fn selected_chat_tool_honors_preferred_chat_tool_over_codex() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        let codex = cfg.tools.get_mut("codex").expect("codex tool");
        codex.enabled = true;
        codex.command = "true".to_string();
        let claude = cfg.tools.get_mut("claude").expect("claude tool");
        claude.enabled = true;
        claude.command = "sh".to_string();
        cfg.preferences.preferred_chat_tool = Some("claude".to_string());

        assert_eq!(selected_chat_tool(&cfg).as_deref(), Some("claude"));
    }

    #[test]
    fn selected_chat_tool_rejects_enabled_but_missing_or_unsupported_routes() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        let codex = cfg.tools.get_mut("codex").expect("codex tool");
        codex.enabled = true;
        codex.command = "definitely-not-installed-quantm-test".to_string();
        assert!(selected_chat_tool(&cfg).is_none());

        let openai = cfg.tools.get_mut("openai").expect("openai tool");
        openai.enabled = true;
        openai.command = "sh".to_string();
        assert!(selected_chat_tool(&cfg).is_none());
    }

    #[test]
    fn selected_model_route_opens_chat_when_governed_provider_is_ready() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        cfg.llm.enabled = true;
        cfg.llm.api_key = Some("test-key".to_string());
        cfg.llm.model = "test/model".to_string();
        cfg.runtime.external_network_enabled = true;

        assert_eq!(
            selected_chat_route(&cfg).as_deref(),
            Some("model:test/model")
        );
    }

    #[test]
    fn add_dir_rejects_paths_outside_project_root() {
        let project = TempDir::new().expect("project");
        let outside = TempDir::new().expect("outside");
        let error = resolve_add_dir_path(outside.path().to_str().expect("path"), project.path())
            .expect_err("outside path must fail");
        assert!(error.contains("inside the project root"));
    }
}
