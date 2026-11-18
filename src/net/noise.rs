//! Noise_XX under the JSON frames.
//!
//! The joining peer is the initiator. The host is the responder and keeps one
//! static key for the life of the process. The waiting screen shows a
//! fingerprint of that key. A peer that was given the fingerprint refuses any
//! other host.

use std::io;
#[cfg(test)]
use std::io::{Read, Write};
#[cfg(test)]
use std::net::TcpStream as StdStream;

use sha2::{Digest, Sha256};
use snow::params::NoiseParams;
use snow::{Builder, TransportState};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::identity::ct_eq;
use crate::protocol;

const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_SHA256";
const MAX_CIPHER: usize = protocol::MAX_FRAME + 64;

pub struct HostKey {
    private: Vec<u8>,
    pub fingerprint: String,
}

impl HostKey {
    pub fn generate() -> Self {
        let pair = Builder::new(params())
            .generate_keypair()
            .expect("noise keypair");
        Self {
            private: pair.private,
            fingerprint: fingerprint(&pair.public),
        }
    }
}

pub fn fingerprint(public_key: &[u8]) -> String {
    let digest = Sha256::digest(public_key);
    let hex: String = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02X}"))
        .collect();
    format!("{}-{}", &hex[..8], &hex[8..])
}

/// Sixteen hex digits, dashes and case ignored. Returns `XXXXXXXX-XXXXXXXX`.
pub fn normalize_fingerprint(raw: &str) -> Option<String> {
    let compact: String = raw
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect();
    if compact.len() == 16 && compact.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("{}-{}", &compact[..8], &compact[8..]))
    } else {
        None
    }
}

pub async fn accept(sock: &mut TcpStream, key: &HostKey) -> io::Result<TransportState> {
    let mut hs = responder(&key.private)?;
    let mut buf = vec![0u8; 65535];
    let msg = read_async(sock).await?;
    hs.read_message(&msg, &mut buf).map_err(noise_err)?;
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    write_async(sock, &buf[..n]).await?;
    let msg = read_async(sock).await?;
    hs.read_message(&msg, &mut buf).map_err(noise_err)?;
    hs.into_transport_mode().map_err(noise_err)
}

pub async fn initiate(sock: &mut TcpStream, expect: Option<&str>) -> io::Result<TransportState> {
    let pair = Builder::new(params())
        .generate_keypair()
        .map_err(noise_err)?;
    let mut hs = Builder::new(params())
        .local_private_key(&pair.private)
        .build_initiator()
        .map_err(noise_err)?;
    let mut buf = vec![0u8; 65535];
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    write_async(sock, &buf[..n]).await?;
    let msg = read_async(sock).await?;
    hs.read_message(&msg, &mut buf).map_err(noise_err)?;
    check_fingerprint(&hs, expect)?;
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    write_async(sock, &buf[..n]).await?;
    hs.into_transport_mode().map_err(noise_err)
}

#[cfg(test)]
pub fn accept_blocking(sock: &mut StdStream, key: &HostKey) -> io::Result<TransportState> {
    let mut hs = responder(&key.private)?;
    let mut buf = vec![0u8; 65535];
    let msg = read_blocking(sock)?;
    hs.read_message(&msg, &mut buf).map_err(noise_err)?;
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    write_blocking(sock, &buf[..n])?;
    let msg = read_blocking(sock)?;
    hs.read_message(&msg, &mut buf).map_err(noise_err)?;
    hs.into_transport_mode().map_err(noise_err)
}

#[cfg(test)]
pub fn initiate_blocking(sock: &mut StdStream, expect: Option<&str>) -> io::Result<TransportState> {
    let pair = Builder::new(params())
        .generate_keypair()
        .map_err(noise_err)?;
    let mut hs = Builder::new(params())
        .local_private_key(&pair.private)
        .build_initiator()
        .map_err(noise_err)?;
    let mut buf = vec![0u8; 65535];
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    write_blocking(sock, &buf[..n])?;
    let msg = read_blocking(sock)?;
    hs.read_message(&msg, &mut buf).map_err(noise_err)?;
    check_fingerprint(&hs, expect)?;
    let n = hs.write_message(&[], &mut buf).map_err(noise_err)?;
    write_blocking(sock, &buf[..n])?;
    hs.into_transport_mode().map_err(noise_err)
}

pub fn seal(transport: &mut TransportState, plain: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = vec![0u8; plain.len() + 64];
    let n = transport
        .write_message(plain, &mut out)
        .map_err(noise_err)?;
    out.truncate(n);
    Ok(out)
}

pub fn open(transport: &mut TransportState, cipher: &[u8]) -> io::Result<Vec<u8>> {
    let mut out = vec![0u8; cipher.len()];
    let n = transport
        .read_message(cipher, &mut out)
        .map_err(noise_err)?;
    out.truncate(n);
    Ok(out)
}

pub async fn send_plain(
    sock: &mut TcpStream,
    transport: &mut TransportState,
    plain: &[u8],
) -> io::Result<()> {
    let cipher = seal(transport, plain)?;
    write_async(sock, &cipher).await
}

pub async fn recv_plain(
    sock: &mut TcpStream,
    transport: &mut TransportState,
) -> io::Result<Vec<u8>> {
    let cipher = read_async(sock).await?;
    open(transport, &cipher)
}

#[cfg(test)]
pub fn send_plain_blocking(
    sock: &mut StdStream,
    transport: &mut TransportState,
    plain: &[u8],
) -> io::Result<()> {
    let cipher = seal(transport, plain)?;
    write_blocking(sock, &cipher)
}

#[cfg(test)]
pub fn recv_plain_blocking(
    sock: &mut StdStream,
    transport: &mut TransportState,
) -> io::Result<Vec<u8>> {
    let cipher = read_blocking(sock)?;
    open(transport, &cipher)
}

fn responder(private: &[u8]) -> io::Result<snow::HandshakeState> {
    Builder::new(params())
        .local_private_key(private)
        .build_responder()
        .map_err(noise_err)
}

fn check_fingerprint(hs: &snow::HandshakeState, expect: Option<&str>) -> io::Result<()> {
    let Some(expect) = expect else {
        return Ok(());
    };
    let want = normalize_fingerprint(expect).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "host key fingerprint is malformed",
        )
    })?;
    let remote = hs
        .get_remote_static()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "host did not present a key"))?;
    let got = fingerprint(remote);
    if ct_eq(&want, &got) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "host key does not match",
        ))
    }
}

fn params() -> NoiseParams {
    PATTERN.parse().expect("noise pattern")
}

fn noise_err(err: snow::Error) -> io::Error {
    io::Error::other(err)
}

async fn write_async(sock: &mut TcpStream, msg: &[u8]) -> io::Result<()> {
    let len = u32::try_from(msg.len()).unwrap_or(u32::MAX);
    sock.write_all(&len.to_be_bytes()).await?;
    sock.write_all(msg).await?;
    sock.flush().await
}

async fn read_async(sock: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_CIPHER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame length"));
    }
    let mut buf = vec![0u8; len];
    sock.read_exact(&mut buf).await?;
    Ok(buf)
}

#[cfg(test)]
fn write_blocking(sock: &mut StdStream, msg: &[u8]) -> io::Result<()> {
    let len = u32::try_from(msg.len()).unwrap_or(u32::MAX);
    sock.write_all(&len.to_be_bytes())?;
    sock.write_all(msg)?;
    sock.flush()
}

#[cfg(test)]
fn read_blocking(sock: &mut StdStream) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_CIPHER {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame length"));
    }
    let mut buf = vec![0u8; len];
    sock.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn handshake_roundtrip_and_bad_fingerprint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let key = HostKey::generate();
        let fp = key.fingerprint.clone();
        let server = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            sock.set_nodelay(true).unwrap();
            let mut transport = accept_blocking(&mut sock, &key).expect("accept");
            let msg = recv_plain_blocking(&mut sock, &mut transport).expect("server read");
            assert_eq!(msg, b"hello");
            send_plain_blocking(&mut sock, &mut transport, b"ok").expect("server write");
        });
        let mut sock = StdStream::connect(addr).unwrap();
        sock.set_nodelay(true).unwrap();
        let mut transport = initiate_blocking(&mut sock, Some(&fp)).expect("initiate");
        send_plain_blocking(&mut sock, &mut transport, b"hello").unwrap();
        let msg = recv_plain_blocking(&mut sock, &mut transport).unwrap();
        assert_eq!(msg, b"ok");
        server.join().unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let key = HostKey::generate();
        let server = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let _ = accept_blocking(&mut sock, &key);
        });
        let mut sock = StdStream::connect(addr).unwrap();
        let err = initiate_blocking(&mut sock, Some("01234567-89ABCDEF")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        drop(sock);
        let _ = server.join();
    }
}
