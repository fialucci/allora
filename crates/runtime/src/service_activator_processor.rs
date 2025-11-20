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
        let inbound = runtime.channel::<crate::DirectChannel>(&from_id);
        let ref_name = self.ref_name().to_string();
        let svc_clone = arc.clone();
        inbound.subscribe(move |exchange: Exchange| {
            let svc_task = svc_clone.clone();
            let runtime_ref: &'static AlloraRuntime = runtime;
            let to_id_clone = to_id.clone();
            let ref_name_clone = ref_name.clone();
            tokio::spawn(async move {
                let mut ex_mut = exchange;
                if let Err(err) = svc_task.process(&mut ex_mut).await {
                    tracing::error!(target="allora::service", service.ref=%ref_name_clone, error=%err, "service processing failed");
                    return;
                }
                let outbound = runtime_ref.channel::<crate::DirectChannel>(&to_id_clone);
                if let Err(err) = outbound.send(ex_mut).await {
                    tracing::error!(target="allora::service", service.ref=%ref_name_clone, outbound.channel=outbound.id(), error=%err, "outbound send failed");
                }
            });
            Ok(())
        });
        self.service = Some(arc);
        Ok(())
    }
}
