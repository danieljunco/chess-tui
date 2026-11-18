//! Direct host ⇄ peer transport. The listener runs inside the hosting
//! player's process. There is no separate server to deploy.

mod conn;
mod host;
mod peer;

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

pub use host::HostProvider;
pub use peer::PeerProvider;

pub use host::NetConfig;

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

#[cfg(test)]
mod tests;
