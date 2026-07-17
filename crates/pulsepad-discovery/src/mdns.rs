use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::Result;

const SERVICE_TYPE: &str = "_pulsepad._tcp";
const MDNS_PORT: u16 = 5353;
const DISCOVERY_PORT: u16 = 35769;

#[derive(Debug, Clone)]
pub struct MdnsResponder {
    service_name: String,
    device_id: String,
    cert_fingerprint: String,
    port: u16,
    running: Arc<RwLock<bool>>,
}

impl MdnsResponder {
    pub fn new(service_name: &str, device_id: &str, cert_fingerprint: &str, port: u16) -> Self {
        Self {
            service_name: service_name.to_string(),
            device_id: device_id.to_string(),
            cert_fingerprint: cert_fingerprint.to_string(),
            port,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;

        let service_name = self.service_name.clone();
        let device_id = self.device_id.clone();
        let fingerprint = self.cert_fingerprint.clone();
        let port = self.port;
        let running = self.running.clone();

        tokio::spawn(async move {
            let bind_addr: SocketAddr = format!("0.0.0.0:{}", MDNS_PORT)
                .parse()
                .unwrap_or_else(|_| ([0, 0, 0, 0], MDNS_PORT).into());

            let socket = match tokio::net::UdpSocket::bind(bind_addr).await {
                Ok(s) => s,
                Err(e) => {
                    error!("mDNS responder bind failed: {}", e);
                    return;
                }
            };
            socket.set_multicast_loop_v4(false).ok();
            socket.join_multicast_v4("224.0.0.251".parse().unwrap(), "0.0.0.0".parse().unwrap()).ok();

            let mut buf = vec![0u8; 4096];
            let txt = format!(
                "id={},fp={},port={},v=1.0",
                device_id, fingerprint, port
            );

            info!("mDNS responder started: {} on port {}", service_name, port);

            loop {
                if !*running.blocking_read() {
                    break;
                }

                match tokio::time::timeout(
                    std::time::Duration::from_secs(1),
                    socket.recv_from(&mut buf),
                )
                .await
                {
                    Ok(Ok((len, src))) => {
                        let data = &buf[..len];
                        // Check if it's a query for our service
                        if is_mdns_query(data, SERVICE_TYPE) {
                            debug!("mDNS query from {}", src);
                            if let Ok(response) = build_mdns_response(
                                &service_name,
                                &txt,
                                port,
                            ) {
                                socket.send_to(&response, src).await.ok();
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("mDNS recv error: {}", e);
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("mDNS responder stopped");
    }
}

#[derive(Debug, Clone)]
pub struct MdnsBrowser {
    discovered: Arc<RwLock<HashMap<String, DiscoveredMdnsService>>>,
    running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredMdnsService {
    pub name: String,
    pub device_id: String,
    pub address: String,
    pub port: u16,
    pub cert_fingerprint: String,
    pub txt_pairs: HashMap<String, String>,
    pub last_seen: u64,
}

impl MdnsBrowser {
    pub fn new() -> Self {
        Self {
            discovered: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;

        let discovered = self.discovered.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let bind_addr: SocketAddr = format!("0.0.0.0:{}", MDNS_PORT)
                .parse()
                .unwrap_or_else(|_| ([0, 0, 0, 0], MDNS_PORT).into());

            let socket = match tokio::net::UdpSocket::bind(bind_addr).await {
                Ok(s) => s,
                Err(e) => {
                    error!("mDNS browser bind failed: {}", e);
                    return;
                }
            };
            socket.set_multicast_loop_v4(false).ok();
            socket.join_multicast_v4("224.0.0.251".parse().unwrap(), "0.0.0.0".parse().unwrap()).ok();

            // Send initial query
            if let Ok(query) = build_mdns_query(SERVICE_TYPE) {
                let dest: std::net::SocketAddr = ([224, 0, 0, 251], 5353).into();
                socket.send_to(&query, dest).await.ok();
            }

            info!("mDNS browser started");

            loop {
                if !*running.blocking_read() {
                    break;
                }

                // Re-query every 30 seconds
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                if let Ok(query) = build_mdns_query(SERVICE_TYPE) {
                let dest: SocketAddr = ([224, 0, 0, 251], 5353).into();
                socket.send_to(&query, dest).await.ok();
                }
            }
        });

        // Also listen for unsolicited responses
        let discovered2 = self.discovered.clone();
        let running2 = self.running.clone();
        tokio::spawn(async move {
            let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(e) => {
                    error!("mDNS browser listener bind failed: {}", e);
                    return;
                }
            };
            socket.join_multicast_v4("224.0.0.251".parse().unwrap(), "0.0.0.0".parse().unwrap()).ok();

            let mut buf = vec![0u8; 4096];
            loop {
                if !*running2.blocking_read() {
                    break;
                }
                match tokio::time::timeout(
                    std::time::Duration::from_secs(1),
                    socket.recv_from(&mut buf),
                )
                .await
                {
                    Ok(Ok((len, src))) => {
                        if let Some(svc) = parse_mdns_response(&buf[..len], src, SERVICE_TYPE) {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            let mut map = discovered2.blocking_write();
                            map.insert(svc.name.clone(), DiscoveredMdnsService {
                                last_seen: now,
                                ..svc
                            });
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        *self.running.write().await = false;
        info!("mDNS browser stopped");
    }

    pub async fn list_services(&self) -> Vec<DiscoveredMdnsService> {
        self.discovered.read().await.values().cloned().collect()
    }

    pub async fn get_service(&self, name: &str) -> Option<DiscoveredMdnsService> {
        self.discovered.read().await.get(name).cloned()
    }
}

// --- Raw mDNS packet construction (minimal DNS-SD) ---

/// Build a DNS query for PTR records matching the given service type.
fn build_mdns_query(service_type: &str) -> Result<Vec<u8>> {
    let mut pkt = Vec::new();
    // DNS header: ID=0, flags=0x0100 (standard query), QDCOUNT=1
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    // Query: PTR for service type
    encode_dns_name(&mut pkt, service_type);
    pkt.extend_from_slice(&[0x00, 0x0C, 0x00, 0x01]); // QTYPE=PTR, QCLASS=IN
    Ok(pkt)
}

/// Check if a received packet is a query for our service type.
fn is_mdns_query(data: &[u8], service_type: &str) -> bool {
    if data.len() < 12 { return false; }
    // Simple check: look for the service type string in the packet
    let service_bytes = service_type.as_bytes();
    data.windows(service_bytes.len()).any(|w| w == service_bytes)
}

/// Build a DNS response announcing our service.
fn build_mdns_response(name: &str, txt: &str, port: u16) -> Result<Vec<u8>> {
    let mut pkt = Vec::new();
    // DNS header: ID=0, flags=0x8400 (response, authoritative), ANCOUNT=2 (SRV+TXT)
    pkt.extend_from_slice(&[0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00]);

    // PTR answer: instance._pulsepad._tcp.local -> service name
    let full_name = format!("{}.{}._pulsepad._tcp.local", name, name);
    encode_dns_name(&mut pkt, &format!("_pulsepad._tcp.local"));
    pkt.extend_from_slice(&[0x00, 0x0C, 0x00, 0x01]); // TYPE=PTR, CLASS=IN
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x78]); // TTL=120s
    let name_encoded = encode_dns_name_bytes(&full_name);
    let rdlength = name_encoded.len() as u16;
    pkt.extend_from_slice(&rdlength.to_be_bytes());
    pkt.extend_from_slice(&name_encoded);

    // SRV answer: hostname + port
    encode_dns_name(&mut pkt, &full_name);
    pkt.extend_from_slice(&[0x00, 0x21, 0x00, 0x01]); // TYPE=SRV, CLASS=IN
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x78]); // TTL=120s
    let srv_len = 6 + encode_dns_name_bytes(&format!("{}.local", name)).len();
    pkt.extend_from_slice(&(srv_len as u16).to_be_bytes());
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Priority=0, Weight=0
    pkt.extend_from_slice(&port.to_be_bytes());
    encode_dns_name(&mut pkt, &format!("{}.local", name));

    // TXT answer: key=value pairs
    encode_dns_name(&mut pkt, &full_name);
    pkt.extend_from_slice(&[0x00, 0x10, 0x00, 0x01]); // TYPE=TXT, CLASS=IN
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x78]); // TTL=120s
    let txt_bytes = txt.as_bytes();
    let txt_len = 1 + txt_bytes.len();
    pkt.extend_from_slice(&(txt_len as u16).to_be_bytes());
    pkt.push(txt_bytes.len() as u8);
    pkt.extend_from_slice(txt_bytes);

    Ok(pkt)
}

/// Parse a DNS response to extract service info.
fn parse_mdns_response(data: &[u8], src: SocketAddr, _service_type: &str) -> Option<DiscoveredMdnsService> {
    if data.len() < 12 { return None; }
    // Very minimal parser — extract port from SRV and txt from TXT records
    let mut name = String::new();
    let mut port = 0u16;
    let mut txt_pairs = HashMap::new();
    let mut device_id = String::new();
    let mut cert_fingerprint = String::new();

    // Skip header (12 bytes)
    let mut pos = 12;

    // Skip questions
    let qdcount = u16::from_be_bytes([data[4], data[5]]);
    for _ in 0..qdcount {
        pos += skip_dns_name(&data[pos..]);
        pos += 4; // QTYPE + QCLASS
    }

    // Parse answers
    let ancount = u16::from_be_bytes([data[6], data[7]]);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    for _ in 0..ancount {
        pos += skip_dns_name(&data[pos..]); // Name
        if pos + 10 > data.len() { break; }
        let rtype = u16::from_be_bytes([data[pos], data[pos+1]]);
        let _rclass = u16::from_be_bytes([data[pos+2], data[pos+3]]);
        let _ttl = u32::from_be_bytes([data[pos+4], data[pos+5], data[pos+6], data[pos+7]]);
        let rdlength = u16::from_be_bytes([data[pos+8], data[pos+9]]) as usize;
        pos += 10;
        if pos + rdlength > data.len() { break; }

        match rtype {
            0x21 => { // SRV
                if rdlength >= 6 {
                    port = u16::from_be_bytes([data[pos+4], data[pos+5]]);
                }
            }
            0x10 => { // TXT
                let txt_data = &data[pos..pos+rdlength];
                let mut txt_pos = 0;
                while txt_pos < txt_data.len() {
                    let len = txt_data[txt_pos] as usize;
                    txt_pos += 1;
                    if txt_pos + len > txt_data.len() { break; }
                    let entry = String::from_utf8_lossy(&txt_data[txt_pos..txt_pos+len]).to_string();
                    txt_pos += len;
                    if let Some(eq) = entry.find('=') {
                        let key = &entry[..eq];
                        let val = &entry[eq+1..];
                        match key {
                            "id" => device_id = val.to_string(),
                            "fp" => cert_fingerprint = val.to_string(),
                            "name" => name = val.to_string(),
                            _ => { txt_pairs.insert(key.to_string(), val.to_string()); }
                        }
                    }
                }
            }
            0x0C => { // PTR
                // Extract instance name from PTR target
                // (skip for now, we get name from TXT)
            }
            _ => {}
        }
        pos += rdlength;
    }

    if name.is_empty() {
        name = format!("PulsePad@{}", src.ip());
    }

    Some(DiscoveredMdnsService {
        name,
        device_id,
        address: src.ip().to_string(),
        port,
        cert_fingerprint,
        txt_pairs,
        last_seen: now,
    })
}

fn encode_dns_name(buf: &mut Vec<u8>, name: &str) {
    for part in name.trim_matches('.').split('.') {
        buf.push(part.len() as u8);
        buf.extend_from_slice(part.as_bytes());
    }
    buf.push(0);
}

fn encode_dns_name_bytes(name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    for part in name.trim_matches('.').split('.') {
        buf.push(part.len() as u8);
        buf.extend_from_slice(part.as_bytes());
    }
    buf.push(0);
    buf
}

fn skip_dns_name(data: &[u8]) -> usize {
    let mut pos = 0;
    loop {
        if pos >= data.len() { return pos; }
        let len = data[pos] as usize;
        if len == 0 { return pos + 1; }
        if len & 0xC0 == 0xC0 { return pos + 2; } // Compression pointer
        pos += 1 + len;
    }
}
