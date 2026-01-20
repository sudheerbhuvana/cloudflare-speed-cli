//! Traceroute functionality module
//!
//! Provides traceroute functionality to measure network path to Cloudflare edge.
//! Uses raw ICMP sockets when available (requires CAP_NET_RAW or root),
//! with fallback to system traceroute command.

use crate::model::{TestEvent, TracerouteHop, TracerouteSummary};
use anyhow::{Context, Result};
use pnet_packet::icmp::IcmpTypes;
use socket2::{Domain, Protocol, Socket, Type};
use std::io::ErrorKind;
use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Number of probes per hop
const PROBES_PER_HOP: usize = 3;

/// Timeout for each probe
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Run traceroute to the destination.
///
/// Tries raw ICMP first, falls back to system traceroute if that fails.
pub async fn run_traceroute(
    destination: &str,
    max_hops: u8,
    event_tx: &mpsc::Sender<TestEvent>,
) -> Result<TracerouteSummary> {
    // Resolve destination to IP
    let ip = resolve_destination(destination)?;

    // Try raw ICMP first
    match run_icmp_traceroute(&ip, max_hops, event_tx).await {
        Ok(summary) => return Ok(summary),
        Err(e) => {
            // Send info about fallback
            let _ = event_tx
                .send(TestEvent::Info {
                    message: format!("ICMP traceroute unavailable ({}), using system command", e),
                })
                .await;
        }
    }

    // Fall back to system traceroute
    run_system_traceroute(destination, &ip, max_hops, event_tx).await
}

/// Resolve destination hostname to IP address.
fn resolve_destination(destination: &str) -> Result<IpAddr> {
    // Try to parse as IP first
    if let Ok(ip) = destination.parse::<IpAddr>() {
        return Ok(ip);
    }

    // Try DNS resolution
    let addr = format!("{}:0", destination)
        .to_socket_addrs()
        .with_context(|| format!("Failed to resolve {}", destination))?
        .next()
        .ok_or_else(|| anyhow::anyhow!("No addresses found for {}", destination))?;

    Ok(addr.ip())
}

/// Run traceroute using raw ICMP sockets (requires elevated privileges).
async fn run_icmp_traceroute(
    destination: &IpAddr,
    max_hops: u8,
    event_tx: &mpsc::Sender<TestEvent>,
) -> Result<TracerouteSummary> {
    // Check if we're dealing with IPv4 - IPv6 traceroute is more complex
    let dest_v4 = match destination {
        IpAddr::V4(v4) => *v4,
        IpAddr::V6(_) => {
            return Err(anyhow::anyhow!(
                "IPv6 traceroute not yet supported via raw sockets"
            ));
        }
    };

    // Try to create raw ICMP socket
    let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
        .context("Failed to create raw ICMP socket (need CAP_NET_RAW or root)")?;

    socket.set_read_timeout(Some(PROBE_TIMEOUT))?;
    socket.set_nonblocking(false)?;

    let mut hops = Vec::new();
    let mut completed = false;

    for ttl in 1..=max_hops {
        socket.set_ttl(ttl as u32)?;

        let mut rtts = Vec::new();
        let mut hop_ip: Option<IpAddr> = None;
        let mut timeout = false;

        for probe_num in 0..PROBES_PER_HOP {
            let icmp_id = std::process::id() as u16;
            let icmp_seq = ((ttl as u16) << 8) | (probe_num as u16);

            // Build ICMP echo request packet
            let packet = build_icmp_packet(icmp_id, icmp_seq);

            let dest_addr = SocketAddr::new(IpAddr::V4(dest_v4), 0);

            let start = Instant::now();
            if socket.send_to(&packet, &dest_addr.into()).is_err() {
                continue;
            }

            // Wait for reply using MaybeUninit buffer
            let mut recv_buf: [MaybeUninit<u8>; 512] =
                unsafe { MaybeUninit::uninit().assume_init() };
            match socket.recv_from(&mut recv_buf) {
                Ok((len, from)) => {
                    let rtt = start.elapsed().as_secs_f64() * 1000.0;
                    rtts.push(rtt);

                    // Extract source IP from reply
                    let from_addr: SocketAddr = from.as_socket().unwrap_or(dest_addr);
                    if hop_ip.is_none() {
                        hop_ip = Some(from_addr.ip());
                    }

                    // Check if we've reached the destination
                    if from_addr.ip() == IpAddr::V4(dest_v4) {
                        completed = true;
                    }

                    // Check ICMP type to see if we should continue
                    if len >= 20 + 8 {
                        // IP header + ICMP header
                        // Safe to read since we received at least 28 bytes
                        let icmp_type = unsafe { recv_buf[20].assume_init() };
                        if icmp_type == IcmpTypes::EchoReply.0 {
                            completed = true;
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    timeout = true;
                }
                Err(_) => {
                    timeout = true;
                }
            }
        }

        let hop = TracerouteHop {
            hop_number: ttl,
            ip_address: hop_ip.map(|ip| ip.to_string()),
            hostname: hop_ip.and_then(|ip| resolve_hostname(&ip)),
            rtt_ms: rtts,
            timeout: timeout && hop_ip.is_none(),
        };

        // Send hop event
        let _ = event_tx
            .send(TestEvent::TracerouteHop {
                hop_number: ttl,
                hop: hop.clone(),
            })
            .await;

        hops.push(hop);

        if completed {
            break;
        }
    }

    Ok(TracerouteSummary {
        destination: destination.to_string(),
        hops,
        completed,
    })
}

/// Build an ICMP echo request packet.
fn build_icmp_packet(id: u16, seq: u16) -> Vec<u8> {
    let mut packet = vec![0u8; 64];

    // ICMP header
    packet[0] = IcmpTypes::EchoRequest.0; // Type
    packet[1] = 0; // Code
    packet[2] = 0; // Checksum (will be calculated)
    packet[3] = 0;
    packet[4] = (id >> 8) as u8; // Identifier
    packet[5] = (id & 0xff) as u8;
    packet[6] = (seq >> 8) as u8; // Sequence number
    packet[7] = (seq & 0xff) as u8;

    // Payload (timestamp and padding)
    for i in 8..64 {
        packet[i] = (i - 8) as u8;
    }

    // Calculate checksum
    let checksum = calculate_icmp_checksum(&packet);
    packet[2] = (checksum >> 8) as u8;
    packet[3] = (checksum & 0xff) as u8;

    packet
}

/// Calculate ICMP checksum.
fn calculate_icmp_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;

    while i < data.len() - 1 {
        sum += ((data[i] as u32) << 8) | (data[i + 1] as u32);
        i += 2;
    }

    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }

    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !sum as u16
}

/// Try to resolve an IP address to a hostname.
fn resolve_hostname(_ip: &IpAddr) -> Option<String> {
    // Skip hostname resolution for now to keep it simple
    // In production, we'd want async reverse DNS resolution
    None
}

/// Fall back to system traceroute command.
async fn run_system_traceroute(
    destination: &str,
    destination_ip: &IpAddr,
    max_hops: u8,
    event_tx: &mpsc::Sender<TestEvent>,
) -> Result<TracerouteSummary> {
    // Clone strings to avoid lifetime issues with spawn_blocking
    let dest = destination.to_string();
    let dest_ip_str = destination_ip.to_string();

    // Determine which command to use based on OS
    let (cmd, args): (&'static str, Vec<String>) = if cfg!(target_os = "windows") {
        (
            "tracert",
            vec![
                "-h".to_string(),
                max_hops.to_string(),
                "-d".to_string(),
                dest.clone(),
            ],
        )
    } else {
        (
            "traceroute",
            vec![
                "-m".to_string(),
                max_hops.to_string(),
                "-n".to_string(),
                "-q".to_string(),
                "3".to_string(),
                dest.clone(),
            ],
        )
    };

    let output = tokio::task::spawn_blocking(move || Command::new(cmd).args(&args).output())
        .await
        .context("Traceroute task failed")?
        .context("Failed to execute traceroute command")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let hops = parse_traceroute_output(&stdout, event_tx).await;

    let completed = hops
        .last()
        .map(|h| h.ip_address.as_deref() == Some(&dest_ip_str))
        .unwrap_or(false);

    Ok(TracerouteSummary {
        destination: destination.to_string(),
        hops,
        completed,
    })
}

/// Parse traceroute command output into hop structures.
async fn parse_traceroute_output(
    output: &str,
    event_tx: &mpsc::Sender<TestEvent>,
) -> Vec<TracerouteHop> {
    let mut hops = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // Skip header lines
        if line.is_empty()
            || line.starts_with("traceroute")
            || line.starts_with("Tracing")
            || line.contains("hops max")
        {
            continue;
        }

        // Parse hop line (format varies by OS)
        // Linux: " 1  192.168.1.1  0.123 ms  0.456 ms  0.789 ms"
        // macOS: " 1  192.168.1.1  0.123 ms  0.456 ms  0.789 ms"
        // Windows: "  1    <1 ms    <1 ms    <1 ms  192.168.1.1"

        if let Some(hop) = parse_hop_line(line) {
            let _ = event_tx
                .send(TestEvent::TracerouteHop {
                    hop_number: hop.hop_number,
                    hop: hop.clone(),
                })
                .await;
            hops.push(hop);
        }
    }

    hops
}

/// Parse a single hop line from traceroute output.
fn parse_hop_line(line: &str) -> Option<TracerouteHop> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // First part should be hop number
    let hop_number: u8 = parts.first()?.parse().ok()?;

    // Check for timeout line (all asterisks)
    if parts.iter().skip(1).all(|p| *p == "*") {
        return Some(TracerouteHop {
            hop_number,
            ip_address: None,
            hostname: None,
            rtt_ms: Vec::new(),
            timeout: true,
        });
    }

    // Find IP address and RTT values
    let mut ip_address: Option<String> = None;
    let mut rtts: Vec<f64> = Vec::new();

    for part in parts.iter().skip(1) {
        // Skip "ms" markers
        if *part == "ms" {
            continue;
        }

        // Try to parse as RTT (number)
        if let Ok(rtt) = part.trim_end_matches("ms").parse::<f64>() {
            rtts.push(rtt);
            continue;
        }

        // Handle Windows "<1 ms" format
        if part.starts_with('<') {
            if let Ok(rtt) = part
                .trim_start_matches('<')
                .trim_end_matches("ms")
                .parse::<f64>()
            {
                rtts.push(rtt);
                continue;
            }
        }

        // Try to parse as IP address
        if part.parse::<IpAddr>().is_ok() || (part.contains('.') && !part.contains("ms")) {
            ip_address = Some(part.to_string());
        }
    }

    if ip_address.is_none() && rtts.is_empty() {
        return None;
    }

    Some(TracerouteHop {
        hop_number,
        ip_address,
        hostname: None,
        rtt_ms: rtts,
        timeout: false,
    })
}
