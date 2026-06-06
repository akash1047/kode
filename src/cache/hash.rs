use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use xxhash_rust::xxh3::Xxh3;

const BUF_SIZE: usize = 64 * 1024;

pub fn file_hash(path: &Path) -> std::io::Result<[u8; 8]> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(BUF_SIZE, file);
    let mut hasher = Xxh3::new();
    let mut buf = [0u8; BUF_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.digest().to_le_bytes())
}

pub fn bytes_hash(bytes: &[u8]) -> [u8; 8] {
    let mut hasher = Xxh3::new();
    hasher.update(bytes);
    hasher.digest().to_le_bytes()
}

pub fn repo_id_from(remote: Option<&str>, abs_path: &Path) -> String {
    let key = remote
        .map(|r| format!("remote:{r}"))
        .unwrap_or_else(|| format!("path:{}", abs_path.display()));
    let mut hasher = Xxh3::new();
    hasher.update(key.as_bytes());
    format!("{:016x}", hasher.digest())
}
