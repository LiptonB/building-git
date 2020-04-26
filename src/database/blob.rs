use super::object::Object;

#[derive(Debug, Clone)]
pub struct Blob {
    data: Vec<u8>,
    oid: Option<String>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, oid: None }
    }
}

impl Object for Blob {
    fn object_type(&self) -> &str {
        "blob"
    }

    fn content(&self) -> Vec<u8> {
        self.data.clone()
    }

    fn set_oid(&mut self, oid: String) {
        assert!(self.oid.is_none());
        self.oid = Some(oid);
    }

    fn get_oid(&self) -> Option<&str> {
        self.oid.as_deref()
    }
}
