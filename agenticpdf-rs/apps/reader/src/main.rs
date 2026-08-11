// SPDX-License-Identifier: AGPL-3.0-or-later
//! An agentic-first document reader and editor.
//!
//! Reads every format the engine knows and edits its own — see
//! `agenticpdf::adf`. Built on Dewey, whose ontology is the point: every
//! control registers a schema, a semantic role and an `agent_id`, so an agent
//! discovers what the app can do by asking it rather than by being told
//! separately. The consequence to preserve is that **the agent and the user
//! drive the same code**: both go through [`session::Session`], so a capability
//! cannot exist for one and not the other.

use apdf_reader::{actions, canvas, ontology, session};

use std::cell::RefCell;

use dewey::prelude::*;
use dewey::widget::input::TextInputState;
use dewey::widget::list::ListState;

use session::{ACTOR_USER, Hit, Session};

/// Height of one outline row, matching what the list widget draws.
const OUTLINE_ROW_HEIGHT: f32 = 24.0;

/// What the user can be looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    /// The typeset page.
    Page,
    /// The document as editable blocks.
    Outline,
    /// Search results.
    Search,
}

struct App {
    session: Option<Session>,
    status: String,
    pane: Pane,
    query: String,
    hits: Vec<Hit>,
    /// Widget state Dewey keeps on our behalf. `view` takes `&self`, so it
    /// lives behind `RefCell` — the pattern Dewey's own examples use.
    outline_state: RefCell<ListState>,
    results_state: RefCell<ListState>,
    input_state: RefCell<TextInputState>,
    /// Where each toolbar button was last drawn.
    ///
    /// Dewey's widgets paint and register themselves but do not dispatch, so a
    /// click is matched against the rectangles `view` recorded. Keeping them
    /// keyed by message means adding a button cannot leave it inert — the same
    /// entry that positions it is the one that makes it work.
    buttons: RefCell<Vec<(Rect, Msg)>>,
    /// Where the outline list was last drawn, so a click maps to a row.
    outline_rect: RefCell<Rect>,
    /// Index of the selected block in the outline, if any.
    selected: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Open(String),
    NextPage,
    PreviousPage,
    ZoomIn,
    ZoomOut,
    ShowPane(Pane),
    SetQuery(String),
    RunSearch,
    SelectBlock(usize),
    DeleteSelected,
    Save,
    Export(String),
}

impl App {
    fn new() -> App {
        App {
            session: None,
            status: "No document open. Pass a path on the command line.".into(),
            pane: Pane::Page,
            query: String::new(),
            hits: Vec::new(),
            outline_state: RefCell::new(ListState::new()),
            results_state: RefCell::new(ListState::new()),
            input_state: RefCell::new(TextInputState::new()),
            buttons: RefCell::new(Vec::new()),
            outline_rect: RefCell::new(Rect::ZERO),
            selected: None,
        }
    }

    fn open_path(&mut self, path: &str) {
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| Session::open(bytes).map_err(|e| e.to_string()))
        {
            Ok(session) => {
                self.status = format!(
                    "{} — {}, {} page(s)",
                    session.title(),
                    session.format().label(),
                    session.page_count()
                );
                self.session = Some(session);
                self.selected = None;
            }
            // A failure to open is shown, never swallowed: the user picked this
            // file and is owed the reason it did not load.
            Err(why) => self.status = format!("Could not open {path}: {why}"),
        }
    }
}

impl Model for App {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Open(path) => self.open_path(&path),
            Msg::ShowPane(pane) => self.pane = pane,
            Msg::SetQuery(query) => self.query = query,
            Msg::NextPage => {
                if let Some(session) = &mut self.session {
                    session.next_page();
                }
            }
            Msg::PreviousPage => {
                if let Some(session) = &mut self.session {
                    session.previous_page();
                }
            }
            Msg::ZoomIn => {
                if let Some(session) = &mut self.session {
                    let zoom = session.zoom();
                    session.set_zoom(zoom * 1.25);
                }
            }
            Msg::ZoomOut => {
                if let Some(session) = &mut self.session {
                    let zoom = session.zoom();
                    session.set_zoom(zoom / 1.25);
                }
            }
            Msg::RunSearch => {
                if let Some(session) = &self.session {
                    self.hits = session.search(&self.query);
                    self.status = format!("{} result(s) for \"{}\"", self.hits.len(), self.query);
                    self.pane = Pane::Search;
                }
            }
            Msg::SelectBlock(index) => self.selected = Some(index),
            Msg::DeleteSelected => {
                if let (Some(session), Some(index)) = (&mut self.session, self.selected)
                    && let Some(id) = session.blocks().get(index).map(|(id, _)| *id)
                {
                    session.delete_block(ACTOR_USER, id);
                    self.status = "Deleted a block.".into();
                    self.selected = None;
                }
            }
            Msg::Save => {
                if let Some(session) = &mut self.session {
                    self.status = match session.save() {
                        Ok(bytes) => match std::fs::write("document.adf", &bytes) {
                            Ok(()) => format!("Saved {} bytes to document.adf", bytes.len()),
                            Err(why) => format!("Could not write document.adf: {why}"),
                        },
                        Err(why) => format!("Could not save: {why}"),
                    };
                }
            }
            Msg::Export(to) => {
                if let Some(session) = &self.session {
                    self.status = match session.export(&to) {
                        Ok(text) => format!("Exported {} characters of {to}", text.len()),
                        Err(why) => format!("Could not export: {why}"),
                    };
                }
            }
        }
        Command::None
    }

    fn view(&self, frame: &mut Frame<'_>) {
        let rows = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(34.0), // toolbar
                Constraint::Fill(1.0),    // content
                Constraint::Length(24.0), // status
            ],
        )
        .split(frame.area);

        self.view_toolbar(rows[0], frame);
        match self.pane {
            Pane::Page => self.view_page(rows[1], frame),
            Pane::Outline => self.view_outline(rows[1], frame),
            Pane::Search => self.view_search(rows[1], frame),
        }
        Label::new(&self.status)
            .agent_id("status")
            .render(rows[2], frame);
    }

    fn handle_event(&self, event: Event) -> Option<Msg> {
        match event {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Right | KeyCode::PageDown => Some(Msg::NextPage),
                KeyCode::Left | KeyCode::PageUp => Some(Msg::PreviousPage),
                KeyCode::Char('+') | KeyCode::Char('=') => Some(Msg::ZoomIn),
                KeyCode::Char('-') => Some(Msg::ZoomOut),
                KeyCode::Char('/') => Some(Msg::ShowPane(Pane::Search)),
                KeyCode::Char('o') => Some(Msg::ShowPane(Pane::Outline)),
                KeyCode::Char('p') => Some(Msg::ShowPane(Pane::Page)),
                KeyCode::Char('s') => Some(Msg::Save),
                _ => None,
            },
            Event::Mouse(ref mouse) if mouse.is_click() => {
                let hit = self
                    .buttons
                    .borrow()
                    .iter()
                    .find(|(rect, _)| rect.contains(mouse.position))
                    .map(|(_, msg)| msg.clone());
                if hit.is_some() {
                    return hit;
                }
                self.input_state.borrow_mut().focused = false;

                // A click in the outline selects the row under it. Row height
                // is the list widget's, so this stays in step with what is
                // actually drawn rather than assuming a constant.
                let outline = *self.outline_rect.borrow();
                if self.pane == Pane::Outline && outline.contains(mouse.position) {
                    let row = ((mouse.position.y - outline.y) / OUTLINE_ROW_HEIGHT) as usize;
                    return Some(Msg::SelectBlock(row));
                }
                None
            }
            // Dropping a file onto the window opens it — the most direct way
            // in, and the one a user tries first.
            Event::FileDrop(paths) => paths.first().map(|path| Msg::Open(path.clone())),
            Event::TextInput(text) => Some(Msg::SetQuery(text)),
            _ => None,
        }
    }

    fn register_ontology(&self, registry: &mut OntologyRegistry) {
        ontology::register(registry);
    }

    fn execute_action(
        &mut self,
        _agent_id: &str,
        action: &str,
        params: &serde_json::Value,
    ) -> serde_json::Value {
        // Straight into the same session the buttons drive — see `actions`.
        let result = actions::execute(&mut self.session, action, params);
        self.status = format!("Agent: {action}");
        result
    }

    fn title(&self) -> &str {
        "AgenticPDF Reader"
    }
}

impl App {
    /// Draw a button and record where it landed, so clicks reach `update`.
    fn button(&self, label: &str, agent_id: &str, msg: Msg, area: Rect, frame: &mut Frame<'_>) {
        Button::new(label).agent_id(agent_id).render(area, frame);
        self.buttons.borrow_mut().push((area, msg));
    }

    fn view_toolbar(&self, area: Rect, frame: &mut Frame<'_>) {
        // Rebuilt every frame, so a button that moves takes its hit area with
        // it rather than leaving a stale rectangle behind.
        self.buttons.borrow_mut().clear();

        let columns = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(90.0),
                Constraint::Length(90.0),
                Constraint::Length(90.0),
                Constraint::Length(70.0),
                Constraint::Length(70.0),
                Constraint::Fill(1.0),
                Constraint::Length(80.0),
            ],
        )
        .split(area);

        self.button(
            "Page [p]",
            "show_page",
            Msg::ShowPane(Pane::Page),
            columns[0],
            frame,
        );
        self.button(
            "Outline [o]",
            "show_outline",
            Msg::ShowPane(Pane::Outline),
            columns[1],
            frame,
        );
        self.button(
            "Search [/]",
            "show_search",
            Msg::RunSearch,
            columns[2],
            frame,
        );
        self.button(
            "Prev",
            "previous_page",
            Msg::PreviousPage,
            columns[3],
            frame,
        );
        self.button("Next", "next_page", Msg::NextPage, columns[4], frame);

        let position = match &self.session {
            Some(session) => format!(
                "  {} / {}   {:.0}%{}",
                session.page(),
                session.page_count(),
                session.zoom() * 100.0,
                if session.is_dirty() {
                    "   (edited)"
                } else {
                    ""
                }
            ),
            None => String::new(),
        };
        Label::new(position)
            .agent_id("page_position")
            .render(columns[5], frame);

        self.button("Save [s]", "save", Msg::Save, columns[6], frame);
    }

    /// The outline's own controls, shown only where they apply.
    fn view_outline_actions(&self, area: Rect, frame: &mut Frame<'_>) {
        let columns = Layout::new(
            Direction::Horizontal,
            [
                Constraint::Length(110.0),
                Constraint::Length(110.0),
                Constraint::Fill(1.0),
            ],
        )
        .split(area);

        self.button(
            "Delete block",
            "delete_block",
            Msg::DeleteSelected,
            columns[0],
            frame,
        );
        self.button(
            "Export .md",
            "export_markdown",
            Msg::Export("markdown".into()),
            columns[1],
            frame,
        );

        let selection = match self.selected {
            Some(index) => format!("  block {index} selected"),
            None => "  select a block to edit it".to_string(),
        };
        Label::new(selection)
            .agent_id("outline_selection")
            .render(columns[2], frame);
    }

    fn view_page(&self, area: Rect, frame: &mut Frame<'_>) {
        let Some(session) = &self.session else {
            Label::new("Open a document to begin.")
                .agent_id("empty_state")
                .render(area, frame);
            return;
        };

        match session.display_list() {
            Ok(list) => {
                let transform = canvas::Transform::fit(&list, area, session.zoom());
                // Registered before painting so the page is addressable by an
                // agent even while its pixels are still being produced.
                frame.register_widget(
                    UiNode::new("DocumentCanvas", SemanticRole::Display)
                        .with_id("page_canvas")
                        .with_bounds(area.into())
                        .with_property("page", serde_json::json!(session.page()))
                        .with_property("ops", serde_json::json!(list.ops.len())),
                );
                canvas::paint_page(frame.painter(), &list, transform, &[]);
            }
            // Formats with no geometry are not a failure — the crate's standing
            // rule is to say so rather than invent coordinates.
            Err(why) => Label::new(format!("This page has no geometry: {why}"))
                .agent_id("page_unavailable")
                .render(area, frame),
        }
    }

    fn view_outline(&self, area: Rect, frame: &mut Frame<'_>) {
        let Some(session) = &self.session else {
            return;
        };
        let rows = Layout::new(
            Direction::Vertical,
            [Constraint::Length(30.0), Constraint::Fill(1.0)],
        )
        .split(area);
        self.view_outline_actions(rows[0], frame);
        let area = rows[1];

        // Recorded so a click can be turned into a selection, the same way the
        // toolbar buttons are.
        *self.outline_rect.borrow_mut() = area;

        let items: Vec<String> = session
            .blocks()
            .iter()
            .map(|(id, block)| {
                let mut text = String::new();
                agenticpdf::doc::block_text_into(block, &mut text);
                let text = text.trim();
                // Attribution in the outline, because "who wrote this" is the
                // question a document edited by agents raises constantly.
                match session.attribution(*id) {
                    Some((who, true, _)) => format!("[{who}] {}", truncate(text, 70)),
                    _ => truncate(text, 80),
                }
            })
            .collect();

        List::new(items).agent_id("outline").render(
            area,
            frame,
            &mut self.outline_state.borrow_mut(),
        );
    }

    fn view_search(&self, area: Rect, frame: &mut Frame<'_>) {
        let rows = Layout::new(
            Direction::Vertical,
            [Constraint::Length(30.0), Constraint::Fill(1.0)],
        )
        .split(area);

        TextInput::new().agent_id("search_query").render(
            rows[0],
            frame,
            &mut self.input_state.borrow_mut(),
        );

        let results: Vec<String> = self
            .hits
            .iter()
            .map(|hit| match hit.score {
                Some(score) => format!("{:.2}  {}", score, truncate(&hit.text, 70)),
                None => truncate(&hit.text, 80),
            })
            .collect();

        List::new(results).agent_id("search_results").render(
            rows[1],
            frame,
            &mut self.results_state.borrow_mut(),
        );
    }
}

/// Shorten to `limit` characters on a character boundary.
fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn main() -> std::result::Result<(), eframe::Error> {
    // Agent discovery without launching a window: an agent can ask what this
    // program does before deciding to drive it.
    if std::env::args().any(|arg| arg == "--capabilities") {
        println!("{:#}", ontology::capabilities());
        return Ok(());
    }

    let mut app = App::new();
    if let Some(path) = std::env::args().nth(1) {
        app.open_path(&path);
    }

    Program::new(app)
        .with_options(ProgramOptions {
            width: 1100.0,
            height: 800.0,
            ..Default::default()
        })
        .run()
}
