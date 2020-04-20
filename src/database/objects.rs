use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub struct Blob {
    data: Vec<u8>,
}

pub trait Object {
    fn object_type(&self) -> String;
    fn oid(&self) -> String;
    fn to_bytes(&self) -> Vec<u8>;
}

pub type BoxObject = Box<dyn Object>;

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Object for Blob {
    fn object_type(&self) -> String {
        "blob".to_owned()
    }

    fn oid(&self) -> String {
        let mut hasher = Sha1::new();
        hasher.input(&self.data);
        hasher.result_str()
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}
