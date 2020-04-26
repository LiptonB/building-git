use crypto::{digest::Digest, sha1::Sha1};

// TODO: would an enum make more sense since it seems like content is the only real function
// needing to be overloaded?
pub trait Object {
    fn object_type(&self) -> &str;
    fn content(&self) -> Vec<u8>;

    fn to_bytes(&self) -> Vec<u8> {
        let object_type = self.object_type();
        let content = self.content();
        let len_tag = content.len().to_string();

        let mut serialized =
            Vec::with_capacity(object_type.len() + len_tag.len() + content.len() + 2);
        serialized.extend_from_slice(object_type.as_ref());
        serialized.push(b' ');
        serialized.extend_from_slice(len_tag.as_ref());
        serialized.push(b'\0');
        serialized.extend_from_slice(&content);

        serialized
    }

    fn oid(&self) -> String {
        let mut hasher = Sha1::new();
        // TODO: caching?
        hasher.input(&self.to_bytes());
        let oid = hasher.result_str();
        println!("Computed oid of {}", oid);
        oid
    }
}
