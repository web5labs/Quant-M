use crate::config::{ChannelPreference, Config, ModelPreference};
use crate::domain;
use crate::execution_runtime::{self, WorkflowRunResult};
use crate::sessions;
use crate::shared_state;
use crate::workflow_registry::{self, WorkflowId};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::path::Path;
use std::time::Duration;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_VISIBLE_ITEMS: usize = 8;
const MOCK_RESEARCH_WORKFLOW: &str = "workflow:mock-research-brief";

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
