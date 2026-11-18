//! One direct TCP connection. Frames are newline-delimited JSON, capped at 64 KiB.

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_util::codec::{Framed, LinesCodec};

use crate::protocol::{self, Frame};

pub type Fr = Framed<TcpStream, LinesCodec>;
pub type Line = Option<Result<String, tokio_util::codec::LinesCodecError>>;

pub const PING_EVERY: Duration = Duration::from_secs(2);
pub const DEAD_AFTER: Duration = Duration::from_secs(6);
pub const HANDSHAKE: Duration = Duration::from_secs(5);
const WRITE_LIMIT: Duration = Duration::from_secs(2);

pub struct Conn {
    sink: futures_util::stream::SplitSink<Fr, String>,
    pub stream: futures_util::stream::SplitStream<Fr>,
    pub last_rx: Instant,
}

impl Conn {
    pub fn new(sock: TcpStream) -> Self {
        let _ = sock.set_nodelay(true);
        Self::from_framed(Framed::new(sock, codec()))
    }

    pub fn from_framed(framed: Fr) -> Self {
        let (sink, stream) = framed.split();
        Self {
            sink,
            stream,
            last_rx: Instant::now(),
        }
    }

    pub async fn send(&mut self, frame: &Frame) -> bool {
        let line = protocol::to_line(frame);
        matches!(timeout(WRITE_LIMIT, self.sink.send(line)).await, Ok(Ok(())))
    }

    pub fn idle(&self) -> bool {
        self.last_rx.elapsed() > DEAD_AFTER
    }
}

pub fn codec() -> LinesCodec {
    LinesCodec::new_with_max_length(protocol::MAX_FRAME)
}

pub fn touch(conn: &mut Conn) {
    conn.last_rx = Instant::now();
}
