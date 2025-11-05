#![cfg(feature = "http")]
//! HTTP Inbound Adapter: bridges incoming HTTP requests into Allora `Exchange`s.
//!
//! This module was renamed from `http_inbound` to `http_inbound_adapter` for consistency with
//! the test file naming (`http_inbound_adapter.rs`). The public type name `HttpInboundAdapter`
//! remains unchanged.
//!
//! # Overview
//! `HttpInboundAdapter` implements [`InboundAdapter`] and exposes two run styles:
//! * `serve(&self)`: continuous server until externally stopped.
//! * `run_once(self)`: handle exactly one request (useful for tests).
//!
//! Requests are converted to a `Message` with the raw UTF-8 body (lossy conversion for non-UTF-8) and basic headers:
//! * `http.method`
//! * `http.path`
//! * `http.is_get` (present only for GET)
//!
//! A correlation id is guaranteed via [`ensure_correlation`]; downstream processors and patterns (Aggregator, Splitter, request/reply) can rely on `correlation_id` being present.
//!
//! # Feature Gate
//! This module is compiled only when the `http` feature is enabled (requires `hyper`).
//!
//! # Example (single request)
//! ```no_run
//! use allora::{HttpInboundAdapter, InMemoryChannel, route::Route, processor::ClosureProcessor, Message};
//! use std::sync::Arc;
//! #[tokio::main]
//! async fn main() {
//!     let route = Route::with_correlation(None)
//!         .add(ClosureProcessor::new(|ex| {
//!             let echo = ex.in_msg.body_text().unwrap_or("");
//!             ex.out_msg = Some(Message::from_text(echo));
//!             Ok(())
//!         }))
//!         .build();
//!     let channel = Arc::new(InMemoryChannel::new(route));
//!     let adapter = HttpInboundAdapter::new("127.0.0.1:31001".parse().unwrap(), channel);
//!     // Serve exactly one request then exit (handy in integration tests).
//!     adapter.run_once().await.unwrap();
//! }
//! ```
//!
//! # Example (continuous)
//! ```no_run
//! use allora::{HttpInboundAdapter, InMemoryChannel, route::Route, processor::ClosureProcessor, Message};
//! use std::sync::Arc;
//! #[tokio::main]
//! async fn main() {
//!     let route = Route::with_correlation(None)
//!         .add(ClosureProcessor::new(|ex| { ex.out_msg = Some(Message::from_text("ok")); Ok(()) }))
//!         .build();
//!     let channel = Arc::new(InMemoryChannel::new(route));
//!     HttpInboundAdapter::new("127.0.0.1:8080".parse().unwrap(), channel).serve().await.unwrap();
//! }
//! ```
//!
//! # Error Handling
//! Any failure during request adaptation or channel dispatch results in a `500 Internal Error` response with a minimal body.
//! Future improvements may map specific `Error` variants to more granular HTTP statuses.
//!
//! # Future Extensions
//! * Header propagation
//! * Query parameter extraction
//! * Content-Type parsing
//! * Metrics & tracing middleware
//! * Graceful shutdown signals

use crate::adapter::{ensure_correlation, InboundAdapter};
use crate::{channel::ChannelRef, error::Result, Exchange, Message};
use async_trait::async_trait;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tracing::{error, info};

#[derive(Clone, Debug)]
pub struct HttpInboundAdapter {
    addr: SocketAddr,
    channel: ChannelRef,
}

pub struct HttpServerHandle {
    join: tokio::task::JoinHandle<Result<()>>,
}

impl HttpServerHandle {
    pub async fn wait(self) -> Result<()> {
        self.join
            .await
            .unwrap_or_else(|e| Err(crate::error::Error::other(e.to_string())))
    }
}

impl std::future::Future for HttpServerHandle {
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.join) };
        match inner.poll(cx) {
            Poll::Ready(r) => Poll::Ready(
                r.unwrap_or_else(|e| Err(crate::error::Error::other(e.to_string()))),
            ),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl HttpInboundAdapter {
    pub fn new(addr: SocketAddr, channel: ChannelRef) -> Self {
        Self { addr, channel }
    }
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn serve(&self) -> Result<()> {
        let channel = self.channel.clone();
        let make = make_service_fn(move |_conn| {
            let channel_clone = channel.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let c = channel_clone.clone();
                    async move {
                        match adapt_request(c, req).await {
                            Ok(resp) => Ok::<_, hyper::Error>(resp),
                            Err(e) => {
                                error!(error=%e, "request handling failed");
                                Ok(Response::builder()
                                    .status(500)
                                    .body(Body::from("internal error"))
                                    .unwrap())
                            }
                        }
                    }
                }))
            }
        });
        info!(address=%self.addr, "starting HTTP inbound adapter (continuous)");
        Server::bind(&self.addr)
            .serve(make)
            .await
            .map_err(|e| crate::error::Error::other(e.to_string()))?;
        Ok(())
    }

    pub async fn run_once(self) -> Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let tx_guard = Arc::new(Mutex::new(Some(tx)));
        let channel = self.channel.clone();
        let make = make_service_fn(move |_conn| {
            let channel_clone = channel.clone();
            let tx_clone = tx_guard.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| {
                    let c = channel_clone.clone();
                    let tx_local = tx_clone.clone();
                    async move {
                        let result = adapt_request(c, req).await;
                        match result {
                            Ok(resp) => {
                                if let Some(sender) = tx_local.lock().unwrap().take() {
                                    let _ = sender.send(());
                                }
                                Ok::<_, hyper::Error>(resp)
                            }
                            Err(e) => {
                                error!(error=%e, "request handling failed");
                                if let Some(sender) = tx_local.lock().unwrap().take() {
                                    let _ = sender.send(());
                                }
                                Ok(Response::builder()
                                    .status(500)
                                    .body(Body::from("internal error"))
                                    .unwrap())
                            }
                        }
                    }
                }))
            }
        });
        info!(address=%self.addr, "starting HTTP inbound adapter (single request)");
        Server::bind(&self.addr)
            .serve(make)
            .with_graceful_shutdown(async {
                let _ = rx.await;
            })
            .await
            .map_err(|e| crate::error::Error::other(e.to_string()))?;
        Ok(())
    }

    pub fn spawn_once(self) -> HttpServerHandle {
        HttpServerHandle {
            join: tokio::spawn(async move { self.run_once().await }),
        }
    }
    pub fn spawn_serve(self) -> HttpServerHandle {
        HttpServerHandle {
            join: tokio::spawn(async move { self.serve().await }),
        }
    }
}

#[async_trait]
impl InboundAdapter for HttpInboundAdapter {
    async fn run(&self) -> Result<()> {
        self.serve().await
    }
}

async fn adapt_request(channel: ChannelRef, req: Request<Body>) -> Result<Response<Body>> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let body_bytes = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(|e| crate::error::Error::other(e.to_string()))?;
    let text_body = String::from_utf8_lossy(&body_bytes).to_string();
    let mut msg = Message::from_text(text_body);
    msg.set_header("http.method", method.as_str());
    msg.set_header("http.path", path);
    if method == Method::GET {
        msg.set_header("http.is_get", "true");
    }
    let mut ex = Exchange::new(msg);
    ensure_correlation(&mut ex);
    #[cfg(feature = "async")]
    let processed = channel.dispatch_async(ex).await?;
    #[cfg(not(feature = "async"))]
    let processed = channel.dispatch(ex)?;
    let response_body = processed
        .out_msg
        .and_then(|m| m.body_text().map(|s| s.to_string()))
        .unwrap_or_default();
    Ok(Response::new(Body::from(response_body)))
}
