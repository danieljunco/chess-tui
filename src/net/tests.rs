use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use super::*;
use crate::app::{GameCommand, GameProvider, GameView, Link, ProviderEvent, Seat};
use crate::domain::{Color, Square};
use crate::protocol::{Frame, RefuseCode, WireColor, from_line, to_line};

fn cfg(color: Color, delay: Duration) -> NetConfig {
    NetConfig {
        bind: "127.0.0.1".parse().unwrap(),
        port: 0,
        color,
        name: "Ana".into(),
        fail_delay: delay,
    }
}

fn until<T>(mut poll: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(value) = poll() {
            return value;
        }
        if start.elapsed() > Duration::from_secs(4) {
            panic!("timed out");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_waiting(host: &mut HostProvider) -> crate::app::WaitingInfo {
    until(|| {
        for event in host.poll(Instant::now()) {
            match event {
                ProviderEvent::Waiting(info) => return Some(info),
                ProviderEvent::Failed { reason } => panic!("host failed: {reason}"),
                _ => {}
            }
        }
        None
    })
}

fn wait_view(provider: &mut dyn GameProvider, pred: impl Fn(&GameView) -> bool) -> GameView {
    until(|| {
        for event in provider.poll(Instant::now()) {
            match event {
                ProviderEvent::View(view) if pred(&view) => return Some(*view),
                ProviderEvent::Failed { reason } => panic!("failed: {reason}"),
                _ => {}
            }
        }
        None
    })
}

fn wait_reject(provider: &mut dyn GameProvider) -> String {
    until(|| {
        for event in provider.poll(Instant::now()) {
            if let ProviderEvent::Rejected { reason } = event {
                return Some(reason);
            }
        }
        None
    })
}

fn wait_failed(provider: &mut dyn GameProvider) -> String {
    until(|| {
        for event in provider.poll(Instant::now()) {
            if let ProviderEvent::Failed { reason } = event {
                return Some(reason);
            }
        }
        None
    })
}

fn write_frame(sock: &mut TcpStream, frame: &Frame) {
    let mut line = to_line(frame);
    line.push('\n');
    sock.write_all(line.as_bytes()).unwrap();
}

fn read_frame(sock: &mut TcpStream) -> Frame {
    sock.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        sock.read_exact(&mut byte).unwrap();
        if byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
        assert!(buf.len() < 70_000, "frame ran away");
    }
    from_line(std::str::from_utf8(&buf).unwrap()).unwrap()
}

#[test]
fn production_invite_penalty_is_one_second() {
    assert_eq!(NetConfig::default().fail_delay, Duration::from_secs(1));
}

#[test]
fn unknown_frame_fields_are_rejected() {
    assert!(from_line(r#"{"type":"ping","nonce":1,"extra":true}"#).is_none());
    assert!(from_line(r#"{"type":"nope"}"#).is_none());
}

#[test]
fn two_peers_sync_moves_and_reject_illegal_ones() {
    let mut host = HostProvider::start(cfg(Color::White, Duration::ZERO));
    let waiting = wait_waiting(&mut host);
    let mut peer = PeerProvider::start(
        format!("127.0.0.1:{}", waiting.port),
        waiting.invite,
        "Ben".into(),
    );
    let host_board = wait_view(&mut host, |v| v.version == 1);
    let peer_board = wait_view(&mut peer, |v| v.version == 1);
    assert_eq!(host_board.game.fen(), peer_board.game.fen());
    assert_eq!(peer_board.seat, Seat::Player(Color::Black));
    assert_eq!(host_board.link, Link::Live { rtt_ms: None });

    peer.send(GameCommand::Move {
        from: Square::E7,
        to: Square::E5,
        promotion: None,
    });
    assert!(wait_reject(&mut peer).contains("Not your turn"));

    host.send(GameCommand::Move {
        from: Square::E2,
        to: Square::E4,
        promotion: None,
    });
    let host_board = wait_view(&mut host, |v| v.version == 2);
    let peer_board = wait_view(&mut peer, |v| v.version == 2);
    assert_eq!(host_board.game.fen(), peer_board.game.fen());
    assert_eq!(host_board.game.history()[0].san, "e4");

    peer.send(GameCommand::Move {
        from: Square::E7,
        to: Square::E5,
        promotion: None,
    });
    let host_board = wait_view(&mut host, |v| v.version == 3);
    let peer_board = wait_view(&mut peer, |v| v.version == 3);
    assert_eq!(host_board.game.fen(), peer_board.game.fen());
    assert_eq!(peer_board.game.history().len(), 2);

    host.send(GameCommand::Move {
        from: Square::E2,
        to: Square::E4,
        promotion: None,
    });
    assert!(wait_reject(&mut host).contains("Illegal"));
    peer.send(GameCommand::Move {
        from: Square::E7,
        to: Square::E5,
        promotion: None,
    });
    assert!(wait_reject(&mut peer).contains("Not your turn"));
    host.send(GameCommand::Move {
        from: Square::D2,
        to: Square::D4,
        promotion: None,
    });
    let host_board = wait_view(&mut host, |v| v.version == 4);
    let peer_board = wait_view(&mut peer, |v| v.version == 4);
    assert_eq!(host_board.game.history().len(), 3);
    assert_eq!(host_board.game.fen(), peer_board.game.fen());
    assert_eq!(host_board.game.history()[2].san, "d4");
}

#[test]
fn bad_invite_is_delayed_and_does_not_take_the_seat() {
    let mut host = HostProvider::start(cfg(Color::White, Duration::from_millis(300)));
    let waiting = wait_waiting(&mut host);
    let started = Instant::now();
    let mut intruder = PeerProvider::start(
        format!("127.0.0.1:{}", waiting.port),
        "ZZZZ-ZZZZ".into(),
        "Nope".into(),
    );
    let reason = wait_failed(&mut intruder);
    assert!(
        started.elapsed() >= Duration::from_millis(250),
        "wrong codes are answered slowly"
    );
    assert!(reason.to_lowercase().contains("invite"));
    drop(intruder);

    let mut peer = PeerProvider::start(
        format!("127.0.0.1:{}", waiting.port),
        waiting.invite,
        "Ben".into(),
    );
    let view = wait_view(&mut host, |v| v.version == 1);
    let joined = wait_view(&mut peer, |v| v.version == 1);
    assert_eq!(view.black, "Ben");
    assert_eq!(joined.seat, Seat::Player(Color::Black));
}

#[test]
fn resume_needs_the_token_and_picks_up_moves() {
    let mut host = HostProvider::start(cfg(Color::White, Duration::ZERO));
    let waiting = wait_waiting(&mut host);
    let addr = format!("127.0.0.1:{}", waiting.port);
    let mut sock = TcpStream::connect(&addr).unwrap();
    write_frame(
        &mut sock,
        &Frame::Hello {
            protocol_version: 1,
            invite_code: waiting.invite.clone(),
            name: Some("Ben".into()),
        },
    );
    let token = match read_frame(&mut sock) {
        Frame::Welcome {
            seat: WireColor::Black,
            seat_token: Some(token),
            ..
        } => token,
        other => panic!("welcome: {other:?}"),
    };
    assert!(matches!(
        read_frame(&mut sock),
        Frame::Snapshot { version: 1, .. }
    ));
    drop(sock);

    let mut again = TcpStream::connect(&addr).unwrap();
    write_frame(
        &mut again,
        &Frame::Hello {
            protocol_version: 1,
            invite_code: waiting.invite,
            name: None,
        },
    );
    match read_frame(&mut again) {
        Frame::Refused {
            code: RefuseCode::SeatTaken,
            ..
        } => {}
        other => panic!("second hello: {other:?}"),
    }
    drop(again);

    host.send(GameCommand::Move {
        from: Square::E2,
        to: Square::E4,
        promotion: None,
    });
    let _ = wait_view(&mut host, |v| v.version == 2);

    let mut back = TcpStream::connect(&addr).unwrap();
    write_frame(
        &mut back,
        &Frame::Resume {
            protocol_version: 1,
            seat_token: token.clone(),
        },
    );
    match read_frame(&mut back) {
        Frame::Welcome {
            seat_token: None,
            seat: WireColor::Black,
            ..
        } => {}
        other => panic!("resume welcome: {other:?}"),
    }
    match read_frame(&mut back) {
        Frame::Snapshot {
            version: 2, moves, ..
        } => assert_eq!(moves, ["e2e4".to_string()]),
        other => panic!("resume snapshot: {other:?}"),
    }

    let leaked = format!("{:?}", host.poll(Instant::now()));
    assert!(!leaked.contains(&token), "seat token reached the host UI");
}

#[test]
fn duplicate_request_is_not_applied_twice() {
    let mut host = HostProvider::start(cfg(Color::Black, Duration::ZERO));
    let waiting = wait_waiting(&mut host);
    let mut sock = TcpStream::connect(format!("127.0.0.1:{}", waiting.port)).unwrap();
    write_frame(
        &mut sock,
        &Frame::Hello {
            protocol_version: 1,
            invite_code: waiting.invite,
            name: Some("Ben".into()),
        },
    );
    let _ = read_frame(&mut sock);
    let _ = read_frame(&mut sock);
    let move_frame = Frame::Move {
        request_id: 7,
        expected_version: 1,
        uci: "e2e4".into(),
    };
    write_frame(&mut sock, &move_frame);
    write_frame(&mut sock, &move_frame);
    match read_frame(&mut sock) {
        Frame::Accepted {
            request_id: 7,
            version: 2,
        } => {}
        other => panic!("first accept: {other:?}"),
    }
    match read_frame(&mut sock) {
        Frame::Snapshot {
            version: 2,
            ref moves,
            ..
        } => assert_eq!(moves, &["e2e4".to_string()]),
        other => panic!("snapshot: {other:?}"),
    }
    match read_frame(&mut sock) {
        Frame::Accepted {
            request_id: 7,
            version: 2,
        } => {}
        other => panic!("duplicate accept: {other:?}"),
    }
    match read_frame(&mut sock) {
        Frame::Snapshot {
            version: 2,
            ref moves,
            ..
        } => assert_eq!(moves.len(), 1),
        other => panic!("duplicate snapshot: {other:?}"),
    }
}

#[test]
fn host_quit_reaches_the_peer() {
    let mut host = HostProvider::start(cfg(Color::White, Duration::ZERO));
    let waiting = wait_waiting(&mut host);
    let mut peer = PeerProvider::start(
        format!("127.0.0.1:{}", waiting.port),
        waiting.invite,
        "Ben".into(),
    );
    let _ = wait_view(&mut peer, |v| v.version == 1);
    drop(host);
    let closed = wait_view(&mut peer, |v| v.link == Link::HostClosed);
    assert_eq!(closed.link, Link::HostClosed);
}

#[test]
fn oversized_frame_does_not_kill_the_table() {
    let mut host = HostProvider::start(cfg(Color::White, Duration::ZERO));
    let waiting = wait_waiting(&mut host);
    let addr = format!("127.0.0.1:{}", waiting.port);
    let mut sock = TcpStream::connect(&addr).unwrap();
    sock.set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let _ = sock.write_all(&vec![b'A'; 70_000]);
    drop(sock);

    let mut peer = PeerProvider::start(addr, waiting.invite, "Ben".into());
    let view = wait_view(&mut host, |v| v.version == 1);
    let _ = wait_view(&mut peer, |v| v.version == 1);
    assert_eq!(view.black, "Ben");
}
