use allora::{build, Error};

#[tokio::test]
async fn runtime_services_process() -> allora::Result<()> {
    let rt = allora::dsl::build("./tests/fixtures/allora_with_services.yml")?;
    let svc = &rt.services()[0];
    let mut exchange = allora::Exchange::new(allora::Message::from_text("hello"));
    svc.process(&mut exchange).await?;
    assert!(exchange
        .in_msg
        .header("service-activator.ref-name")
        .is_some());
    Ok(())
}
