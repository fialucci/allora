use allora::{Exchange, Message, OutboundAdapter, Result, Runtime};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio::sync::{oneshot, Mutex};

fn spawn_server_shutdown_after_first(addr: &str) -> oneshot::Receiver<()> {
    let socket: SocketAddr = addr.parse().expect("valid socket addr");
    let (tx_internal, rx_internal) = oneshot::channel();
    let (tx_external, rx_external) = oneshot::channel();
    let trigger = Arc::new(Mutex::new(Some((tx_internal, tx_external))));
    let make_svc = {
        let trigger = trigger.clone();
        make_service_fn(move |_| {
            let trigger = trigger.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let trigger = trigger.clone();
                    async move {
                        let bytes = hyper::body::to_bytes(req.into_body())
                            .await
                            .unwrap_or_default();
                        println!(
                            "server received payload='{}'",
                            String::from_utf8_lossy(&bytes).trim()
                        );
                        let resp = Response::builder()
                            .status(202)
                            .body(Body::from("accepted"))
                            .unwrap();
                        if let Some((tx_int, tx_ext)) = trigger.lock().await.take() {
                            let _ = tx_int.send(());
                            let _ = tx_ext.send(());
                        }
                        Ok::<_, Infallible>(resp)
                    }
                }))
            }
        })
    };
    let server = Server::bind(&socket).serve(make_svc);
    let graceful = server.with_graceful_shutdown(async move {
        let _ = rx_internal.await;
    });
    tokio::spawn(graceful);
    rx_external
}

#[tokio::main]
async fn main() -> Result<()> {
    let rx = spawn_server_shutdown_after_first("127.0.0.1:18080");
    let result = Runtime::new()
        .with_config_file("examples/basic/http-outbound/allora.yml")
        .run()?
        .http_outbound_adapters()
        .first()
        .expect("adapter")
        .adapter()
        .dispatch(&Exchange::new(Message::from_text("ping")))
        .await?;
    let _ = rx.await;
    println!(
        "status={} body={}",
        result.status_code.unwrap_or(0),
        result.body.unwrap_or_default()
    );
    Ok(())
}
