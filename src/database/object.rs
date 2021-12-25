use crypto::{digest::Digest, sha1::Sha1};

// TODO: would an enum make more sense since it seems like content is the only real function
// needing to be overloaded?
pub trait Object {
    fn object_type(&self) -> &str;
    fn content(&self) -> Vec<u8>;

    // TODO: I don't really like the duplication of implementing these - would prefer a distinct
    // object for things with oids
    fn set_oid(&mut self, oid: String);
    fn get_oid(&self) -> Option<&str>;

    fn oid(&self) -> &str {
        self.get_oid().expect("Oid not computed yet")
    }
}

pub fn to_bytes<O: Object>(object: &O) -> Vec<u8> {
    let object_type = object.object_type();
    let content = object.content();
    let len_tag = content.len().to_string();

    let mut serialized = Vec::with_capacity(object_type.len() + len_tag.len() + content.len() + 2);
    serialized.extend_from_slice(object_type.as_ref());
    serialized.push(b' ');
    serialized.extend_from_slice(len_tag.as_ref());
    serialized.push(b'\0');
    serialized.extend_from_slice(&content);

    serialized
}

pub fn compute_oid<O: Object>(object: &mut O) {
    assert!(object.get_oid().is_none());

    let mut hasher = Sha1::new();
    hasher.input(&to_bytes(object));
    let oid = hasher.result_str();
    object.set_oid(oid);
}
