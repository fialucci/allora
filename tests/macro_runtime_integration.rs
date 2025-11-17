use allora::{all_service_descriptors, error::Result, service, Exchange, Message, Service};

#[derive(Debug)]
struct RuntimeSvc;
#[service(name = "runtime")]
impl RuntimeSvc {
    pub fn new() -> Self {
        Self
    }
}
#[cfg_attr(feature = "async", async_trait::async_trait)]
impl Service for RuntimeSvc {
    #[cfg(feature = "async")]
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        exchange.in_msg.set_header("runtime", "ok");
        Ok(())
    }
    #[cfg(not(feature = "async"))]
    fn process(&self, exchange: &mut Exchange) -> Result<()> {
        exchange.in_msg.set_header("runtime", "ok");
        Ok(())
    }
}

#[test]
fn descriptor_registration_and_execution() {
    let descs = all_service_descriptors();
    let found = descs
        .iter()
        .find(|d| d.name == "runtime")
        .expect("descriptor runtime present");
    let proc = (found.constructor)();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    #[cfg(feature = "async")]
    {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            proc.process_sync(&mut exchange).unwrap();
        });
    }
    #[cfg(not(feature = "async"))]
    {
        proc.process_sync(&mut exchange).unwrap();
    }
    assert_eq!(exchange.in_msg.header("runtime"), Some("ok"));
}
