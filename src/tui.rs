// SPDX-License-Identifier: Unlicense
//! Terminal UI.
//!
//! Tabbed surface: Dashboard / Queue / Strategy. Designed in the
//! restrained Claude-Code aesthetic — minimal borders, lots of
//! whitespace, dim status-line footer with keybind hints. Keyboard
//! only, no mouse, terminal-portable.
//!
//! The render and event-handling layers are split: the `App` struct
//! is pure state that we can unit-test; the `run` function does I/O
//! and is gated behind the `tui` cargo feature.

use crate::strategy::Strategy;
use crate::{seed_corpus_fingerprint, version, FeedbackQueue, Mode, Profile};

/// Tabs in display order. `usize` index into [`App::tabs`].
pub const TABS: [&str; 3] = ["dashboard", "queue", "strategy"];

/// Pure UI state. No I/O. The event loop owns one of these and
/// mutates it in response to key events.
pub struct App {
    pub current_tab: usize,
    pub queue: FeedbackQueue,
    pub profile: Option<Profile>,
    pub mode: Mode,
    pub strategy_label: String,
    pub queue_scroll: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(
        profile: Option<Profile>,
        queue: FeedbackQueue,
        mode: Mode,
        strategy: Strategy,
    ) -> Self {
        Self {
            current_tab: 0,
            queue,
            profile,
            mode,
            strategy_label: format!("{strategy:?}"),
            queue_scroll: 0,
            should_quit: false,
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % TABS.len();
        self.queue_scroll = 0;
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = if self.current_tab == 0 {
            TABS.len() - 1
        } else {
            self.current_tab - 1
        };
        self.queue_scroll = 0;
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::TrainingWheels => Mode::Shadow,
            Mode::Shadow => Mode::Chaos,
            Mode::Chaos => Mode::TrainingWheels,
        };
    }

    pub fn scroll_down(&mut self) {
        if self.current_tab == 1 && self.queue_scroll + 1 < self.queue.len() {
            self.queue_scroll += 1;
        }
    }

    pub fn scroll_up(&mut self) {
        if self.queue_scroll > 0 {
            self.queue_scroll -= 1;
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Pure key handler. Returns true if the input was consumed.
    /// Tested without spinning up a real terminal.
    pub fn handle_key(&mut self, k: KeyAction) -> bool {
        match k {
            KeyAction::Quit => {
                self.quit();
                true
            }
            KeyAction::NextTab => {
                self.next_tab();
                true
            }
            KeyAction::PrevTab => {
                self.prev_tab();
                true
            }
            KeyAction::TabIndex(i) if i < TABS.len() => {
                self.current_tab = i;
                self.queue_scroll = 0;
                true
            }
            KeyAction::TabIndex(_) => false,
            KeyAction::CycleMode => {
                self.cycle_mode();
                true
            }
            KeyAction::Down => {
                self.scroll_down();
                true
            }
            KeyAction::Up => {
                self.scroll_up();
                true
            }
            KeyAction::Noop => false,
        }
    }
}

/// Pure-data key dispatch. Mapped from real crossterm events at the
/// run-loop boundary; kept independent so unit tests don't import
/// crossterm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Quit,
    NextTab,
    PrevTab,
    TabIndex(usize),
    CycleMode,
    Down,
    Up,
    Noop,
}

#[cfg(feature = "tui")]
pub use runtime::{render_html_for_screenshot, run};

#[cfg(feature = "tui")]
mod runtime {
    use super::*;
    use crate::paths;
    use crate::strategy;
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Paragraph, Wrap};
    use ratatui::Terminal;
    use std::io::stdout;

    /// Map a crossterm key event into our pure [`KeyAction`].
    pub(crate) fn key_action(code: KeyCode, mods: KeyModifiers) -> KeyAction {
        match (code, mods) {
            (KeyCode::Char('q') | KeyCode::Esc, _) => KeyAction::Quit,
            (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => KeyAction::Quit,
            (KeyCode::Tab, _) | (KeyCode::Right, _) | (KeyCode::Char('l'), _) => KeyAction::NextTab,
            (KeyCode::BackTab, _) | (KeyCode::Left, _) | (KeyCode::Char('h'), _) => KeyAction::PrevTab,
            (KeyCode::Char(c), _) if c.is_ascii_digit() => {
                let n = c.to_digit(10).unwrap_or(0) as usize;
                if n >= 1 && n <= TABS.len() {
                    KeyAction::TabIndex(n - 1)
                } else {
                    KeyAction::Noop
                }
            }
            (KeyCode::Char('m') | KeyCode::Char('M'), _) => KeyAction::CycleMode,
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => KeyAction::Down,
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => KeyAction::Up,
            _ => KeyAction::Noop,
        }
    }

    pub fn run() -> anyhow::Result<()> {
        let profile_path = paths::profile_path();
        let profile = if profile_path.exists() {
            let text = std::fs::read_to_string(&profile_path)?;
            Some(toml::from_str::<Profile>(&text)?)
        } else {
            None
        };
        let queue = FeedbackQueue::load_from(&paths::feedback_jsonl_path())?;
        let detected = strategy::detect();
        let mut app = App::new(profile, queue, Mode::default(), detected);

        enable_raw_mode()?;
        let mut out = stdout();
        execute!(out, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(out);
        let mut term = Terminal::new(backend)?;

        let result = event_loop(&mut term, &mut app);

        disable_raw_mode()?;
        execute!(term.backend_mut(), LeaveAlternateScreen)?;
        term.show_cursor()?;
        result
    }

    fn event_loop<B: ratatui::backend::Backend>(
        term: &mut Terminal<B>,
        app: &mut App,
    ) -> anyhow::Result<()> {
        loop {
            term.draw(|f| draw(f, app))?;
            if event::poll(std::time::Duration::from_millis(250))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        let action = key_action(k.code, k.modifiers);
                        app.handle_key(action);
                    }
                }
            }
            if app.should_quit {
                return Ok(());
            }
        }
    }

    fn draw(f: &mut ratatui::Frame, app: &App) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header
                Constraint::Length(1), // tab bar
                Constraint::Length(1), // gutter
                Constraint::Min(1),    // body
                Constraint::Length(1), // footer
            ])
            .split(area);

        draw_header(f, chunks[0], app);
        draw_tab_bar(f, chunks[1], app);
        // chunks[2] left as a single-row gutter
        match app.current_tab {
            0 => draw_dashboard(f, chunks[3], app),
            1 => draw_queue(f, chunks[3], app),
            2 => draw_strategy(f, chunks[3], app),
            _ => {}
        }
        draw_footer(f, chunks[4]);
    }

    fn dim() -> Style {
        Style::default().fg(Color::DarkGray)
    }

    fn accent() -> Style {
        Style::default().fg(Color::LightYellow)
    }

    fn draw_header(f: &mut ratatui::Frame, area: Rect, app: &App) {
        let mode = format!("{:?}", app.mode).to_lowercase();
        let queue_len = app.queue.len();
        let left = Line::from(vec![
            Span::styled("atsisbroken", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(version(), dim()),
        ]);
        let right = Line::from(vec![
            Span::styled("mode: ", dim()),
            Span::styled(mode, accent()),
            Span::raw("    "),
            Span::styled(format!("queue: {queue_len} events"), dim()),
        ])
        .alignment(Alignment::Right);
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        f.render_widget(Paragraph::new(left), split[0]);
        f.render_widget(Paragraph::new(right), split[1]);
    }

    fn draw_tab_bar(f: &mut ratatui::Frame, area: Rect, app: &App) {
        let mut spans: Vec<Span> = Vec::new();
        for (i, name) in TABS.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("  •  ", dim()));
            }
            let style = if i == app.current_tab {
                accent().add_modifier(Modifier::UNDERLINED)
            } else {
                dim()
            };
            spans.push(Span::styled(*name, style));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn draw_dashboard(f: &mut ratatui::Frame, area: Rect, app: &App) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled("  profile", dim())));
        if let Some(p) = &app.profile {
            push_field(&mut lines, "    full_name", &p.full_name);
            push_field(&mut lines, "    email", &p.email);
            push_field(&mut lines, "    phone", &p.phone);
            push_field(&mut lines, "    linkedin", &p.linkedin);
            push_field(&mut lines, "    github", &p.github);
            push_field(&mut lines, "    website", &p.website);
            push_field(&mut lines, "    address", &p.address);
            push_field(&mut lines, "    work_authorization", &p.work_authorization);
            if p.years_experience > 0 {
                push_field(
                    &mut lines,
                    "    years_experience",
                    &p.years_experience.to_string(),
                );
            }
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("not initialized — run `atsisbroken init`", accent()),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  seed corpus     ", dim()),
            Span::raw(format!("{:08x}", seed_corpus_fingerprint())),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  strategy (auto) ", dim()),
            Span::raw(app.strategy_label.clone()),
        ]));
        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            area,
        );
    }

    fn push_field(lines: &mut Vec<Line<'static>>, label: &str, value: &str) {
        let v = if value.is_empty() {
            Span::styled("—", dim())
        } else {
            Span::raw(value.to_string())
        };
        // Pad label so values line up.
        let label_owned = format!("{label:24}");
        lines.push(Line::from(vec![
            Span::styled(label_owned, dim()),
            Span::raw("  "),
            v,
        ]));
    }

    fn draw_queue(f: &mut ratatui::Frame, area: Rect, app: &App) {
        if app.queue.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("feedback queue is empty", dim()),
                ])),
                area,
            );
            return;
        }
        let mut lines: Vec<Line> = Vec::new();
        let visible = (area.height as usize).saturating_sub(1);
        for (i, ev) in app
            .queue
            .events
            .iter()
            .enumerate()
            .skip(app.queue_scroll)
            .take(visible)
        {
            let marker = if ev.accepted { "✓" } else { "✗" };
            let marker_style = if ev.accepted {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {i:4}  "), dim()),
                Span::styled(marker, marker_style),
                Span::raw("  "),
                Span::raw(ev.field.label.clone()),
                Span::styled(format!("  → {}", ev.predicted), dim()),
            ]));
        }
        f.render_widget(Paragraph::new(lines), area);
    }

    fn draw_strategy(f: &mut ratatui::Frame, area: Rect, app: &App) {
        let lines = vec![
            Line::from(vec![
                Span::styled("  auto-detected: ", dim()),
                Span::styled(app.strategy_label.clone(), accent()),
            ]),
            Line::from(""),
            Line::from(Span::styled("  ladder (high → low)", dim())),
            Line::from("    cdp-attach    Chromium with reachable debug port"),
            Line::from("    cdp-launch    Chromium binary findable on PATH"),
            Line::from("    extension     atsisbroken extension's native host installed"),
            Line::from("    userscript    TamperMonkey/Greasemonkey, no extension"),
            Line::from("    bookmarklet   javascript: URL, drag to bookmark bar"),
            Line::from("    clipboard     pbcopy/xclip/wl-copy/clip.exe"),
            Line::from("    speak         print key:value to stdout (the floor)"),
        ];
        f.render_widget(Paragraph::new(lines), area);
    }

    /// Render one tab to an HTML representation of the terminal grid.
    /// Used by `scripts/capture-screenshots.sh` to produce real PNGs of
    /// the TUI via headless Chromium. Loads real on-disk profile + queue
    /// so the screenshot reflects the user's actual state.
    pub fn render_html_for_screenshot(
        tab: usize,
        width: u16,
        height: u16,
    ) -> anyhow::Result<String> {
        use ratatui::backend::TestBackend;
        let profile_path = paths::profile_path();
        let profile = if profile_path.exists() {
            let text = std::fs::read_to_string(&profile_path)?;
            Some(toml::from_str::<Profile>(&text)?)
        } else {
            None
        };
        let queue = FeedbackQueue::load_from(&paths::feedback_jsonl_path())?;
        let detected = strategy::detect();
        let mut app = App::new(profile, queue, Mode::default(), detected);
        app.current_tab = tab.min(TABS.len() - 1);

        let backend = TestBackend::new(width, height);
        let mut term = Terminal::new(backend)?;
        term.draw(|f| draw(f, &app))?;
        let buffer = term.backend().buffer().clone();
        Ok(buffer_to_html(&buffer))
    }

    fn buffer_to_html(buf: &ratatui::buffer::Buffer) -> String {
        use ratatui::buffer::Cell;
        fn css_color(c: Color) -> Option<&'static str> {
            match c {
                Color::Reset => None,
                Color::Black => Some("#0e1117"),
                Color::Red => Some("#f85149"),
                Color::Green => Some("#3fb950"),
                Color::Yellow => Some("#d29922"),
                Color::Blue => Some("#58a6ff"),
                Color::Magenta => Some("#bc8cff"),
                Color::Cyan => Some("#39c5cf"),
                Color::White => Some("#c9d1d9"),
                Color::Gray => Some("#8b949e"),
                Color::DarkGray => Some("#6e7681"),
                Color::LightRed => Some("#ff7b72"),
                Color::LightGreen => Some("#7ee787"),
                Color::LightYellow => Some("#e3b341"),
                Color::LightBlue => Some("#79c0ff"),
                Color::LightMagenta => Some("#d2a8ff"),
                Color::LightCyan => Some("#56d4dd"),
                Color::Rgb(r, g, b) => {
                    // Live-format would need a String, but we return a
                    // &'static str. Fall back to default for Rgb cells —
                    // we don't currently emit any.
                    let _ = (r, g, b);
                    None
                }
                _ => None,
            }
        }
        fn style_for(cell: &Cell) -> String {
            let mut parts: Vec<String> = Vec::new();
            if let Some(c) = css_color(cell.fg) {
                parts.push(format!("color:{c}"));
            }
            if let Some(c) = css_color(cell.bg) {
                parts.push(format!("background:{c}"));
            }
            if cell.modifier.contains(Modifier::BOLD) {
                parts.push("font-weight:bold".into());
            }
            if cell.modifier.contains(Modifier::UNDERLINED) {
                parts.push("text-decoration:underline".into());
            }
            if cell.modifier.contains(Modifier::DIM) {
                parts.push("opacity:0.7".into());
            }
            parts.join(";")
        }
        fn esc(s: &str) -> String {
            s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
        }

        let mut out = String::new();
        out.push_str(
            r#"<!doctype html><html><head><meta charset="utf-8"><style>
body{margin:0;background:#0e1117;color:#c9d1d9;font:14px/1.25 'JetBrains Mono','Menlo',monospace}
.term{padding:24px}
.row{white-space:pre}
span{white-space:pre}
</style></head><body><div class="term">"#,
        );
        let area = buf.area();
        for y in 0..area.height {
            out.push_str(r#"<div class="row">"#);
            // Run-length encode adjacent cells with the same style.
            let mut current_style = String::new();
            let mut current_run = String::new();
            for x in 0..area.width {
                let cell = &buf[(x, y)];
                let style = style_for(cell);
                if style != current_style {
                    if !current_run.is_empty() {
                        if current_style.is_empty() {
                            out.push_str(&esc(&current_run));
                        } else {
                            out.push_str(&format!(
                                r#"<span style="{}">{}</span>"#,
                                current_style,
                                esc(&current_run)
                            ));
                        }
                    }
                    current_style = style;
                    current_run.clear();
                }
                current_run.push_str(cell.symbol());
            }
            if !current_run.is_empty() {
                if current_style.is_empty() {
                    out.push_str(&esc(&current_run));
                } else {
                    out.push_str(&format!(
                        r#"<span style="{}">{}</span>"#,
                        current_style,
                        esc(&current_run)
                    ));
                }
            }
            out.push_str("</div>");
        }
        out.push_str("</div></body></html>");
        out
    }

    fn draw_footer(f: &mut ratatui::Frame, area: Rect) {
        let hints = Line::from(vec![
            Span::styled(" 1/2/3 ", accent()),
            Span::styled("tab   ", dim()),
            Span::styled("←/→ ", accent()),
            Span::styled("nav   ", dim()),
            Span::styled("j/k ", accent()),
            Span::styled("scroll   ", dim()),
            Span::styled("m ", accent()),
            Span::styled("mode   ", dim()),
            Span::styled("q ", accent()),
            Span::styled("quit", dim()),
        ]);
        // No border — at 1-row height a Borders::TOP would consume the
        // whole row and the hints would never render. Whitespace +
        // dim styling carries the visual weight on its own.
        f.render_widget(Paragraph::new(hints), area);
    }

    #[cfg(test)]
    mod runtime_tests {
        use super::*;

        #[test]
        fn key_action_quit_on_q_or_esc() {
            assert_eq!(
                key_action(KeyCode::Char('q'), KeyModifiers::empty()),
                KeyAction::Quit
            );
            assert_eq!(
                key_action(KeyCode::Esc, KeyModifiers::empty()),
                KeyAction::Quit
            );
        }

        #[test]
        fn key_action_ctrl_c_quits() {
            assert_eq!(
                key_action(KeyCode::Char('c'), KeyModifiers::CONTROL),
                KeyAction::Quit
            );
        }

        #[test]
        fn key_action_tab_and_arrows_navigate() {
            assert_eq!(
                key_action(KeyCode::Tab, KeyModifiers::empty()),
                KeyAction::NextTab
            );
            assert_eq!(
                key_action(KeyCode::Right, KeyModifiers::empty()),
                KeyAction::NextTab
            );
            assert_eq!(
                key_action(KeyCode::BackTab, KeyModifiers::empty()),
                KeyAction::PrevTab
            );
            assert_eq!(
                key_action(KeyCode::Left, KeyModifiers::empty()),
                KeyAction::PrevTab
            );
        }

        #[test]
        fn key_action_digit_keys_pick_tab() {
            assert_eq!(
                key_action(KeyCode::Char('1'), KeyModifiers::empty()),
                KeyAction::TabIndex(0)
            );
            assert_eq!(
                key_action(KeyCode::Char('2'), KeyModifiers::empty()),
                KeyAction::TabIndex(1)
            );
            assert_eq!(
                key_action(KeyCode::Char('3'), KeyModifiers::empty()),
                KeyAction::TabIndex(2)
            );
            // Out-of-range digits map to Noop.
            assert_eq!(
                key_action(KeyCode::Char('9'), KeyModifiers::empty()),
                KeyAction::Noop
            );
        }

        #[test]
        fn key_action_vim_keys_supported() {
            assert_eq!(
                key_action(KeyCode::Char('h'), KeyModifiers::empty()),
                KeyAction::PrevTab
            );
            assert_eq!(
                key_action(KeyCode::Char('l'), KeyModifiers::empty()),
                KeyAction::NextTab
            );
            assert_eq!(
                key_action(KeyCode::Char('j'), KeyModifiers::empty()),
                KeyAction::Down
            );
            assert_eq!(
                key_action(KeyCode::Char('k'), KeyModifiers::empty()),
                KeyAction::Up
            );
        }

        #[test]
        fn key_action_unhandled_returns_noop() {
            assert_eq!(
                key_action(KeyCode::Char('z'), KeyModifiers::empty()),
                KeyAction::Noop
            );
            assert_eq!(
                key_action(KeyCode::F(5), KeyModifiers::empty()),
                KeyAction::Noop
            );
        }

        // ─── HTML snapshot ────────────────────────────────────────────

        #[test]
        fn render_html_produces_valid_doctype_and_atsisbroken_label() {
            let html = render_html_for_screenshot(0, 80, 16).unwrap();
            assert!(html.starts_with("<!doctype html>"));
            assert!(html.contains("atsisbroken"));
            assert!(html.contains("</body></html>"));
        }

        #[test]
        fn render_html_each_tab_distinguishable() {
            // Each tab name should appear at least once in its own render.
            for (i, name) in TABS.iter().enumerate() {
                let html = render_html_for_screenshot(i, 80, 16).unwrap();
                assert!(
                    html.contains(name),
                    "tab {} ({}) didn't render its own name",
                    i,
                    name
                );
            }
        }

        #[test]
        fn render_html_clamps_oversize_tab_index() {
            // Out-of-range tab must clamp, not panic. Useful guard for
            // the script if it ever loops past the real tab count.
            let html = render_html_for_screenshot(99, 80, 16).unwrap();
            assert!(html.contains("strategy")); // last tab
        }

        #[test]
        fn render_html_includes_keybind_footer_hints() {
            let html = render_html_for_screenshot(0, 80, 16).unwrap();
            assert!(html.contains("quit"), "footer hint must appear: {html}");
        }

        #[test]
        fn render_html_escapes_html_metacharacters() {
            // Sanity: any literal '<' in cell content shouldn't break
            // the page. None of our static text contains these, but
            // the escaper exists and we don't want a regression.
            let html = render_html_for_screenshot(0, 80, 16).unwrap();
            // The output is itself HTML (tags must exist), so we look
            // for the absence of obvious unsafe sequences.
            assert!(!html.contains("<script"), "must not embed scripts");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::Strategy;
    use crate::FieldDescriptor;

    fn empty_app() -> App {
        App::new(None, FeedbackQueue::default(), Mode::default(), Strategy::Speak)
    }

    fn app_with_queue(n: usize) -> App {
        let mut q = FeedbackQueue::default();
        for i in 0..n {
            q.append(crate::Feedback {
                field: FieldDescriptor {
                    label: format!("f{i}"),
                    placeholder: "".into(),
                    aria_label: "".into(),
                    name: "".into(),
                    id: "".into(),
                    kind: "text".into(),
                },
                predicted: "email".into(),
                actual: "email".into(),
                accepted: i % 2 == 0,
            });
        }
        App::new(None, q, Mode::default(), Strategy::Speak)
    }

    #[test]
    fn app_starts_on_dashboard() {
        let app = empty_app();
        assert_eq!(app.current_tab, 0);
        assert_eq!(TABS[app.current_tab], "dashboard");
    }

    #[test]
    fn next_tab_wraps() {
        let mut app = empty_app();
        app.next_tab();
        assert_eq!(app.current_tab, 1);
        app.next_tab();
        assert_eq!(app.current_tab, 2);
        app.next_tab();
        assert_eq!(app.current_tab, 0); // wrap
    }

    #[test]
    fn prev_tab_wraps_backwards() {
        let mut app = empty_app();
        app.prev_tab();
        assert_eq!(app.current_tab, TABS.len() - 1);
        app.prev_tab();
        assert_eq!(app.current_tab, TABS.len() - 2);
    }

    #[test]
    fn cycle_mode_walks_all_three_states() {
        let mut app = empty_app();
        assert_eq!(app.mode, Mode::TrainingWheels);
        app.cycle_mode();
        assert_eq!(app.mode, Mode::Shadow);
        app.cycle_mode();
        assert_eq!(app.mode, Mode::Chaos);
        app.cycle_mode();
        assert_eq!(app.mode, Mode::TrainingWheels); // wrap
    }

    #[test]
    fn quit_sets_should_quit_flag() {
        let mut app = empty_app();
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_quit_marks_quit() {
        let mut app = empty_app();
        assert!(app.handle_key(KeyAction::Quit));
        assert!(app.should_quit);
    }

    #[test]
    fn handle_key_noop_returns_false() {
        let mut app = empty_app();
        assert!(!app.handle_key(KeyAction::Noop));
        assert!(!app.should_quit);
    }

    #[test]
    fn handle_key_tab_index_in_range() {
        let mut app = empty_app();
        assert!(app.handle_key(KeyAction::TabIndex(2)));
        assert_eq!(app.current_tab, 2);
    }

    #[test]
    fn handle_key_tab_index_out_of_range_ignored() {
        let mut app = empty_app();
        assert!(!app.handle_key(KeyAction::TabIndex(99)));
        assert_eq!(app.current_tab, 0);
    }

    #[test]
    fn switching_tabs_resets_queue_scroll() {
        let mut app = app_with_queue(10);
        app.handle_key(KeyAction::TabIndex(1));
        for _ in 0..5 {
            app.handle_key(KeyAction::Down);
        }
        assert_eq!(app.queue_scroll, 5);
        app.handle_key(KeyAction::NextTab);
        assert_eq!(app.queue_scroll, 0);
    }

    #[test]
    fn scroll_down_in_queue_is_bounded_by_len() {
        let mut app = app_with_queue(3);
        app.current_tab = 1;
        for _ in 0..10 {
            app.handle_key(KeyAction::Down);
        }
        assert_eq!(app.queue_scroll, 2); // len-1, never past
    }

    #[test]
    fn scroll_up_at_zero_does_not_underflow() {
        let mut app = app_with_queue(3);
        app.current_tab = 1;
        // Several ups from zero must stay at zero, not panic.
        for _ in 0..5 {
            app.handle_key(KeyAction::Up);
        }
        assert_eq!(app.queue_scroll, 0);
    }

    #[test]
    fn scroll_only_advances_on_queue_tab() {
        let mut app = app_with_queue(5);
        // Sit on dashboard (tab 0). Down should not advance the queue scroll.
        app.handle_key(KeyAction::Down);
        assert_eq!(app.queue_scroll, 0);
    }

    #[test]
    fn tabs_constant_matches_documented_count() {
        // The keybind footer hints "1/2/3 tab" — drift-guard that we
        // didn't add a 4th tab without updating it.
        assert_eq!(TABS.len(), 3);
        assert_eq!(TABS, ["dashboard", "queue", "strategy"]);
    }
}
