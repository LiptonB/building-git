use super::object::Object;

#[derive(Debug, Clone)]
pub struct Blob {
    data: Vec<u8>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Object for Blob {
    fn object_type(&self) -> &str {
        "blob"
    }

    fn content(&self) -> Vec<u8> {
        self.data.clone()
    }
}
