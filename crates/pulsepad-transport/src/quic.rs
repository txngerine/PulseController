use async_trait::async_trait;
use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig};
use rcgen::{CertificateParams, KeyPair, SanType};
use rcgen::Error as RcgenError;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::convert::TryInto;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::error::{Result, TransportError};
use crate::traits::{Transport, TransportConfig, TransportState};

fn generate_self_signed_cert() -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    let keypair = KeyPair::generate()
        .map_err(|e| TransportError::Protocol(format!("QUIC keygen: {e}")))?;

    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "pulsepad.local");
    params.subject_alt_names = vec![
        SanType::DnsName("pulsepad.local".try_into()
            .map_err(|e: RcgenError| TransportError::Protocol(format!("Ia5String: {e}")))?),
    ];

    let cert = params
        .self_signed(&keypair)
        .map_err(|e| TransportError::Protocol(format!("QUIC self-sign: {e}")))?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivatePkcs8KeyDer::from(keypair.serialize_der());
    let key = PrivateKeyDer::Pkcs8(key_der);

    Ok((cert_der, key))
}

fn make_server_config() -> Result<ServerConfig> {
    let (cert, key) = generate_self_signed_cert()?;

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(|e| TransportError::Protocol(format!("TLS server: {e}")))?;

    let quic_config = QuicServerConfig::try_from(tls_config)
        .map_err(|e| TransportError::Protocol(format!("QUIC server config: {e}")))?;

    let mut transport_config = quinn::TransportConfig::default();
    transport_config.max_concurrent_bidi_streams(4u32.into());
    transport_config.max_concurrent_uni_streams(4u32.into());

    let mut config = ServerConfig::with_crypto(Arc::new(quic_config));
    config.transport_config(Arc::new(transport_config));
    Ok(config)
}

fn make_client_config() -> Result<ClientConfig> {
    let (cert, _key) = generate_self_signed_cert()?;

    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(cert)
        .map_err(|e| TransportError::Protocol(format!("add root cert: {e}")))?;

    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let quic_config = QuicClientConfig::try_from(tls_config)
        .map_err(|e| TransportError::Protocol(format!("QUIC client config: {e}")))?;

    let mut transport_config = quinn::TransportConfig::default();
    transport_config.max_concurrent_bidi_streams(4u32.into());
    transport_config.max_concurrent_uni_streams(4u32.into());

    let mut config = ClientConfig::new(Arc::new(quic_config));
    config.transport_config(Arc::new(transport_config));
    Ok(config)
}

#[derive(Debug)]
pub struct QuicTransport {
    config: TransportConfig,
    state: TransportState,
    endpoint: Option<Endpoint>,
    connection: Option<Arc<Mutex<Connection>>>,
    local_addr: Option<SocketAddr>,
}

impl QuicTransport {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: TransportState::Disconnected,
            endpoint: None,
            connection: None,
            local_addr: None,
        }
    }
}

#[async_trait]
impl Transport for QuicTransport {
    fn name(&self) -> &str {
        "QUIC"
    }

    fn state(&self) -> TransportState {
        self.state
    }

    fn config(&self) -> &TransportConfig {
        &self.config
    }

    async fn connect(&mut self, address: &str, port: u16) -> Result<()> {
        if self.state == TransportState::Connected {
            return Err(TransportError::AlreadyConnected);
        }
        self.state = TransportState::Connecting;
        info!("connecting QUIC to {address}:{port}");

        let bind_addr: SocketAddr = format!("{}:0", self.config.bind_address)
            .parse::<SocketAddr>()
            .map_err(|e: std::net::AddrParseError| TransportError::AddressParse(e.to_string()))?;

        let client_config = make_client_config()?;
        let mut endpoint = Endpoint::client(bind_addr)
            .map_err(|e| TransportError::Protocol(format!("QUIC endpoint: {e}")))?;
        endpoint.set_default_client_config(client_config);

        let remote: SocketAddr = format!("{address}:{port}")
            .parse::<SocketAddr>()
            .map_err(|e: std::net::AddrParseError| TransportError::AddressParse(e.to_string()))?;

        let connection = endpoint
            .connect(remote, "pulsepad.local")
            .map_err(|e| TransportError::Protocol(format!("QUIC connect: {e}")))?
            .await
            .map_err(|e| TransportError::Protocol(format!("QUIC handshake: {e}")))?;

        info!("QUIC connected to {address}:{port}");
        self.local_addr = connection.local_ip().map(|ip| SocketAddr::new(ip, 0));
        self.connection = Some(Arc::new(Mutex::new(connection)));
        self.endpoint = Some(endpoint);
        self.state = TransportState::Connected;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref conn) = self.connection {
            let c = conn.lock().await;
            c.close(0u32.into(), b"disconnect");
        }
        self.connection = None;
        self.endpoint = None;
        self.local_addr = None;
        self.state = TransportState::Disconnected;
        info!("QUIC disconnected");
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let conn = self
            .connection
            .as_ref()
            .ok_or(TransportError::NotConnected)?;
        let c = conn.lock().await;
        let mut stream = c
            .open_uni()
            .await
            .map_err(|e| TransportError::Send(format!("QUIC open stream: {e}")))?;
        stream
            .write_all(data)
            .await
            .map_err(|e| TransportError::Send(format!("QUIC write: {e}")))?;
        stream
            .finish()
            .map_err(|e| TransportError::Send(format!("QUIC finish: {e}")))?;
        Ok(data.len())
    }

    async fn receive(&mut self) -> Result<Vec<u8>> {
        let conn = self
            .connection
            .as_ref()
            .ok_or(TransportError::NotConnected)?;
        let c = conn.lock().await;
        let mut stream = c
            .accept_uni()
            .await
            .map_err(|e| TransportError::Receive(format!("QUIC accept stream: {e}")))?;
        let data = stream
            .read_to_end(self.config.buffer_size)
            .await
            .map_err(|e| TransportError::Receive(format!("QUIC read: {e}")))?;

        debug!("QUIC received {} bytes", data.len());
        Ok(data)
    }

    fn local_address(&self) -> Option<String> {
        self.local_addr.map(|a| a.to_string())
    }
}
