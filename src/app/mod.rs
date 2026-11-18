//! UI state machine. Owns screens, focus, selection and dialogs; talks to
//! the game only through a `GameProvider`. No rendering, no sockets.

pub mod fixtures;
pub mod mock;
pub mod provider;

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::domain::{Color, DrawReason, Game, Role, Square, color_name, parse_square};
use crate::identity::normalize_fingerprint;
pub use provider::*;

pub const DEFAULT_PORT: u16 = 4747;
const STATUS_TTL: Duration = Duration::from_millis(4500);

/// What the user asked to start; the factory turns it into a provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartRequest {
    Host,
    Join {
        addr: String,
        code: String,
        fingerprint: Option<String>,
    },
    Local,
}

pub type ProviderFactory = Box<dyn FnMut(StartRequest, Instant) -> Box<dyn GameProvider>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    Host,
    Join,
    Local,
    HowTo,
    Quit,
}

impl MenuItem {
    pub const ALL: [MenuItem; 5] = [
        MenuItem::Host,
        MenuItem::Join,
        MenuItem::Local,
        MenuItem::HowTo,
        MenuItem::Quit,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MenuItem::Host => "Host a game",
            MenuItem::Join => "Join a game",
            MenuItem::Local => "Local game · pass & play",
            MenuItem::HowTo => "How to play",
            MenuItem::Quit => "Quit",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            MenuItem::Host => "Open a table on this machine and invite a friend",
            MenuItem::Join => "Connect directly to a friend's table",
            MenuItem::Local => "Two players, one keyboard",
            MenuItem::HowTo => "Keys, moves and connection tips",
            MenuItem::Quit => "Leave Vesper",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JoinForm {
    pub address: String,
    pub code: String,
    pub fingerprint: String,
    pub field: usize,
    pub error: Option<(usize, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Board,
    History,
    Actions,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Resign,
    ClaimDraw,
    Flip,
    Glyphs,
    Leave,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Resign => "Resign",
            Action::ClaimDraw => "Claim draw",
            Action::Flip => "Flip board",
            Action::Glyphs => "Piece style",
            Action::Leave => "Leave table",
        }
    }

    pub fn key(self) -> char {
        match self {
            Action::Resign => 'r',
            Action::ClaimDraw => 'd',
            Action::Flip => 'o',
            Action::Glyphs => 'g',
            Action::Leave => 'q',
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    Resign,
    ClaimDraw(DrawReason),
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    Help,
    Promotion {
        from: Square,
        to: Square,
        choice: usize,
    },
    Confirm(Confirm),
    Result,
}

pub const PROMOTION_ROLES: [Role; 4] = [Role::Queen, Role::Rook, Role::Bishop, Role::Knight];

/// Semantic message tone; the theme maps it to colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Accent,
    Positive,
    Warning,
    Danger,
    Muted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub text: String,
    pub tone: Tone,
    pub until: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandInput {
    pub text: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GameScreen {
    pub view: GameView,
    pub cursor: Square,
    pub selected: Option<Square>,
    pub targets: Vec<Square>,
    pub orientation: Color,
    pub focus: Focus,
    pub pending: Option<String>,
    pub command: Option<CommandInput>,
    pub status: Option<Status>,
    pub history_scroll: Option<usize>,
    pub action: usize,
    pub result_seen: bool,
}

impl GameScreen {
    pub fn new(view: GameView) -> Self {
        let orientation = match view.seat {
            Seat::Player(c) => c,
            Seat::Both => Color::White,
        };
        let cursor = match orientation {
            Color::White => Square::E2,
            Color::Black => Square::E7,
        };
        Self {
            view,
            cursor,
            selected: None,
            targets: Vec::new(),
            orientation,
            focus: Focus::Board,
            pending: None,
            command: None,
            status: None,
            history_scroll: None,
            action: 0,
            result_seen: false,
        }
    }

    pub fn game(&self) -> &Game {
        &self.view.game
    }

    pub fn can_move(&self) -> bool {
        self.view.my_turn() && self.view.link.can_play() && self.pending.is_none()
    }

    pub fn actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        if !self.game().is_over() {
            actions.push(Action::Resign);
            if self.game().claimable_draw().is_some() && self.view.my_turn() {
                actions.push(Action::ClaimDraw);
            }
        }
        actions.extend([Action::Flip, Action::Glyphs, Action::Leave]);
        actions
    }

    fn set_status(&mut self, text: impl Into<String>, tone: Tone, now: Instant) {
        self.status = Some(Status {
            text: text.into(),
            tone,
            until: Some(now + STATUS_TTL),
        });
    }

    fn clear_selection(&mut self) {
        self.selected = None;
        self.targets.clear();
    }

    fn refresh_selection(&mut self) {
        if let Some(sel) = self.selected {
            let own = self
                .game()
                .piece_at(sel)
                .is_some_and(|p| p.color == self.game().turn());
            if own && self.can_move() {
                self.targets = self.game().legal_targets(sel);
            } else {
                self.clear_selection();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Screen {
    Home { selected: usize },
    Join(JoinForm),
    Waiting(WaitingInfo),
    Connecting { addr: String },
    Game(Box<GameScreen>),
}

pub struct App {
    pub screen: Screen,
    pub overlay: Option<Overlay>,
    pub ascii: bool,
    pub quit: bool,
    pub demo: bool,
    pub notice: Option<Status>,
    pub started_at: Instant,
    pub now: Instant,
    provider: Option<Box<dyn GameProvider>>,
    factory: ProviderFactory,
}

impl App {
    pub fn new(factory: ProviderFactory, now: Instant) -> Self {
        Self {
            screen: Screen::Home { selected: 0 },
            overlay: None,
            ascii: false,
            quit: false,
            demo: false,
            notice: None,
            started_at: now,
            now,
            provider: None,
            factory,
        }
    }

    /// Animation frame counter for spinners (≈ 8 fps).
    pub fn frame(&self) -> usize {
        (self.now.duration_since(self.started_at).as_millis() / 120) as usize
    }

    pub fn start(&mut self, request: StartRequest) {
        let now = self.now;
        let provider = (self.factory)(request.clone(), now);
        self.provider = Some(provider);
        self.overlay = None;
        self.notice = None;
        self.screen = match request {
            StartRequest::Host => Screen::Waiting(WaitingInfo {
                addresses: Vec::new(),
                port: DEFAULT_PORT,
                invite: String::new(),
                seat: Color::White,
                fingerprint: String::new(),
                public_ip: None,
            }),
            StartRequest::Join { addr, .. } => Screen::Connecting { addr },
            StartRequest::Local => Screen::Connecting {
                addr: "local".into(),
            },
        };
        self.tick(now);
    }

    pub fn attach(&mut self, provider: Box<dyn GameProvider>, initial: Option<ProviderEvent>) {
        self.provider = Some(provider);
        if let Some(ev) = initial {
            self.on_event(ev);
        }
    }

    pub fn tick(&mut self, now: Instant) {
        self.now = now;
        let events = match self.provider.as_mut() {
            Some(p) => p.poll(now),
            None => Vec::new(),
        };
        for ev in events {
            self.on_event(ev);
        }
        if let Screen::Game(g) = &mut self.screen
            && g.status
                .as_ref()
                .and_then(|s| s.until)
                .is_some_and(|t| t <= now)
        {
            g.status = None;
        }
        if self
            .notice
            .as_ref()
            .and_then(|s| s.until)
            .is_some_and(|t| t <= now)
        {
            self.notice = None;
        }
    }

    pub fn on_event(&mut self, event: ProviderEvent) {
        let now = self.now;
        match event {
            ProviderEvent::Waiting(info) => self.screen = Screen::Waiting(info),
            ProviderEvent::Connecting { addr } => self.screen = Screen::Connecting { addr },
            ProviderEvent::View(view) => match &mut self.screen {
                Screen::Game(g) => {
                    let was_link = g.view.link;
                    let moved = view.game.history().len() != g.game().history().len();
                    g.view = *view;
                    if moved || g.view.game.is_over() {
                        g.pending = None;
                    }
                    if was_link != g.view.link {
                        match g.view.link {
                            Link::Live { .. } if !was_link.can_play() => {
                                g.set_status("Back online", Tone::Positive, now)
                            }
                            Link::Live { .. } if was_link == Link::OpponentOffline => {
                                g.set_status("Opponent reconnected", Tone::Positive, now)
                            }
                            _ => {}
                        }
                        if !g.view.link.can_play() {
                            g.pending = None;
                        }
                    }
                    g.refresh_selection();
                    if g.game().is_over() && !g.result_seen {
                        g.clear_selection();
                        g.command = None;
                        self.overlay = Some(Overlay::Result);
                    }
                }
                _ => {
                    let g = GameScreen::new(*view);
                    let over = g.game().is_over();
                    self.screen = Screen::Game(Box::new(g));
                    self.overlay = over.then_some(Overlay::Result);
                }
            },
            ProviderEvent::Rejected { reason } => {
                if let Screen::Game(g) = &mut self.screen {
                    g.pending = None;
                    g.set_status(reason, Tone::Danger, now);
                }
            }
            ProviderEvent::Failed { reason } => {
                self.provider = None;
                self.overlay = None;
                self.screen = Screen::Home { selected: 0 };
                self.notice = Some(Status {
                    text: reason,
                    tone: Tone::Danger,
                    until: Some(now + STATUS_TTL * 2),
                });
            }
        }
    }

    fn send(&mut self, command: GameCommand) {
        if let Some(p) = self.provider.as_mut() {
            p.send(command);
        }
    }

    fn go_home(&mut self) {
        self.send(GameCommand::Leave);
        self.provider = None;
        self.overlay = None;
        self.screen = Screen::Home { selected: 0 };
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if let Some(overlay) = self.overlay {
            self.on_overlay_key(overlay, key);
            return;
        }
        match &self.screen {
            Screen::Home { .. } => self.on_home_key(key),
            Screen::Join(_) => self.on_join_key(key),
            Screen::Waiting(_) | Screen::Connecting { .. } => match key.code {
                KeyCode::Esc => self.go_home(),
                KeyCode::Char('?') => self.overlay = Some(Overlay::Help),
                _ => {}
            },
            Screen::Game(_) => self.on_game_key(key),
        }
    }

    fn on_home_key(&mut self, key: KeyEvent) {
        let Screen::Home { selected } = &mut self.screen else {
            return;
        };
        let n = MenuItem::ALL.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => *selected = (*selected + n - 1) % n,
            KeyCode::Down | KeyCode::Char('j') => *selected = (*selected + 1) % n,
            KeyCode::Char('?') => self.overlay = Some(Overlay::Help),
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Enter | KeyCode::Char(' ') => match MenuItem::ALL[*selected] {
                MenuItem::Host => self.start(StartRequest::Host),
                MenuItem::Join => self.screen = Screen::Join(JoinForm::default()),
                MenuItem::Local => self.start(StartRequest::Local),
                MenuItem::HowTo => self.overlay = Some(Overlay::Help),
                MenuItem::Quit => self.quit = true,
            },
            _ => {}
        }
    }

    fn on_join_key(&mut self, key: KeyEvent) {
        let Screen::Join(form) = &mut self.screen else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.screen = Screen::Home { selected: 1 },
            KeyCode::Tab | KeyCode::Down => form.field = (form.field + 1) % 3,
            KeyCode::BackTab | KeyCode::Up => form.field = (form.field + 2) % 3,
            KeyCode::Backspace => {
                field_mut(form).pop();
                form.error = None;
            }
            KeyCode::Enter => {
                if form.field == 0 && form.code.is_empty() {
                    form.field = 1;
                    return;
                }
                let fingerprint = match form.fingerprint.trim() {
                    "" => None,
                    raw => match normalize_fingerprint(raw) {
                        Some(fp) => Some(fp),
                        None => {
                            form.error = Some((2, "Host key looks like A1B2C3D4-E5F60718".into()));
                            return;
                        }
                    },
                };
                match validate_join(&form.address, &form.code) {
                    Ok((addr, code)) => self.start(StartRequest::Join {
                        addr,
                        code,
                        fingerprint,
                    }),
                    Err(err) => form.error = Some(err),
                }
            }
            KeyCode::Char(' ') => match form.field {
                0 => {
                    let word = form.address.trim().to_ascii_lowercase();
                    if word == "vesper" || word == "join" {
                        form.address.clear();
                    } else if !form.address.is_empty() {
                        form.field = 1;
                    }
                    form.error = None;
                }
                1 => {
                    form.field = 2;
                    form.error = None;
                }
                _ => {}
            },
            KeyCode::Char(c) if !c.is_control() => {
                let limit = match form.field {
                    0 => 64,
                    1 => 12,
                    _ => 19,
                };
                let field = field_mut(form);
                if field.chars().count() < limit {
                    field.push(c);
                }
                form.error = None;
            }
            _ => {}
        }
    }

    fn on_overlay_key(&mut self, overlay: Overlay, key: KeyEvent) {
        match overlay {
            Overlay::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter | KeyCode::Char('q')
                ) {
                    self.overlay = None;
                }
            }
            Overlay::Promotion { from, to, choice } => {
                let pick = |c: usize| Overlay::Promotion {
                    from,
                    to,
                    choice: c,
                };
                match key.code {
                    KeyCode::Esc => {
                        self.overlay = None;
                    }
                    KeyCode::Left | KeyCode::Char('h') | KeyCode::Up | KeyCode::Char('k') => {
                        self.overlay = Some(pick((choice + 3) % 4))
                    }
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Down | KeyCode::Char('j') => {
                        self.overlay = Some(pick((choice + 1) % 4))
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        self.submit_move(from, to, Some(PROMOTION_ROLES[choice]))
                    }
                    KeyCode::Char(c) => {
                        if let Some(i) = "qrbn".find(c.to_ascii_lowercase()) {
                            self.submit_move(from, to, Some(PROMOTION_ROLES[i]));
                        }
                    }
                    _ => {}
                }
            }
            Overlay::Confirm(kind) => match key.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.overlay = None;
                    match kind {
                        Confirm::Resign => self.send_command(GameCommand::Resign, "Resigning…"),
                        Confirm::ClaimDraw(_) => {
                            self.send_command(GameCommand::ClaimDraw, "Claiming draw…")
                        }
                        Confirm::Leave => self.go_home(),
                    }
                }
                KeyCode::Esc | KeyCode::Char('n') => self.overlay = None,
                _ => {}
            },
            Overlay::Result => match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                    self.overlay = None;
                    if let Screen::Game(g) = &mut self.screen {
                        g.result_seen = true;
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('h') => self.go_home(),
                KeyCode::Char('q') => self.quit = true,
                _ => {}
            },
        }
    }

    fn send_command(&mut self, command: GameCommand, pending: &str) {
        if let Screen::Game(g) = &mut self.screen {
            g.pending = Some(pending.into());
        }
        self.send(command);
    }

    fn submit_move(&mut self, from: Square, to: Square, promotion: Option<Role>) {
        self.overlay = None;
        if let Screen::Game(g) = &mut self.screen {
            g.clear_selection();
            g.command = None;
            g.pending = Some(format!("Sending move {from} → {to}…"));
        }
        self.send(GameCommand::Move {
            from,
            to,
            promotion,
        });
    }

    fn on_game_key(&mut self, key: KeyEvent) {
        let now = self.now;
        let demo = self.demo;
        let Screen::Game(g) = &mut self.screen else {
            return;
        };

        if let Some(input) = &mut g.command {
            match key.code {
                KeyCode::Esc => g.command = None,
                KeyCode::Backspace => {
                    input.text.pop();
                    input.error = None;
                }
                KeyCode::Enter => {
                    let text = input.text.clone();
                    match parse_coordinate_move(g, &text) {
                        Ok(CoordinateMove::Move(from, to, promo)) => {
                            self.submit_move(from, to, promo)
                        }
                        Ok(CoordinateMove::NeedsPromotion(from, to)) => {
                            g.command = None;
                            self.overlay = Some(Overlay::Promotion {
                                from,
                                to,
                                choice: 0,
                            });
                        }
                        Err(err) => {
                            if let Some(input) = &mut g.command {
                                input.error = Some(err);
                            }
                        }
                    }
                }
                KeyCode::Char(c) if !c.is_control() && input.text.len() < 12 => {
                    input.text.push(c);
                    input.error = None;
                }
                _ => {}
            }
            return;
        }

        let over = g.game().is_over();
        match key.code {
            KeyCode::Tab => {
                g.focus = match g.focus {
                    Focus::Board => Focus::History,
                    Focus::History => Focus::Actions,
                    Focus::Actions => Focus::Board,
                }
            }
            KeyCode::BackTab => {
                g.focus = match g.focus {
                    Focus::Board => Focus::Actions,
                    Focus::History => Focus::Board,
                    Focus::Actions => Focus::History,
                }
            }
            KeyCode::Char('?') => self.overlay = Some(Overlay::Help),
            KeyCode::Char('/') | KeyCode::Char(':') => {
                if g.can_move() {
                    g.command = Some(CommandInput {
                        text: String::new(),
                        error: None,
                    });
                } else {
                    let msg = blocked_reason(g);
                    g.set_status(msg, Tone::Warning, now);
                }
            }
            KeyCode::Char('o') => g.orientation = !g.orientation,
            KeyCode::Char('g') => self.ascii = !self.ascii,
            KeyCode::Char('r') if !over => self.overlay = Some(Overlay::Confirm(Confirm::Resign)),
            KeyCode::Char('d') if !over && g.view.my_turn() => match g.game().claimable_draw() {
                Some(reason) => self.overlay = Some(Overlay::Confirm(Confirm::ClaimDraw(reason))),
                None => g.set_status("No draw can be claimed now", Tone::Warning, now),
            },
            KeyCode::Char('q') if over => self.quit = true,
            KeyCode::Char('q') => self.overlay = Some(Overlay::Confirm(Confirm::Leave)),
            KeyCode::Char('n') if over => self.go_home(),
            KeyCode::Char('x') if demo => {
                if let Some(p) = self.provider.as_mut() {
                    p.simulate_drop(now);
                }
            }
            KeyCode::Esc => {
                if g.selected.is_some() {
                    g.clear_selection();
                } else if g.focus != Focus::Board {
                    g.focus = Focus::Board;
                }
            }
            _ => match g.focus {
                Focus::Board => self.on_board_key(key),
                Focus::History => on_history_key(g, key),
                Focus::Actions => self.on_actions_key(key),
            },
        }
    }

    fn on_board_key(&mut self, key: KeyEvent) {
        let now = self.now;
        let Screen::Game(g) = &mut self.screen else {
            return;
        };
        let (df, dr) = match key.code {
            KeyCode::Up | KeyCode::Char('k') => (0, 1),
            KeyCode::Down | KeyCode::Char('j') => (0, -1),
            KeyCode::Left | KeyCode::Char('h') => (-1, 0),
            KeyCode::Right | KeyCode::Char('l') => (1, 0),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let sq = g.cursor;
                if let Some((from, to, promo)) = select_square(g, sq, now) {
                    if promo {
                        self.overlay = Some(Overlay::Promotion {
                            from,
                            to,
                            choice: 0,
                        });
                    } else {
                        self.submit_move(from, to, None);
                    }
                }
                return;
            }
            _ => return,
        };
        let (df, dr) = match g.orientation {
            Color::White => (df, dr),
            Color::Black => (-df, -dr),
        };
        let file = (g.cursor.file() as i32 + df).clamp(0, 7);
        let rank = (g.cursor.rank() as i32 + dr).clamp(0, 7);
        g.cursor = Square::from_coords(
            crate::domain::File::new(file as u32),
            crate::domain::Rank::new(rank as u32),
        );
    }

    fn on_actions_key(&mut self, key: KeyEvent) {
        let Screen::Game(g) = &mut self.screen else {
            return;
        };
        let actions = g.actions();
        let n = actions.len();
        g.action = g.action.min(n - 1);
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => g.action = (g.action + n - 1) % n,
            KeyCode::Down | KeyCode::Char('j') => g.action = (g.action + 1) % n,
            KeyCode::Enter | KeyCode::Char(' ') => {
                let action = actions[g.action];
                let key = KeyEvent::new(KeyCode::Char(action.key()), KeyModifiers::NONE);
                if action == Action::Leave && g.game().is_over() {
                    self.go_home();
                } else {
                    self.on_game_key(key);
                }
            }
            _ => {}
        }
    }
}

fn on_history_key(g: &mut GameScreen, key: KeyEvent) {
    let rows = g.game().history().len().div_ceil(2);
    let current = g.history_scroll.unwrap_or(rows.saturating_sub(1));
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => g.history_scroll = Some(current.saturating_sub(1)),
        KeyCode::Down | KeyCode::Char('j') => {
            let next = current + 1;
            g.history_scroll = if next + 1 >= rows { None } else { Some(next) };
        }
        KeyCode::Home | KeyCode::Char('g') => g.history_scroll = Some(0),
        KeyCode::End | KeyCode::Char('G') => g.history_scroll = None,
        _ => {}
    }
}

fn field_mut(form: &mut JoinForm) -> &mut String {
    match form.field {
        0 => &mut form.address,
        1 => &mut form.code,
        _ => &mut form.fingerprint,
    }
}

fn blocked_reason(g: &GameScreen) -> String {
    if g.game().is_over() {
        "The game is over".into()
    } else if !g.view.link.can_play() {
        "Connection lost · moves paused".into()
    } else if g.pending.is_some() {
        "Waiting for the host to confirm…".into()
    } else {
        format!("Waiting for {}'s move", color_name(g.game().turn()))
    }
}

/// Returns `(from, to, needs_promotion)` when the press completes a move.
fn select_square(g: &mut GameScreen, sq: Square, now: Instant) -> Option<(Square, Square, bool)> {
    let piece = g.game().piece_at(sq);
    let turn = g.game().turn();
    if let Some(from) = g.selected {
        if sq == from {
            g.clear_selection();
            return None;
        }
        if g.targets.contains(&sq) {
            let promo = g.game().needs_promotion(from, sq);
            return Some((from, sq, promo));
        }
        if piece.is_some_and(|p| p.color == turn) {
            g.selected = Some(sq);
            g.targets = g.game().legal_targets(sq);
            if g.targets.is_empty() {
                g.set_status("That piece has no legal moves", Tone::Warning, now);
            }
            return None;
        }
        g.set_status(
            "Illegal move · choose a highlighted square",
            Tone::Danger,
            now,
        );
        return None;
    }
    if !g.can_move() {
        let msg = blocked_reason(g);
        g.set_status(msg, Tone::Warning, now);
        return None;
    }
    match piece {
        Some(p) if p.color == turn => {
            g.selected = Some(sq);
            g.targets = g.game().legal_targets(sq);
            if g.targets.is_empty() {
                g.set_status("That piece has no legal moves", Tone::Warning, now);
            }
        }
        Some(_) => g.set_status(
            format!("That's {}'s piece", color_name(!turn)),
            Tone::Warning,
            now,
        ),
        None => {}
    }
    None
}

#[derive(Debug, PartialEq, Eq)]
pub enum CoordinateMove {
    Move(Square, Square, Option<Role>),
    NeedsPromotion(Square, Square),
}

pub fn parse_coordinate_move(g: &GameScreen, text: &str) -> Result<CoordinateMove, String> {
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | 'x' | '→'))
        .collect::<String>()
        .to_ascii_lowercase();
    if cleaned.len() != 4 && cleaned.len() != 5 {
        return Err("Use coordinates like e2e4 or e7e8q".into());
    }
    let (Some(from), Some(to)) = (parse_square(&cleaned[0..2]), parse_square(&cleaned[2..4]))
    else {
        return Err("Use coordinates like e2e4 or e7e8q".into());
    };
    let promotion = match cleaned.get(4..5) {
        None => None,
        Some(p) => Some(match p {
            "q" => Role::Queen,
            "r" => Role::Rook,
            "b" => Role::Bishop,
            "n" => Role::Knight,
            _ => return Err("Promotion piece must be q, r, b or n".into()),
        }),
    };
    if !g.can_move() {
        return Err(blocked_reason(g));
    }
    if !g.game().legal_targets(from).contains(&to) {
        return Err(format!("{from}{to} is not legal here"));
    }
    let needs = g.game().needs_promotion(from, to);
    match (needs, promotion) {
        (true, None) => Ok(CoordinateMove::NeedsPromotion(from, to)),
        (true, Some(p)) => Ok(CoordinateMove::Move(from, to, Some(p))),
        (false, None) => Ok(CoordinateMove::Move(from, to, None)),
        (false, Some(_)) => Err(format!("{from}{to} is not a promotion")),
    }
}

/// Validates the join form. Returns normalized `(host:port, CODE)` or
/// `(field index, message)`.
pub fn validate_join(address: &str, code: &str) -> Result<(String, String), (usize, String)> {
    let address = address.trim();
    if address.is_empty() {
        return Err((0, "Enter the host's address, e.g. 192.168.1.20".into()));
    }
    let addr = match address.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') || host.starts_with('[') => {
            if host.is_empty() || port.parse::<u16>().map_or(true, |p| p == 0) {
                return Err((0, "Port must be a number between 1 and 65535".into()));
            }
            address.to_string()
        }
        _ => format!("{address}:{DEFAULT_PORT}"),
    };
    if addr.chars().any(char::is_whitespace) {
        return Err((0, "Addresses cannot contain spaces".into()));
    }
    let code = normalize_invite(code).ok_or((
        1,
        "Invite codes look like K7QF-M2XA (8 letters/digits)".into(),
    ))?;
    Ok((addr, code))
}

pub use crate::identity::normalize_invite;

#[cfg(test)]
mod tests;
