use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::{Context, Result, anyhow};
use tokio::net::UdpSocket;

use crate::config::DeviceConfig;

pub async fn wake_device(device: &DeviceConfig) -> Result<()> {
    let mac = parse_mac(&device.mac)?;
    let packet = build_magic_packet(&mac);
    let target = resolve_target(&device.host, device.port)?;

    let socket = if target.is_ipv4() {
        UdpSocket::bind("0.0.0.0:0").await?
    } else {
        UdpSocket::bind("[::]:0").await?
    };
    socket.set_broadcast(true)?;
    socket
        .send_to(&packet, target)
        .await
        .with_context(|| format!("failed to send WOL packet to {target}"))?;

    Ok(())
}

fn resolve_target(host: &str, port: u16) -> Result<SocketAddr> {
    (host, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("unable to resolve {host}:{port}"))
}

fn parse_mac(input: &str) -> Result<[u8; 6]> {
    let normalized = input.replace([':', '-'], "");
    if normalized.len() != 12 {
        return Err(anyhow!("invalid MAC address: {input}"));
    }

    let mut bytes = [0u8; 6];
    for (index, chunk) in normalized.as_bytes().chunks_exact(2).enumerate() {
        let part = std::str::from_utf8(chunk)?;
        bytes[index] =
            u8::from_str_radix(part, 16).with_context(|| format!("invalid MAC byte: {part}"))?;
    }

    Ok(bytes)
}

fn build_magic_packet(mac: &[u8; 6]) -> [u8; 102] {
    let mut packet = [0u8; 102];
    packet[..6].fill(0xFF);
    for offset in 0..16 {
        let start = 6 + (offset * 6);
        packet[start..start + 6].copy_from_slice(mac);
    }
    packet
}

#[cfg(test)]
mod tests {
    use super::{build_magic_packet, parse_mac};

    #[test]
    fn parses_mac() {
        let mac = parse_mac("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(mac, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn builds_magic_packet() {
        let mac = [1, 2, 3, 4, 5, 6];
        let packet = build_magic_packet(&mac);
        assert_eq!(&packet[..6], &[0xFF; 6]);
        assert_eq!(&packet[6..12], &mac);
        assert_eq!(&packet[96..102], &mac);
    }
}
