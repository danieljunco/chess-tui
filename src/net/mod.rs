//! Direct host ⇄ peer transport. The listener runs inside the hosting
//! player's process. There is no separate server to deploy.

mod conn;
mod host;
mod noise;
mod peer;

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::time::Duration;

pub use crate::identity::normalize_fingerprint;
pub use host::HostProvider;
pub use host::NetConfig;
pub use peer::PeerProvider;

pub(crate) fn advertise(bind: IpAddr) -> Vec<String> {
    match bind {
        IpAddr::V4(ip) if ip.is_unspecified() => {
            let mut ips = Vec::new();
            if let Some(found) = outbound_ip()
                && !found.is_loopback()
            {
                ips.push(found.to_string());
            }
            ips.push(Ipv4Addr::LOCALHOST.to_string());
            ips
        }
        other => vec![other.to_string()],
    }
}

fn outbound_ip() -> Option<IpAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    // TEST-NET. UDP connect picks the outbound interface and sends nothing.
    sock.connect("192.0.2.1:9").ok()?;
    Some(sock.local_addr().ok()?.ip())
}

/// Best-effort public IPv4 via a STUN binding request. This only reports the
/// address a router is using. It does not open a path through that router.
pub(crate) fn public_ipv4() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(800)))
        .ok()?;
    let mut request = [0u8; 20];
    request[1] = 0x01;
    request[4..8].copy_from_slice(&0x2112A442u32.to_be_bytes());
    getrandom::fill(&mut request[8..]).ok()?;
    sock.send_to(&request, "stun.l.google.com:19302").ok()?;
    let mut buf = [0u8; 512];
    let (n, _) = sock.recv_from(&mut buf).ok()?;
    stun_xor_ipv4(&buf[..n]).map(|ip| ip.to_string())
}

fn stun_xor_ipv4(buf: &[u8]) -> Option<Ipv4Addr> {
    if buf.len() < 20 {
        return None;
    }
    let mut i = 20;
    while i + 4 <= buf.len() {
        let kind = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        let value = i + 4;
        let padded = len.div_ceil(4) * 4;
        if value + len > buf.len() {
            break;
        }
        if kind == 0x0020 && buf.get(value + 1) == Some(&0x01) && len >= 8 {
            let cookie = 0x2112A442u32.to_be_bytes();
            let mut addr = [
                buf[value + 4],
                buf[value + 5],
                buf[value + 6],
                buf[value + 7],
            ];
            for (byte, mask) in addr.iter_mut().zip(cookie) {
                *byte ^= mask;
            }
            return Some(Ipv4Addr::from(addr));
        }
        i = value + padded;
    }
    None
}

#[cfg(test)]
mod tests;
