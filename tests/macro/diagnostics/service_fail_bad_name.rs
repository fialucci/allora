use allora::service;
#[derive(Debug)]
struct BadName;
impl BadName {
    pub fn new() -> Self {
        Self
    }
}
#[service(name = 123)]
impl BadName {}
fn main() {}
