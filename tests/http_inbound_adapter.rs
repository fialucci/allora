#![cfg(feature = "http")]
use allora::adapter::Adapter;
use allora::channel::ChannelBuilder;
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_injects_correlation_and_echoes_it() {
    use allora::OutboundQueue;
    // Pure pipe channel (no internal route processing)
    let channel = Arc::new(ChannelBuilder::point_to_point().in_memory().build());
    let addr: SocketAddr = "127.0.0.1:31002".parse().unwrap();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(31002)
        .channel(channel.clone())
        .build();
    let handle = adapter.spawn_once();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://{}", addr))
        .header("Content-Type", "text/plain")
        .body(hyper::Body::from("test"))
        .unwrap();
    let resp = client.request(req).await.expect("http response");
    assert_eq!(resp.status(), 200);
    let bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&bytes[..], b"test"); // echo behavior
                                     // Inspect queued exchange for correlation headers
    let ex = channel.try_receive_async().await.expect("queued exchange");
    let cid = ex
        .in_msg
        .header("correlation_id")
        .expect("correlation_id header");
    assert_eq!(cid.len(), 36);
    // minimal UUID v4 check: hyphen positions
    for (i, ch) in cid.chars().enumerate() {
        if [8, 13, 18, 23].contains(&i) {
            assert_eq!(ch, '-');
        } else {
            assert!(ch.is_ascii_hexdigit());
        }
    }
    handle.await.expect("server completed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_handle_wait_method() {
    let channel = Arc::new(ChannelBuilder::point_to_point().in_memory().build());
    let addr: SocketAddr = "127.0.0.1:31003".parse().unwrap();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(31003)
        .channel(channel.clone())
        .build();
    let handle = adapter.spawn_once();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://{}", addr))
        .body(hyper::Body::from("ignored"))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&body[..], b"ignored"); // echo
    handle.await.expect("server completed via wait method");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_staged_builder() {
    let channel = Arc::new(ChannelBuilder::point_to_point().in_memory().build());
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(32030)
        .base_path("/")
        .channel(channel.clone())
        .id("staged-inbound")
        .build();
    assert_eq!(adapter.id(), "staged-inbound");
    let handle = adapter.clone().spawn_once();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("http://127.0.0.1:32030")
        .body(hyper::Body::from("ping"))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&body[..], b"ping"); // echo
    handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_multi_endpoint_registration() {
    use hyper::Body;
    let channel1 = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .id("chan1")
            .build(),
    );
    let channel2 = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .id("chan2")
            .build(),
    );
    let ep1 = allora::endpoint::EndpointBuilder::in_out()
        .in_memory()
        .channel(channel1.clone())
        .build();
    let ep2 = allora::endpoint::EndpointBuilder::in_out()
        .in_memory()
        .channel(channel2.clone())
        .build();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(33001)
        .channel(channel1.clone())
        .register("POST", "/echo1", ep1.clone())
        .register("POST", "/echo2", ep2.clone())
        .build();
    let handle = adapter.spawn_serve();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let client = hyper::Client::new();
    let req1 = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("http://127.0.0.1:33001/echo1")
        .body(Body::from("x"))
        .unwrap();
    let resp1 = client.request(req1).await.unwrap();
    assert_eq!(resp1.status(), 200);
    let b1 = hyper::body::to_bytes(resp1.into_body()).await.unwrap();
    assert_eq!(&b1[..], b"x"); // echo
    let req2 = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("http://127.0.0.1:33001/echo2")
        .body(Body::from("y"))
        .unwrap();
    let resp2 = client.request(req2).await.unwrap();
    assert_eq!(resp2.status(), 200);
    let b2 = hyper::body::to_bytes(resp2.into_body()).await.unwrap();
    assert_eq!(&b2[..], b"y"); // echo
    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_method_discrimination() {
    use hyper::Body;
    let channel_post = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .id("chanP")
            .build(),
    );
    let channel_get = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .id("chanG")
            .build(),
    );
    let ep_post = allora::endpoint::EndpointBuilder::in_out()
        .in_memory()
        .channel(channel_post.clone())
        .build();
    let ep_get = allora::endpoint::EndpointBuilder::in_out()
        .in_memory()
        .channel(channel_get.clone())
        .build();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(33002)
        .channel(channel_post.clone())
        .register("POST", "/method", ep_post.clone())
        .register("GET", "/method", ep_get.clone())
        .build();
    let handle = adapter.spawn_serve();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let client = hyper::Client::new();
    let req_post = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("http://127.0.0.1:33002/method")
        .body(Body::from("p"))
        .unwrap();
    let resp_post = client.request(req_post).await.unwrap();
    assert_eq!(resp_post.status(), 200);
    let body_post = hyper::body::to_bytes(resp_post.into_body()).await.unwrap();
    assert_eq!(&body_post[..], b"p"); // echo
    let req_get = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri("http://127.0.0.1:33002/method")
        .body(Body::from("g"))
        .unwrap();
    let resp_get = client.request(req_get).await.unwrap();
    assert_eq!(resp_get.status(), 200);
    let body_get = hyper::body::to_bytes(resp_get.into_body()).await.unwrap();
    assert_eq!(&body_get[..], b"g"); // echo
    handle.abort();
}
