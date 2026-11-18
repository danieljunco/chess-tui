//! The joining peer. It dials the host directly, submits intentions, and
//! rebuilds the board from the host's snapshots.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::time::{MissedTickBehavior, interval, sleep, timeout};

use super::conn::{self, Conn, HANDSHAKE, PING_EVERY};
use crate::app::{GameCommand, GameProvider, GameView, Link, ProviderEvent, Seat, Table};
use crate::domain::{Color, format_uci};
use crate::protocol::{self, Frame};

enum Wake {
    Cmd(Option<GameCommand>),
    Line(conn::Line),
    Tick,
}

struct Peer {
    addr: String,
    code: String,
    name: String,
    ev: UnboundedSender<ProviderEvent>,
    conn: Option<Conn>,
    token: Option<String>,
    seat: Option<Color>,
    seen: u64,
    last: Option<GameView>,
    attempt: u32,
    backoff: Duration,
    stop: bool,
    bad_snaps: u32,
    ping_nonce: u64,
    pings: HashMap<u64, Instant>,
    rtt: Option<u32>,
    next_id: u64,
}

pub struct PeerProvider {
    cmd_tx: Sender<GameCommand>,
    ev_rx: UnboundedReceiver<ProviderEvent>,
    task: Option<tokio::task::JoinHandle<()>>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl PeerProvider {
    pub fn start(addr: String, code: String, name: String) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(32);
        let (ev_tx, ev_rx) = tokio::sync::mpsc::unbounded_channel();
        let task = runtime.spawn(run(addr, code, name, cmd_rx, ev_tx));
        Self {
            cmd_tx,
            ev_rx,
            task: Some(task),
            runtime: Some(runtime),
        }
    }
}

impl Drop for PeerProvider {
    fn drop(&mut self) {
        let _ = self.cmd_tx.try_send(GameCommand::Leave);
        if let (Some(task), Some(runtime)) = (self.task.take(), self.runtime.take()) {
            runtime.block_on(async {
                let _ = tokio::time::timeout(Duration::from_millis(800), task).await;
            });
        }
    }
}

impl GameProvider for PeerProvider {
    fn send(&mut self, command: GameCommand) {
        let _ = self.cmd_tx.try_send(command);
    }

    fn poll(&mut self, _now: Instant) -> Vec<ProviderEvent> {
        let mut out = Vec::new();
        while let Ok(event) = self.ev_rx.try_recv() {
            out.push(event);
        }
        out
    }
}

async fn run(
    addr: String,
    code: String,
    name: String,
    mut cmd_rx: Receiver<GameCommand>,
    ev: UnboundedSender<ProviderEvent>,
) {
    let mut peer = Peer {
        addr: addr.clone(),
        code,
        name,
        ev,
        conn: None,
        token: None,
        seat: None,
        seen: 0,
        last: None,
        attempt: 0,
        backoff: Duration::from_millis(500),
        stop: false,
        bad_snaps: 0,
        ping_nonce: 0,
        pings: HashMap::new(),
        rtt: None,
        next_id: 1,
    };
    let _ = peer
        .ev
        .send(ProviderEvent::Connecting { addr: addr.clone() });
    while !peer.stop {
        if peer.conn.is_none() {
            match peer.dial().await {
                Some(()) => {}
                None if peer.stop => break,
                None => {
                    peer.note_failure();
                    if peer.sleep_or_leave(&mut cmd_rx).await {
                        break;
                    }
                    continue;
                }
            }
        }
        if !peer.session(&mut cmd_rx).await {
            break;
        }
    }
}

impl Peer {
    async fn dial(&mut self) -> Option<()> {
        let sock = match timeout(HANDSHAKE, TcpStream::connect(&self.addr)).await {
            Ok(Ok(sock)) => sock,
            _ => return None,
        };
        let mut conn = Conn::new(sock);
        let hello = if let Some(token) = &self.token {
            Frame::Resume {
                protocol_version: protocol::PROTOCOL_VERSION,
                seat_token: token.clone(),
            }
        } else {
            Frame::Hello {
                protocol_version: protocol::PROTOCOL_VERSION,
                invite_code: self.code.clone(),
                name: Some(self.name.clone()),
            }
        };
        if !conn.send(&hello).await {
            return None;
        }
        let reply = match timeout(HANDSHAKE, conn.stream.next()).await {
            Ok(Some(Ok(line))) => protocol::from_line(&line)?,
            _ => return None,
        };
        match reply {
            Frame::Welcome {
                seat, seat_token, ..
            } => {
                if self.token.is_none() {
                    match seat_token {
                        Some(token) => self.token = Some(token),
                        None => {
                            self.fail("Host did not grant a seat");
                            return None;
                        }
                    }
                }
                self.seat = Some(seat.into());
                conn::touch(&mut conn);
                self.conn = Some(conn);
                self.backoff = Duration::from_millis(500);
                Some(())
            }
            Frame::Refused { message, code } => {
                let text = if message.is_empty() {
                    code.message().to_string()
                } else {
                    message
                };
                self.fail(text);
                None
            }
            _ => None,
        }
    }

    /// Returns true when the caller should stop for good.
    async fn sleep_or_leave(&mut self, cmd_rx: &mut Receiver<GameCommand>) -> bool {
        let pause = sleep(self.backoff);
        tokio::pin!(pause);
        loop {
            tokio::select! {
                biased;
                cmd = cmd_rx.recv() => match cmd {
                    Some(GameCommand::Leave) | None => {
                        self.stop = true;
                        return true;
                    }
                    Some(_) => {
                        let _ = self.ev.send(ProviderEvent::Rejected {
                            reason: "Connection lost · move not sent".into(),
                        });
                    }
                },
                () = &mut pause => return false,
            }
        }
    }

    /// Drive the live socket. `true` means the link dropped and we should redial.
    async fn session(&mut self, cmd_rx: &mut Receiver<GameCommand>) -> bool {
        let mut ticker = interval(PING_EVERY);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker.tick().await;
        loop {
            if self.stop {
                return false;
            }
            if self.conn.is_none() {
                return !self.stop;
            }
            match self.wait(cmd_rx, &mut ticker).await {
                Wake::Cmd(cmd) => {
                    if self.on_command(cmd).await {
                        return false;
                    }
                }
                Wake::Line(line) => {
                    if !self.on_line(line).await {
                        return !self.stop;
                    }
                }
                Wake::Tick => self.on_tick().await,
            }
        }
    }

    async fn wait(
        &mut self,
        cmd_rx: &mut Receiver<GameCommand>,
        ticker: &mut tokio::time::Interval,
    ) -> Wake {
        let conn = self.conn.as_mut().expect("live");
        tokio::select! {
            biased;
            cmd = cmd_rx.recv() => Wake::Cmd(cmd),
            line = conn.stream.next() => Wake::Line(line),
            _ = ticker.tick() => Wake::Tick,
        }
    }

    async fn on_command(&mut self, cmd: Option<GameCommand>) -> bool {
        let Some(cmd) = cmd else {
            return true;
        };
        match cmd {
            GameCommand::Leave => {
                self.send(&Frame::Bye).await;
                self.stop = true;
                true
            }
            GameCommand::Move {
                from,
                to,
                promotion,
            } => {
                let request_id = self.fresh_id();
                let expected_version = self.seen;
                self.submit(Frame::Move {
                    request_id,
                    expected_version,
                    uci: format_uci(from, to, promotion),
                })
                .await;
                false
            }
            GameCommand::Resign => {
                let request_id = self.fresh_id();
                let expected_version = self.seen;
                self.submit(Frame::Resign {
                    request_id,
                    expected_version,
                })
                .await;
                false
            }
            GameCommand::ClaimDraw => {
                let request_id = self.fresh_id();
                let expected_version = self.seen;
                self.submit(Frame::ClaimDraw {
                    request_id,
                    expected_version,
                })
                .await;
                false
            }
        }
    }

    fn fresh_id(&mut self) -> u64 {
        if self.seen == 0 {
            return 0;
        }
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    async fn submit(&mut self, frame: Frame) {
        if self.conn.is_none() || self.seen == 0 {
            let reason = if self.conn.is_none() {
                "Connection lost · move not sent"
            } else {
                "Waiting for the host"
            };
            let _ = self.ev.send(ProviderEvent::Rejected {
                reason: reason.into(),
            });
            return;
        }
        self.send(&frame).await;
    }

    /// `false` when the socket is gone.
    async fn on_line(&mut self, line: conn::Line) -> bool {
        let Some(conn) = self.conn.as_mut() else {
            return false;
        };
        conn::touch(conn);
        let Some(Ok(text)) = line else {
            self.conn = None;
            return false;
        };
        let Some(frame) = protocol::from_line(&text) else {
            self.conn = None;
            return false;
        };
        if let Some(snap) = frame.as_snapshot() {
            self.adopt(snap);
            return self.conn.is_some();
        }
        match frame {
            Frame::Rejected { message, .. } => {
                let _ = self.ev.send(ProviderEvent::Rejected { reason: message });
                true
            }
            Frame::Accepted { .. } => true,
            Frame::Ping { nonce } => {
                self.send(&Frame::Pong { nonce }).await;
                self.conn.is_some()
            }
            Frame::Pong { nonce } => {
                self.note_pong(nonce);
                true
            }
            Frame::HostClosing { .. } => {
                self.set_link(Link::HostClosed);
                self.stop = true;
                self.conn = None;
                false
            }
            _ => {
                self.conn = None;
                false
            }
        }
    }

    fn adopt(&mut self, snap: protocol::Snapshot) {
        if snap.version < self.seen {
            return;
        }
        if snap.version == self.seen && self.last.is_some() {
            self.set_link(Link::Live { rtt_ms: self.rtt });
            return;
        }
        let Some(seat) = self.seat else {
            return;
        };
        match snap.replay() {
            Some(game) => {
                self.seen = snap.version;
                self.bad_snaps = 0;
                let view = GameView {
                    version: snap.version,
                    game,
                    seat: Seat::Player(seat),
                    white: snap.white,
                    black: snap.black,
                    table: Table::Joined {
                        addr: self.addr.clone(),
                    },
                    link: Link::Live { rtt_ms: self.rtt },
                };
                self.last = Some(view.clone());
                let _ = self.ev.send(ProviderEvent::View(Box::new(view)));
            }
            None => {
                self.bad_snaps += 1;
                self.conn = None;
                if self.bad_snaps >= 3 {
                    self.fail("Host sent a position that doesn't match its moves");
                }
            }
        }
    }

    async fn on_tick(&mut self) {
        if self.conn.as_ref().is_some_and(Conn::idle) {
            self.conn = None;
            self.set_link(Link::Reconnecting {
                attempt: self.attempt.max(1),
            });
            return;
        }
        if self.conn.is_none() {
            return;
        }
        self.ping_nonce = self.ping_nonce.wrapping_add(1);
        let nonce = self.ping_nonce;
        self.pings.insert(nonce, Instant::now());
        self.send(&Frame::Ping { nonce }).await;
    }

    fn note_pong(&mut self, nonce: u64) {
        let Some(sent) = self.pings.remove(&nonce) else {
            return;
        };
        self.rtt = Some(u32::try_from(sent.elapsed().as_millis()).unwrap_or(u32::MAX));
        if self.last.is_some() {
            self.set_link(Link::Live { rtt_ms: self.rtt });
        }
    }

    async fn send(&mut self, frame: &Frame) {
        let ok = match self.conn.as_mut() {
            Some(conn) => conn.send(frame).await,
            None => return,
        };
        if !ok {
            self.conn = None;
        }
    }

    fn note_failure(&mut self) {
        self.attempt = self.attempt.saturating_add(1);
        self.backoff = (self.backoff * 2).min(Duration::from_secs(5));
        if self.last.is_some() {
            self.set_link(Link::Reconnecting {
                attempt: self.attempt,
            });
        }
    }

    fn set_link(&mut self, link: Link) {
        if let Some(view) = &mut self.last {
            view.link = link;
            let _ = self.ev.send(ProviderEvent::View(Box::new(view.clone())));
        } else if link == Link::HostClosed {
            self.fail("Host closed the game");
        }
    }

    fn fail(&mut self, reason: impl Into<String>) {
        self.stop = true;
        self.conn = None;
        let _ = self.ev.send(ProviderEvent::Failed {
            reason: reason.into(),
        });
    }
}
