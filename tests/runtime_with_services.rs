use allora::processor::SyncProcessor;
use allora::{build, Error};

#[test]
fn build_runtime_with_services_success() -> Result<(), Error> {
    let rt = build("tests/fixtures/allora_with_services.yml")?;
    assert_eq!(rt.channel_count(), 2);
    assert_eq!(rt.service_count(), 2);
    // ensure headers are set when services process
    let mut exchange = allora::Exchange::new(allora::Message::from_text("ping"));
    // invoke first service processor
    rt.services()[0].process_sync(&mut exchange)?;
    assert_eq!(
        exchange.in_msg.header("service-activator.id"),
        Some("svc.hello")
    );
    assert_eq!(
        exchange.in_msg.header("service-activator.ref-name"),
        Some("service1")
    );
    Ok(())
}
