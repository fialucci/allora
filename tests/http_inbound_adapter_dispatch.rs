use allora_core::adapter::Adapter;
use allora_core::channel::{ChannelRef, DirectChannel, QueueChannel};
use allora_core::error::Result;
use allora_core::Channel;
use allora_core::Exchange;
use allora_core::PollableChannel;
use allora_http::{InboundHttpExt, Mep};
use hyper::client::HttpConnector;
use hyper::{Body, Client, Request};
use tokio::time::{sleep, Duration};

fn next_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_dispatch_in_only_202_forwards_to_reply_channel_root() -> Result<()> {
    let receive_channel = Arc::new(DirectChannel::with_id("receiveChannel"));
    let reply_channel = Arc::new(QueueChannel::with_id("replyChannel"));
    let reply_clone = reply_channel.clone();
    receive_channel.subscribe(move |exchange| {
        let reply_inner = reply_clone.clone();
        tokio::spawn(async move {
            let _ = reply_inner.send(exchange).await;
        });
        Ok(())
    });
    let port = next_port();
    let adapter = allora_http::HttpInboundAdapter::new_in_only_202(
        "127.0.0.1",
        port,
        "/receiveGateway",
        receive_channel.clone() as ChannelRef,
        None,
    );
    let handle = adapter.clone().spawn_serve();
    sleep(Duration::from_millis(50)).await;
    let client: Client<HttpConnector, Body> = Client::new();
    let uri: hyper::Uri = format!("http://127.0.0.1:{port}/receiveGateway")
        .parse()
        .unwrap();
    let req = Request::post(uri)
        .body(Body::from("Ping"))
        .expect("request build");
    let resp = client.request(req).await.expect("http request succeeds");
    assert_eq!(resp.status().as_u16(), 202);
    let mut received: Option<Exchange> = None;
    for _ in 0..20 {
        if let Some(ex) = reply_channel.try_receive().await {
            received = Some(ex);
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    handle.abort();
    let ex = received.expect("forwarded exchange present");
    assert_eq!(ex.in_msg.body_text(), Some("Ping"));
    assert_eq!(ex.in_msg.header("http.method"), Some("POST"));
    assert_eq!(ex.in_msg.header("http.path"), Some("/"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_dispatch_in_out_enqueues_and_returns_body_root() -> Result<()> {
    let receive_channel = Arc::new(DirectChannel::with_id("receiveChannel"));
    let reply_channel = Arc::new(QueueChannel::with_id("replyChannel"));
    let reply_clone = reply_channel.clone();
    receive_channel.subscribe(move |exchange| {
        let reply_inner = reply_clone.clone();
        tokio::spawn(async move {
            let _ = reply_inner.send(exchange).await;
        });
        Ok(())
    });
    let port = next_port();
    let adapter = allora_http::HttpInboundAdapter::new_in_out(
        "127.0.0.1",
        port,
        "/receiveGateway",
        receive_channel.clone() as ChannelRef,
        None,
    );
    assert_eq!(adapter.mep(), Mep::InOut);
    let handle = adapter.clone().spawn_serve();
    sleep(Duration::from_millis(50)).await;
    let client: Client<HttpConnector, Body> = Client::new();
    let uri: hyper::Uri = format!("http://127.0.0.1:{port}/receiveGateway")
        .parse()
        .unwrap();
    let req = Request::post(uri)
        .body(Body::from("EchoMe"))
        .expect("request build");
    let resp = client.request(req).await.expect("http request succeeds");
    assert_eq!(resp.status().as_u16(), 200);
    let bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(std::str::from_utf8(&bytes).unwrap(), "EchoMe");
    let mut received: Option<Exchange> = None;
    for _ in 0..20 {
        if let Some(ex) = reply_channel.try_receive().await {
            received = Some(ex);
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }
    handle.abort();
    let ex = received.expect("exchange forwarded in InOut mode");
    assert_eq!(ex.in_msg.body_text(), Some("EchoMe"));
    assert_eq!(ex.in_msg.header("http.method"), Some("POST"));
    assert_eq!(ex.in_msg.header("http.path"), Some("/"));
    Ok(())
}
