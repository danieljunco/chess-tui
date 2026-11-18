//! The hosting peer: listener, handshake, and the only task allowed to
//! commit moves.

use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender};
use tokio::time::{MissedTickBehavior, interval, sleep, timeout};
use tokio_util::codec::Framed;

use super::conn::{self, Conn, HANDSHAKE, PING_EVERY, codec};
use crate::app::{
    GameCommand, GameProvider, GameView, Link, ProviderEvent, Seat, Table, WaitingInfo,
};
use crate::domain::{Color, format_uci};
use crate::identity::new_invite;
use crate::protocol::{
    self, Frame, Phase as WirePhase, RefuseCode, RejectCode, Snapshot, WireOutcome,
};
use crate::session::{Applied, Fail, Intent, JoinError, Phase, Session, Side};

#[derive(Clone)]
pub struct NetConfig {
    pub bind: IpAddr,
    pub port: u16,
    pub color: Color,
    pub name: String,
    pub(crate) fail_delay: Duration,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            bind: IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            port: crate::app::DEFAULT_PORT,
            color: Color::White,
            name: "You".into(),
            fail_delay: Duration::from_secs(1),
        }
    }
}

enum Wake {
    Cmd(Option<GameCommand>),
    Line(conn::Line),
    Accept(io::Result<(TcpStream, std::net::SocketAddr)>),
    Tick,
}

enum Grant {
    New(crate::session::Joined),
    Back(Color),
}

struct Host {
    cfg: NetConfig,
    session: Session,
    conn: Option<Conn>,
    ev: UnboundedSender<ProviderEvent>,
    display: String,
    rtt: Option<u32>,
    ping_nonce: u64,
    pings: HashMap<u64, Instant>,
    next_id: u64,
    saw_peer: bool,
}

pub struct HostProvider {
    cmd_tx: Sender<GameCommand>,
    ev_rx: UnboundedReceiver<ProviderEvent>,
    task: Option<tokio::task::JoinHandle<()>>,
    runtime: Option<tokio::runtime::Runtime>,
}

impl HostProvider {
    pub fn start(cfg: NetConfig) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime");
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(32);
        let (ev_tx, ev_rx) = tokio::sync::mpsc::unbounded_channel();
        let task = runtime.spawn(run(cfg, cmd_rx, ev_tx));
        Self {
            cmd_tx,
            ev_rx,
            task: Some(task),
            runtime: Some(runtime),
        }
    }
}

impl Drop for HostProvider {
    fn drop(&mut self) {
        let _ = self.cmd_tx.try_send(GameCommand::Leave);
        if let (Some(task), Some(runtime)) = (self.task.take(), self.runtime.take()) {
            runtime.block_on(async {
                let _ = tokio::time::timeout(Duration::from_millis(800), task).await;
            });
        }
    }
}

impl GameProvider for HostProvider {
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
    cfg: NetConfig,
    mut cmd_rx: Receiver<GameCommand>,
    ev: UnboundedSender<ProviderEvent>,
) {
    let listener = match TcpListener::bind((cfg.bind, cfg.port)).await {
        Ok(listener) => listener,
        Err(err) => {
            let reason = if err.kind() == io::ErrorKind::AddrInUse {
                format!("Port {} is already in use", cfg.port)
            } else {
                format!("Could not listen on {}:{}", cfg.bind, cfg.port)
            };
            let _ = ev.send(ProviderEvent::Failed { reason });
            return;
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(cfg.port);
    let addresses = super::advertise(cfg.bind);
    let display = addresses
        .first()
        .map(|ip| format!("{ip}:{port}"))
        .unwrap_or_else(|| format!("127.0.0.1:{port}"));
    let invite = new_invite();
    let _ = ev.send(ProviderEvent::Waiting(WaitingInfo {
        addresses,
        port,
        invite: invite.clone(),
        seat: cfg.color,
    }));
    let mut host = Host {
        session: Session::new(cfg.color, &cfg.name, invite),
        cfg,
        conn: None,
        ev,
        display,
        rtt: None,
        ping_nonce: 0,
        pings: HashMap::new(),
        next_id: 1,
        saw_peer: false,
    };
    let mut ticker = interval(PING_EVERY);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ticker.tick().await;
    loop {
        match wait(&mut host, &mut cmd_rx, &listener, &mut ticker).await {
            Wake::Cmd(cmd) => {
                if host.on_command(cmd).await {
                    break;
                }
            }
            Wake::Line(line) => host.on_line(line).await,
            Wake::Accept(Ok((sock, _))) => host.handshake(sock).await,
            Wake::Accept(Err(_)) => {}
            Wake::Tick => host.on_tick().await,
        }
        if host.ev.is_closed() {
            break;
        }
    }
}

async fn wait(
    host: &mut Host,
    cmd_rx: &mut Receiver<GameCommand>,
    listener: &TcpListener,
    ticker: &mut tokio::time::Interval,
) -> Wake {
    if let Some(conn) = host.conn.as_mut() {
        tokio::select! {
            biased;
            cmd = cmd_rx.recv() => Wake::Cmd(cmd),
            line = conn.stream.next() => Wake::Line(line),
            accepted = listener.accept() => Wake::Accept(accepted),
            _ = ticker.tick() => Wake::Tick,
        }
    } else {
        tokio::select! {
            biased;
            cmd = cmd_rx.recv() => Wake::Cmd(cmd),
            accepted = listener.accept() => Wake::Accept(accepted),
            _ = ticker.tick() => Wake::Tick,
        }
    }
}

impl Host {
    async fn on_command(&mut self, cmd: Option<GameCommand>) -> bool {
        let Some(cmd) = cmd else {
            return true;
        };
        match cmd {
            GameCommand::Leave => {
                self.send(&Frame::HostClosing {
                    reason: "Host closed the game".into(),
                })
                .await;
                true
            }
            GameCommand::Move {
                from,
                to,
                promotion,
            } => {
                self.local(Intent::Move(format_uci(from, to, promotion)))
                    .await;
                false
            }
            GameCommand::Resign => {
                self.local(Intent::Resign).await;
                false
            }
            GameCommand::ClaimDraw => {
                self.local(Intent::ClaimDraw).await;
                false
            }
        }
    }

    async fn local(&mut self, intent: Intent) {
        let id = self.next_id;
        self.next_id += 1;
        let expected = self.session.version();
        let result = self.session.apply(Side::Host, id, expected, intent);
        if result.accepted {
            self.send_snapshot().await;
            self.emit_view();
        } else {
            let _ = self.ev.send(ProviderEvent::Rejected {
                reason: result.message,
            });
        }
    }

    async fn on_line(&mut self, line: conn::Line) {
        let Some(conn) = self.conn.as_mut() else {
            return;
        };
        conn::touch(conn);
        let Some(Ok(text)) = line else {
            self.drop_conn();
            return;
        };
        let Some(frame) = protocol::from_line(&text) else {
            self.drop_conn();
            return;
        };
        match frame {
            Frame::Move {
                request_id,
                expected_version,
                uci,
            } => {
                self.peer_intent(request_id, expected_version, Intent::Move(uci))
                    .await
            }
            Frame::Resign {
                request_id,
                expected_version,
            } => {
                self.peer_intent(request_id, expected_version, Intent::Resign)
                    .await
            }
            Frame::ClaimDraw {
                request_id,
                expected_version,
            } => {
                self.peer_intent(request_id, expected_version, Intent::ClaimDraw)
                    .await
            }
            Frame::Sync => self.send_snapshot().await,
            Frame::Ping { nonce } => self.send(&Frame::Pong { nonce }).await,
            Frame::Pong { nonce } => self.note_pong(nonce),
            Frame::Bye => self.drop_conn(),
            _ => self.drop_conn(),
        }
    }

    async fn peer_intent(&mut self, request_id: u64, expected: u64, intent: Intent) {
        let result = self.session.apply(Side::Peer, request_id, expected, intent);
        self.send(&result_frame(&result)).await;
        self.send_snapshot().await;
        if result.accepted {
            self.emit_view();
        }
    }

    async fn on_tick(&mut self) {
        if self.conn.as_ref().is_some_and(Conn::idle) {
            self.drop_conn();
            return;
        }
        if self.conn.is_none() {
            return;
        }
        self.ping_nonce = self.ping_nonce.wrapping_add(1);
        let nonce = self.ping_nonce;
        self.pings.insert(nonce, Instant::now());
        if self.pings.len() > 8 {
            self.pings.clear();
            self.pings.insert(nonce, Instant::now());
        }
        self.send(&Frame::Ping { nonce }).await;
    }

    fn note_pong(&mut self, nonce: u64) {
        let Some(sent) = self.pings.remove(&nonce) else {
            return;
        };
        self.rtt = Some(u32::try_from(sent.elapsed().as_millis()).unwrap_or(u32::MAX));
        self.emit_view();
    }

    async fn handshake(&mut self, sock: TcpStream) {
        let _ = sock.set_nodelay(true);
        let mut framed = Framed::new(sock, codec());
        let first = match timeout(HANDSHAKE, framed.next()).await {
            Ok(Some(Ok(line))) => line,
            _ => return,
        };
        let Some(frame) = protocol::from_line(&first) else {
            self.reply(
                &mut framed,
                &protocol::refuse(RefuseCode::Malformed, RefuseCode::Malformed.message()),
            )
            .await;
            return;
        };
        let grant = match frame {
            Frame::Hello {
                protocol_version,
                invite_code,
                name,
            } => {
                if protocol_version != protocol::PROTOCOL_VERSION {
                    self.reply(
                        &mut framed,
                        &protocol::refuse(RefuseCode::BadVersion, RefuseCode::BadVersion.message()),
                    )
                    .await;
                    return;
                }
                match self
                    .session
                    .join(&invite_code, name.as_deref().unwrap_or(""))
                {
                    Ok(joined) => Grant::New(joined),
                    Err(JoinError::BadInvite) => {
                        self.reply_later(
                            &mut framed,
                            &protocol::refuse(
                                RefuseCode::BadInvite,
                                RefuseCode::BadInvite.message(),
                            ),
                        )
                        .await;
                        return;
                    }
                    Err(JoinError::SeatTaken) => {
                        self.reply(
                            &mut framed,
                            &protocol::refuse(
                                RefuseCode::SeatTaken,
                                RefuseCode::SeatTaken.message(),
                            ),
                        )
                        .await;
                        return;
                    }
                    Err(JoinError::GameOver) => {
                        self.reply(
                            &mut framed,
                            &protocol::refuse(RefuseCode::GameOver, RefuseCode::GameOver.message()),
                        )
                        .await;
                        return;
                    }
                    Err(JoinError::BadToken) => return,
                }
            }
            Frame::Resume {
                protocol_version,
                seat_token,
            } => {
                if protocol_version != protocol::PROTOCOL_VERSION {
                    self.reply(
                        &mut framed,
                        &protocol::refuse(RefuseCode::BadVersion, RefuseCode::BadVersion.message()),
                    )
                    .await;
                    return;
                }
                match self.session.resume(&seat_token) {
                    Ok(seat) => Grant::Back(seat),
                    Err(_) => {
                        self.reply_later(
                            &mut framed,
                            &protocol::refuse(RefuseCode::BadToken, RefuseCode::BadToken.message()),
                        )
                        .await;
                        return;
                    }
                }
            }
            _ => {
                self.reply(
                    &mut framed,
                    &protocol::refuse(RefuseCode::Malformed, RefuseCode::Malformed.message()),
                )
                .await;
                return;
            }
        };
        self.conn = Some(Conn::from_framed(framed));
        self.saw_peer = true;
        self.rtt = None;
        let (seat, token) = match grant {
            Grant::New(joined) => (joined.seat, Some(joined.token)),
            Grant::Back(seat) => (seat, None),
        };
        self.send(&Frame::Welcome {
            seat: seat.into(),
            seat_token: token,
            host_name: self.session.host_name().to_string(),
        })
        .await;
        self.send_snapshot().await;
        self.emit_view();
    }

    async fn reply(&self, framed: &mut conn::Fr, frame: &Frame) {
        let line = protocol::to_line(frame);
        let _ = timeout(Duration::from_secs(2), framed.send(line)).await;
    }

    async fn reply_later(&self, framed: &mut conn::Fr, frame: &Frame) {
        sleep(self.cfg.fail_delay).await;
        self.reply(framed, frame).await;
    }

    async fn send(&mut self, frame: &Frame) {
        let ok = match self.conn.as_mut() {
            Some(conn) => conn.send(frame).await,
            None => return,
        };
        if !ok {
            self.drop_conn();
        }
    }

    async fn send_snapshot(&mut self) {
        let frame = self.snapshot_frame();
        self.send(&frame).await;
    }

    fn snapshot_frame(&self) -> Frame {
        let state = self.session.authoritative();
        let phase = if state.phase == Phase::Finished {
            WirePhase::Finished
        } else {
            WirePhase::Active
        };
        Frame::snapshot(Snapshot {
            version: state.version,
            phase,
            white: state.white,
            black: state.black,
            start_fen: state.start_fen,
            moves: state.moves,
            fen: state.fen,
            outcome: state.outcome.map(WireOutcome::from_domain),
            opponent_online: self.conn.is_some(),
        })
    }

    fn emit_view(&mut self) {
        if self.session.phase() == Phase::Waiting {
            return;
        }
        let link = if self.conn.is_some() {
            Link::Live { rtt_ms: self.rtt }
        } else {
            Link::OpponentOffline
        };
        let view = GameView {
            game: self.session.game().clone(),
            seat: Seat::Player(self.session.host_color()),
            white: self.session.white().to_string(),
            black: self.session.black().to_string(),
            table: Table::Hosting {
                addr: self.display.clone(),
            },
            link,
            version: self.session.version(),
        };
        let _ = self.ev.send(ProviderEvent::View(Box::new(view)));
    }

    fn drop_conn(&mut self) {
        self.conn = None;
        self.pings.clear();
        self.rtt = None;
        if self.saw_peer {
            self.emit_view();
        }
    }
}

fn result_frame(result: &Applied) -> Frame {
    if result.accepted {
        Frame::Accepted {
            request_id: result.request_id,
            version: result.version,
        }
    } else {
        Frame::Rejected {
            request_id: result.request_id,
            code: reject_code(result.fail),
            message: result.message.clone(),
            version: result.version,
        }
    }
}

fn reject_code(fail: Option<Fail>) -> RejectCode {
    match fail {
        Some(Fail::NotYourTurn) => RejectCode::NotYourTurn,
        Some(Fail::IllegalMove) => RejectCode::IllegalMove,
        Some(Fail::StaleVersion) => RejectCode::StaleVersion,
        Some(Fail::GameOver) => RejectCode::GameOver,
        Some(Fail::NotClaimable) => RejectCode::NotClaimable,
        Some(Fail::Malformed | Fail::NotStarted) | None => RejectCode::Malformed,
    }
}
