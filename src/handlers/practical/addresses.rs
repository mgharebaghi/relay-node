use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MyAddress {
    addr: String,
}

impl MyAddress {
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}
