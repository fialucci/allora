use allora::service;
#[derive(Debug)]
struct TraitSvc;
impl TraitSvc {
    pub fn new() -> Self {
        Self
    }
}
#[service]
impl Service for TraitSvc {}
trait Service {}
fn main() {}
