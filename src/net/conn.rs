//! One direct TCP connection. A Noise handshake runs first. After that every
//! JSON frame is one length-prefixed ciphertext.

use std::io;
use std::time::{Duration, Instant};

use snow::TransportState;
use tokio::net::TcpStream;
use tokio::time::timeout;

use super::noise::{self, HostKey};
use crate::protocol::{self, Frame};

pub type Line = Option<Result<String, io::Error>>;

pub const PING_EVERY: Duration = Duration::from_secs(2);
pub const DEAD_AFTER: Duration = Duration::from_secs(6);
pub const HANDSHAKE: Duration = Duration::from_secs(5);
const WRITE_LIMIT: Duration = Duration::from_secs(2);

pub struct Conn {
    sock: TcpStream,
    noise: TransportState,
    pub last_rx: Instant,
}

impl Conn {
    pub async fn accept(mut sock: TcpStream, key: &HostKey) -> io::Result<Self> {
        let _ = sock.set_nodelay(true);
        let noise = noise::accept(&mut sock, key).await?;
        Ok(Self {
            sock,
            noise,
            last_rx: Instant::now(),
        })
    }

    pub async fn initiate(mut sock: TcpStream, expect: Option<&str>) -> io::Result<Self> {
        let _ = sock.set_nodelay(true);
        let noise = noise::initiate(&mut sock, expect).await?;
        Ok(Self {
            sock,
            noise,
            last_rx: Instant::now(),
        })
    }

    pub async fn send(&mut self, frame: &Frame) -> bool {
        let line = protocol::to_line(frame);
        matches!(
            timeout(
                WRITE_LIMIT,
                noise::send_plain(&mut self.sock, &mut self.noise, line.as_bytes())
            )
            .await,
            Ok(Ok(()))
        )
    }

    pub async fn recv(&mut self) -> Line {
        match noise::recv_plain(&mut self.sock, &mut self.noise).await {
            Ok(bytes) => Some(
                String::from_utf8(bytes)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err)),
            ),
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => None,
            Err(err) => Some(Err(err)),
        }
    }

    pub fn idle(&self) -> bool {
        self.last_rx.elapsed() > DEAD_AFTER
    }
}

pub fn touch(conn: &mut Conn) {
    conn.last_rx = Instant::now();
}
