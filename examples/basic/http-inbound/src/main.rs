use allora::{Result, Runtime};
use hyper::{Body, Client, Request, Uri};

mod http_echo_service; // ensures service registered

#[tokio::main]
async fn main() -> Result<()> {
    let adapter = Runtime::new()
        .with_config_file("examples/basic/http-inbound/allora.yml")
        .run()? // load config
        .http_inbound_adapters()
        .get(0)
        .expect("http inbound adapter in config")
        .clone();
    let _server = adapter.clone().spawn_serve();
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let uri: Uri = format!("http://{}{}", adapter.addr(), adapter.base_path())
        .parse()
        .unwrap();
    let (status, body) = fire_one_request(uri, "World").await?;
    println!("status={} body={}", status, body);
    Ok(())
}

async fn fire_one_request(uri: Uri, payload: &str) -> Result<(u16, String)> {
    let client: Client<hyper::client::HttpConnector, Body> = Client::new();
    let req = Request::post(uri)
        .header("Content-Type", "text/plain")
        .body(Body::from(payload.to_string()))
        .map_err(|e| allora::Error::other(e.to_string()))?;
    let resp = client
        .request(req)
        .await
        .map_err(|e| allora::Error::other(e.to_string()))?;
    let status = resp.status().as_u16();
    let bytes = hyper::body::to_bytes(resp.into_body())
        .await
        .map_err(|e| allora::Error::other(e.to_string()))?;
    let body = String::from_utf8_lossy(&bytes).to_string();
    Ok((status, body))
}
