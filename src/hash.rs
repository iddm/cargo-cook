use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

use crypto::digest::Digest;

lazy_static::lazy_static! {
    static ref HASHES: HashMap<&'static str, fn(&[u8]) -> String> = {
        let mut m = HashMap::new();
        m.insert("md5", md5 as fn(&[u8]) -> String);
        m.insert("sha256", sha256 as fn(&[u8]) -> String);
        m.insert("sha512", sha512 as fn(&[u8]) -> String);
        m
    };
}

fn md5(bytes: &[u8]) -> String {
    use crypto::md5::Md5;

    let mut hasher = Md5::new();
    hasher.input(bytes);
    hasher.result_str()
}

fn sha256(bytes: &[u8]) -> String {
    use crypto::sha2::Sha256;

    let mut hasher = Sha256::new();
    hasher.input(bytes);
    hasher.result_str()
}

fn sha512(bytes: &[u8]) -> String {
    use crypto::sha2::Sha512;

    let mut hasher = Sha512::new();
    hasher.input(bytes);
    hasher.result_str()
}

pub fn support_hash_type(hash: &str) -> bool {
    HASHES.get::<str>(&hash.to_lowercase()).is_some()
}

pub fn hash(bytes: &[u8], hash: &str) -> String {
    HASHES.get::<str>(&hash.to_lowercase()).unwrap()(bytes)
}

pub fn file_hash(path: &str, hash_type: &str) -> String {
    let mut bytes = Vec::new();
    let mut f = File::open(path).unwrap();
    let _ = f.read_to_end(&mut bytes);
    hash(&bytes, hash_type)
}

pub fn write_file_hash(source: &str, destination: &str, hash_type: &str) {
    let mut f = File::create(destination).unwrap();
    let _ = writeln!(f, "{}", file_hash(source, hash_type));
}
