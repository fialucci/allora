//! Integration tests for `HttpOutboundAdapter`.
//!
//! These tests exercise the public adapter surface — the builder, URL
//! validation, the plain-HTTP dispatch path, and (most importantly) the
//! **HTTPS dispatch path**, which is new in 0.0.9. They live in
//! `crates/http/tests/` so `cargo test -p allora-http` picks them up.

use allora_core::adapter::OutboundAdapter;
use allora_core::{Exchange, Message};
use allora_http::HttpOutboundAdapter;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::sync::{Arc, Mutex};

// ─────────────────────────────────────────────────────────────────────────
// Plain-HTTP dispatch — sanity check that the new `url:` schema still
// drives the basic happy path the previous host/port/base-path triple did.
// ─────────────────────────────────────────────────────────────────────────

struct CaptureState {
    body: Arc<Mutex<Vec<u8>>>,
}
impl CaptureState {
    fn new() -> Self {
        Self {
            body: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_outbound_posts_out_msg_if_present() {
    let state = CaptureState::new();
    let body_arc = state.body.clone();
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = std_listener.local_addr().unwrap().port();
    std_listener.set_nonblocking(true).expect("nonblocking");
    let make = make_service_fn(move |_conn| {
        let body_arc = body_arc.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let body_arc = body_arc.clone();
                async move {
                    let bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
                    *body_arc.lock().unwrap() = bytes.to_vec();
                    Ok::<_, hyper::Error>(Response::new(Body::from("ok")))
                }
            }))
        }
    });
    let server = Server::from_tcp(std_listener)
        .expect("hyper from_tcp")
        .serve(make);
    let server_handle = tokio::spawn(server);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let adapter = HttpOutboundAdapter::builder()
        .url(format!("http://127.0.0.1:{port}/"))
        .id("out-test")
        .build()
        .unwrap();
    let mut exchange = Exchange::new(Message::from_text("in"));
    exchange.out_msg = Some(Message::from_text("out"));

    let res = adapter.dispatch(&exchange).await.expect("dispatch");
    assert!(res.acknowledged);
    assert!(res.message.unwrap().starts_with("HTTP"));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(&*state.body.lock().unwrap(), b"out");

    server_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_outbound_falls_back_to_in_msg() {
    let state = CaptureState::new();
    let body_arc = state.body.clone();
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = std_listener.local_addr().unwrap().port();
    std_listener.set_nonblocking(true).expect("nonblocking");
    let make = make_service_fn(move |_conn| {
        let body_arc = body_arc.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                let body_arc = body_arc.clone();
                async move {
                    let bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
                    *body_arc.lock().unwrap() = bytes.to_vec();
                    Ok::<_, hyper::Error>(Response::new(Body::from("ok")))
                }
            }))
        }
    });
    let server = Server::from_tcp(std_listener)
        .expect("hyper from_tcp")
        .serve(make);
    let server_handle = tokio::spawn(server);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let adapter = HttpOutboundAdapter::builder()
        .url(format!("http://127.0.0.1:{port}/"))
        .build()
        .unwrap();
    let exchange = Exchange::new(Message::from_text("only-in"));

    let res = adapter.dispatch(&exchange).await.expect("dispatch");
    assert!(res.acknowledged);

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(&*state.body.lock().unwrap(), b"only-in");

    server_handle.abort();
}

// ─────────────────────────────────────────────────────────────────────────
// Builder-level URL validation — fail-fast guarantees.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn http_outbound_rejects_invalid_url() {
    let err = HttpOutboundAdapter::builder()
        .url(":::not a url")
        .build()
        .expect_err("builder should reject invalid url");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid url"),
        "expected invalid-url error, got: {msg}"
    );
}

#[test]
fn http_outbound_rejects_unsupported_scheme() {
    let err = HttpOutboundAdapter::builder()
        .url("ftp://example.com/")
        .build()
        .expect_err("builder should reject unsupported scheme");
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported url scheme"),
        "expected unsupported-scheme error, got: {msg}"
    );
}

#[test]
fn http_outbound_builder_caches_parsed_url() {
    let adapter = HttpOutboundAdapter::builder()
        .url("https://devnet.fialucci.org/oracle/submissions")
        .build()
        .expect("build succeeds");
    assert_eq!(adapter.url().scheme(), "https");
    assert_eq!(adapter.url().host_str(), Some("devnet.fialucci.org"));
    assert_eq!(adapter.url().path(), "/oracle/submissions");
}

// ─────────────────────────────────────────────────────────────────────────
// HTTPS roundtrip — proves the new code path actually negotiates TLS.
// ─────────────────────────────────────────────────────────────────────────
//
// Topology:
//   * Local TLS listener on 127.0.0.1:<ephemeral>, self-signed cert minted
//     at test time by `rcgen` for CN=localhost.
//   * One-shot accept loop reads the HTTP/1.1 request, captures the body,
//     and replies `HTTP/1.1 200 OK\r\n\r\nhttps-ok`.
//   * Client = the production `HttpOutboundAdapter`, with the
//     `dangerous_accept_invalid_certs` builder switch turned on so it
//     trusts the self-signed cert. Production callers never set this
//     switch — it has no spec field and the YAML parser cannot enable it.
//
// What this asserts:
//   * Adapter built with `url: https://...` actually dials TLS.
//   * The TLS handshake completes, the request is delivered, response body
//     comes back via the new `reqwest` code path.
//   * The body the server received is the body we sent over TLS.

mod https_roundtrip {
    use super::*;
    use std::sync::Arc as StdArc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;

    fn make_self_signed() -> (Vec<u8>, Vec<u8>) {
        // rcgen 0.13: `cert.der()` returns `&CertificateDer<'_>` (impls
        // `AsRef<[u8]>`), `key_pair.serialize_der()` returns a PKCS#8 DER
        // private key as `Vec<u8>`.
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .expect("rcgen self-signed");
        let cert_der = issued.cert.der().as_ref().to_vec();
        let key_der = issued.key_pair.serialize_der();
        (cert_der, key_der)
    }

    fn build_server_config(cert_der: Vec<u8>, key_der: Vec<u8>) -> rustls::ServerConfig {
        let cert_chain = vec![rustls::pki_types::CertificateDer::from(cert_der)];
        let priv_key = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(key_der),
        );
        // Install the ring crypto provider once per process (idempotent —
        // subsequent installs in the same process are a no-op error which
        // we swallow). Multiple tests in this binary can call this safely.
        let _ = rustls::crypto::ring::default_provider().install_default();
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, priv_key)
            .expect("rustls server config")
    }

    async fn serve_one(
        listener: TcpListener,
        acceptor: TlsAcceptor,
        captured: Arc<Mutex<Vec<u8>>>,
    ) {
        let (sock, _addr) = listener.accept().await.expect("accept");
        let mut tls = acceptor.accept(sock).await.expect("tls accept");
        // Drain headers + body. Single-request lifetime, small payloads.
        let mut buf = Vec::with_capacity(1024);
        let mut tmp = [0u8; 1024];
        loop {
            let n = tls.read(&mut tmp).await.expect("tls read");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(idx) = find_double_crlf(&buf) {
                let header_block = &buf[..idx];
                let content_length = parse_content_length(header_block).unwrap_or(0);
                let body_start = idx + 4;
                while buf.len() < body_start + content_length {
                    let n2 = tls.read(&mut tmp).await.expect("tls read body");
                    if n2 == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n2]);
                }
                let body = buf[body_start..body_start + content_length].to_vec();
                *captured.lock().unwrap() = body;
                break;
            }
        }
        let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\n\r\nhttps-ok";
        tls.write_all(resp).await.expect("tls write resp");
        tls.shutdown().await.ok();
    }

    fn find_double_crlf(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|w| w == b"\r\n\r\n")
    }

    fn parse_content_length(header_block: &[u8]) -> Option<usize> {
        let s = std::str::from_utf8(header_block).ok()?;
        for line in s.lines() {
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                return rest.trim().parse().ok();
            }
        }
        None
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_outbound_dispatches_over_https() {
        let (cert_der, key_der) = make_self_signed();
        let server_config = build_server_config(cert_der, key_der);
        let acceptor = TlsAcceptor::from(StdArc::new(server_config));

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind 0");
        let port = listener.local_addr().unwrap().port();

        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let captured_cl = captured.clone();
        let server_task =
            tokio::spawn(async move { serve_one(listener, acceptor, captured_cl).await });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let url = format!("https://localhost:{port}/oracle/submissions");
        let adapter = HttpOutboundAdapter::builder()
            .id("https-test")
            .url(&url)
            // Self-signed cert: opt into accepting it for this test only.
            .dangerous_accept_invalid_certs(true)
            .build()
            .expect("build https adapter");

        let mut exchange = Exchange::new(Message::from_text("ignored-in"));
        exchange.out_msg = Some(Message::from_text("https-payload"));

        let res = adapter.dispatch(&exchange).await.expect("https dispatch");
        assert!(
            res.acknowledged,
            "expected 2xx from local TLS server, got {:?}",
            res
        );
        assert_eq!(res.status_code, Some(200));
        assert_eq!(res.body.as_deref(), Some("https-ok"));

        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_task).await;
        let got = captured.lock().unwrap().clone();
        assert_eq!(
            got, b"https-payload",
            "server should have received the dispatched body via TLS"
        );
    }
}
