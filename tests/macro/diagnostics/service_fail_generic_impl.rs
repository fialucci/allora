use allora::service;
#[derive(Debug)]
struct GenSvc<T>(T);
impl<T> GenSvc<T> {
    pub fn new() -> Self
    where
        T: Default,
    {
        Self(T::default())
    }
}
#[service]
impl<T> GenSvc<T> {}
fn main() {}
