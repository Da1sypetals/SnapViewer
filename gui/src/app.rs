use std::sync::{Arc, Mutex};

use chrono::Local;
use iced::keyboard::{self, key};
use iced::widget::{
    Id, button, column, container, operation, pick_list, row, rule, scrollable, space, text,
    text_input,
};
use iced::{Element, Fill, Subscription, Task, Theme};
use ipc_channel::ipc::{IpcReceiver, IpcSender};

use crate::font::JETBRAINS_MONO;
use crate::ipc_worker;
use crate::palette::PaletteName;

// ── help / schema strings ─────────────────────────────────────────────────────

const HELP_MSG: &str = "Execute any SQLite commands.\n\
Special commands:\n\
    --help: display this help message\n\
    --schema: display database schema of the memory snapshot\n\
    --clear: clear REPL output\n\
    --find <pattern>: find the message panel (on the left) with a pattern.\n\
                      case INsensitive, does NOT support regex\n";

const DATABASE_SCHEMA: &str = "CREATE TABLE allocs (\n\
    idx INTEGER PRIMARY KEY,\n\
    size INTEGER,\n\
    start_time INTEGER,\n\
    end_time INTEGER,\n\
    callstack TEXT\n\
);";

const REPL_HINT: &[&str] = &[
    "SQLite REPL - This is a SQLite database storing the allocation data.",
    "Type `--help` to see available commands.",
    "Type `--find <pattern>` to search messages.",
    "Ctrl+D to quit application.",
];

const REPL_OUTPUT_ID: &str = "repl_output";

// ── state ─────────────────────────────────────────────────────────────────────

pub struct SnapViewerApp {
    pub sql_tx: IpcSender<String>,
    pub reply_rx: Arc<Mutex<IpcReceiver<String>>>,
    pub event_rx: Arc<Mutex<IpcReceiver<String>>>,
    pub palette_name: PaletteName,

    message_text: String,

    repl_lines: Vec<String>,
    repl_text: String, // cached join of repl_lines for view()
    repl_input: String,
    command_history: Vec<String>,
    history_index: usize,

    repl_visible: bool,
    sql_pending: bool,
}

// ── messages ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    RendererEvent(String),

    ReplInputChanged(String),
    ReplSubmit,
    SqlResult(String),

    ToggleRepl,
    ThemeChanged(PaletteName),
    Quit,

    KeyboardEvent(keyboard::Event),
}

// ── impl ──────────────────────────────────────────────────────────────────────

impl SnapViewerApp {
    pub fn new(
        sql_tx: IpcSender<String>,
        reply_rx: Arc<Mutex<IpcReceiver<String>>>,
        event_rx: Arc<Mutex<IpcReceiver<String>>>,
        palette_name: PaletteName,
    ) -> (Self, Task<Message>) {
        let app = Self {
            sql_tx,
            reply_rx,
            event_rx,
            palette_name,
            message_text: "This panel will show:\n\
                - On left click, info of the allocation you left clicked on\n\
                - On right click, your current mouse position (x -> timestamp, y -> memory)"
                .to_string(),
            repl_lines: REPL_HINT.iter().map(|s| s.to_string()).collect(),
            repl_text: REPL_HINT.join("\n"),
            repl_input: String::new(),
            command_history: Vec::new(),
            history_index: 0,
            repl_visible: true,
            sql_pending: false,
        };
        (app, Task::none())
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        // Keep the cached repl_text in sync after every update.
        self.repl_text = self.repl_lines.join("\n");
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RendererEvent(text) => {
                self.message_text = text;
                Task::none()
            }

            Message::ReplInputChanged(s) => {
                self.repl_input = s;
                Task::none()
            }

            Message::ReplSubmit => {
                let command = self.repl_input.trim().to_string();
                if command.is_empty() {
                    return Task::none();
                }

                if self.command_history.last().map(|s| s.as_str()) != Some(&command) {
                    self.command_history.push(command.clone());
                }
                self.history_index = self.command_history.len();
                self.repl_input.clear();

                if command == "--clear" {
                    self.repl_lines = REPL_HINT.iter().map(|s| s.to_string()).collect();
                    return Task::none();
                }

                let ts = Local::now().format("%H:%M:%S").to_string();
                self.repl_lines.push(format!("[{ts}] > {command}"));

                let parts: Vec<&str> = command.splitn(2, char::is_whitespace).collect();
                let cmd = parts[0];
                let arg = parts.get(1).copied().map(str::trim);

                match cmd {
                    "--help" => {
                        self.repl_lines.push(format!("[{ts}]\n{HELP_MSG}"));
                        Task::none()
                    }
                    "--schema" => {
                        self.repl_lines.push(format!("[{ts}]\n{DATABASE_SCHEMA}"));
                        Task::none()
                    }
                    "--find" => {
                        match arg {
                            None | Some("") => {
                                self.repl_lines
                                    .push(format!("[{ts}]\nUsage: --find <pattern>"));
                            }
                            Some(pattern) => {
                                let pat_lower = pattern.to_lowercase();
                                let found: Vec<&str> = self
                                    .message_text
                                    .lines()
                                    .filter(|line| line.to_lowercase().contains(&pat_lower))
                                    .collect();
                                if found.is_empty() {
                                    self.repl_lines
                                        .push(format!("[{ts}]\nNo matches found for '{pattern}'."));
                                } else {
                                    let result = format!(
                                        "Found {} matching lines for '{pattern}':\n{}",
                                        found.len(),
                                        found.join("\n")
                                    );
                                    self.repl_lines.push(format!("[{ts}]\n{result}"));
                                }
                            }
                        }
                        Task::none()
                    }
                    _ => {
                        if self.sql_pending {
                            self.repl_lines
                                .push(format!("[{ts}]\nBusy - previous query still in-flight."));
                            return Task::none();
                        }
                        self.sql_pending = true;
                        let sql_tx = self.sql_tx.clone();
                        let reply_rx = Arc::clone(&self.reply_rx);
                        Task::perform(
                            ipc_worker::execute_sql(sql_tx, reply_rx, command),
                            Message::SqlResult,
                        )
                    }
                }
            }

            Message::SqlResult(result) => {
                self.sql_pending = false;
                let ts = Local::now().format("%H:%M:%S").to_string();
                self.repl_lines.push(format!("[{ts}]\n{result}"));
                operation::snap_to_end(Id::new(REPL_OUTPUT_ID))
            }

            Message::ToggleRepl => {
                self.repl_visible = !self.repl_visible;
                Task::none()
            }

            Message::ThemeChanged(name) => {
                self.palette_name = name;
                Task::none()
            }

            Message::Quit => iced::exit(),

            Message::KeyboardEvent(event) => {
                if let keyboard::Event::KeyPressed { key, modifiers, .. } = event {
                    if modifiers.control() {
                        match key.as_ref() {
                            keyboard::Key::Character("d") | keyboard::Key::Character("q") => {
                                return iced::exit();
                            }
                            _ => {}
                        }
                    }
                    match key.as_ref() {
                        keyboard::Key::Named(key::Named::ArrowUp) => self.history_up(),
                        keyboard::Key::Named(key::Named::ArrowDown) => self.history_down(),
                        _ => {}
                    }
                }
                Task::none()
            }
        }
    }

    fn history_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        if self.history_index > 0 {
            self.history_index -= 1;
        }
        self.repl_input = self.command_history[self.history_index].clone();
    }

    fn history_down(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        self.history_index += 1;
        if self.history_index >= self.command_history.len() {
            self.history_index = self.command_history.len();
            self.repl_input.clear();
        } else {
            self.repl_input = self.command_history[self.history_index].clone();
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            ipc_worker::sub_listener(Arc::clone(&self.event_rx))
                .map(|ev| Message::RendererEvent(ev.0)),
            keyboard::listen().map(Message::KeyboardEvent),
        ])
    }

    pub fn theme(&self) -> Theme {
        self.palette_name.to_theme()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let cp = self.palette_name.palette();

        // ── top bar ──────────────────────────────────────────────────────────
        const ALL_PALETTES: &[PaletteName] =
            &[PaletteName::Cute, PaletteName::Default, PaletteName::Night];

        // in iced 0.15, order of this function's args changes? wtf
        let theme_picker = pick_list(ALL_PALETTES, Some(self.palette_name), Message::ThemeChanged);

        let toggle_label = if self.repl_visible {
            "Hide REPL"
        } else {
            "Show REPL"
        };
        let toggle_btn = button(text(toggle_label)).on_press(Message::ToggleRepl);
        let quit_btn = button(text("Quit")).on_press(Message::Quit);

        let top_bar = row![theme_picker, space().width(Fill), toggle_btn, quit_btn]
            .spacing(8)
            .padding([4, 8]);

        // ── message panel ────────────────────────────────────────────────────
        let msg_content = container(
            scrollable(
                text(self.message_text.as_str())
                    .color(cp.text_fg)
                    .size(13)
                    .font(JETBRAINS_MONO),
            )
            .width(Fill)
            .height(Fill),
        )
        .style(|_theme| container::Style {
            background: Some(cp.text_area_bg.into()),
            ..Default::default()
        })
        .padding(10)
        .width(Fill)
        .height(Fill);

        let msg_panel = column![text("Messages").size(20).color(cp.accent), msg_content,]
            .spacing(10)
            .padding(16)
            .width(Fill);

        // ── REPL panel ───────────────────────────────────────────────────────
        let repl_out = container(
            scrollable(
                text(self.repl_text.as_str())
                    .color(cp.text_fg)
                    .size(13)
                    .font(JETBRAINS_MONO),
            )
            .id(Id::new(REPL_OUTPUT_ID))
            .width(Fill)
            .height(Fill),
        )
        .style(|_theme| container::Style {
            background: Some(cp.text_area_bg.into()),
            ..Default::default()
        })
        .padding(10)
        .width(Fill)
        .height(Fill);

        let prompt = text("> ").color(cp.accent).size(14).font(JETBRAINS_MONO);
        let input = text_input("Enter SQL or --help", &self.repl_input)
            .on_input(Message::ReplInputChanged)
            .on_submit(Message::ReplSubmit)
            .size(14)
            .font(JETBRAINS_MONO)
            .width(Fill);

        let input_row = row![prompt, input].spacing(4).align_y(iced::Center);

        let repl_panel = column![
            text("SQLite REPL").size(20).color(cp.accent),
            repl_out,
            input_row,
        ]
        .spacing(10)
        .padding(16)
        .width(Fill);

        // ── main layout ──────────────────────────────────────────────────────
        let panels: Element<_> = if self.repl_visible {
            row![msg_panel, rule::vertical(1), repl_panel,]
                .spacing(0)
                .height(Fill)
                .into()
        } else {
            row![msg_panel].height(Fill).into()
        };

        let root = column![top_bar, panels].height(Fill).spacing(0);

        container(root)
            .style(|_theme| container::Style {
                background: Some(cp.window_bg.into()),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill)
            .into()
    }
}
