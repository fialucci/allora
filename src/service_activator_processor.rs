use crate::channel::Channel;
use crate::dsl::runtime::AlloraRuntime;
use crate::{error::Result, service::Service, spec::ServiceActivatorSpec, Exchange};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

/// Processor that binds a ServiceSpec (activator metadata) to service logic and wires channels.
pub struct ServiceActivatorProcessor {
    activator: ServiceActivatorSpec,
    service: Option<Arc<dyn Service>>, // logic assigned after runtime build
}

impl Debug for ServiceActivatorProcessor {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("ServiceActivatorProcessor")
            .field("id", &self.activator.id())
            .field("from", &self.activator.from())
            .field("to", &self.activator.to())
            .finish()
    }
}

impl ServiceActivatorProcessor {
    pub fn new(activator: ServiceActivatorSpec) -> Self {
        Self {
            activator,
            service: None,
        }
    }
    pub fn id(&self) -> &str {
        self.activator.id().unwrap_or("")
    }
    pub fn from(&self) -> &str {
        self.activator.from()
    }
    pub fn to(&self) -> &str {
        self.activator.to()
    }
    pub fn ref_name(&self) -> &str {
        self.activator.ref_name()
    }
    pub fn has_service(&self) -> bool {
        self.service.is_some()
    }
    pub fn set_service_and_wire(
        &mut self,
        svc: impl Service + 'static,
        runtime: &'static AlloraRuntime,
    ) -> Result<()> {
        let arc = Arc::new(svc);
        let from_id = self.from().to_string();
        let to_id = self.to().to_string();
        let inbound = runtime.channel_typed_or_panic::<crate::DirectChannel>(&from_id);
        let svc_clone = arc.clone();
        inbound.subscribe(move |exchange: Exchange| {
            #[cfg(feature = "async")]
            {
                use tokio::runtime::Handle;
                // Bridge async service + outbound send it into sync closure without spawning a new runtime each message.
                tokio::task::block_in_place(|| {
                    let mut ex_mut = exchange;
                    Handle::current().block_on(async {
                        svc_clone.process(&mut ex_mut).await?;
                        let outbound =
                            runtime.channel_typed_or_panic::<crate::DirectChannel>(&to_id);
                        outbound.send_async(ex_mut).await
                    })
                })?;
            }
            #[cfg(not(feature = "async"))]
            {
                let mut ex_mut = exchange;
                svc_clone.process(&mut ex_mut)?;
                let outbound = runtime.channel_typed_or_panic::<crate::DirectChannel>(&to_id);
                outbound.send(ex_mut)?;
            }
            Ok(())
        });
        self.service = Some(arc);
        Ok(())
    }
}
