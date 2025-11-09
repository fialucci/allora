#![cfg(feature = "http")]
use allora::adapter::BaseAdapter;
use allora::{Exchange, HttpOutboundAdapter, Message, OutboundAdapter};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

// Simple capture server storing last body received.
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
    let addr: SocketAddr = "127.0.0.1:31110".parse().unwrap();
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
    let server = Server::bind(&addr).serve(make);
    let server_handle = tokio::spawn(server);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await; // wait for bind

    let adapter = HttpOutboundAdapter::builder()
        .host("127.0.0.1")
        .port(31110)
        .base_path("/")
        .id("out-test")
        .build()
        .unwrap();
    let mut ex = Exchange::new(Message::from_text("in"));
    ex.out_msg = Some(Message::from_text("out"));

    let res = adapter.dispatch(&ex).await.expect("dispatch");
    assert!(res.acknowledged);
    assert!(res.message.unwrap().starts_with("HTTP"));

    tokio::time::sleep(std::time::Duration::from_millis(50)).await; // ensure server processed
    assert_eq!(&*state.body.lock().unwrap(), b"out");

    server_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_outbound_falls_back_to_in_msg() {
    let state = CaptureState::new();
    let body_arc = state.body.clone();
    let addr: SocketAddr = "127.0.0.1:31111".parse().unwrap();
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
    let server = Server::bind(&addr).serve(make);
    let server_handle = tokio::spawn(server);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await; // wait for bind

    let adapter = HttpOutboundAdapter::builder()
        .host("127.0.0.1")
        .port(31111)
        .base_path("/")
        .build()
        .unwrap();
    let ex = Exchange::new(Message::from_text("only-in"));

    let res = adapter.dispatch(&ex).await.expect("dispatch");
    assert!(res.acknowledged);

    tokio::time::sleep(std::time::Duration::from_millis(50)).await; // ensure server processed
    assert_eq!(&*state.body.lock().unwrap(), b"only-in");

    server_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_outbound_staged_builder() {
    use allora::adapter::Adapter; // staged builder root
    let addr: SocketAddr = "127.0.0.1:32031".parse().unwrap();
    let make = make_service_fn(move |_conn| async move {
        Ok::<_, hyper::Error>(service_fn(move |req: Request<Body>| async move {
            let bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
            Ok::<_, hyper::Error>(Response::new(Body::from(bytes)))
        }))
    });
    let server_handle = tokio::spawn(Server::bind(&addr).serve(make));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let outbound = Adapter::outbound()
        .http()
        .host("127.0.0.1")
        .port(32031)
        .base_path("/")
        .id("staged-outbound")
        .build()
        .expect("build outbound");
    assert_eq!(outbound.id(), "staged-outbound");
    let mut ex = Exchange::new(Message::from_text("hello"));
    ex.out_msg = Some(Message::from_text("world"));
    let res = outbound.dispatch(&ex).await.expect("dispatch");
    assert!(res.acknowledged);
    server_handle.abort();
}
