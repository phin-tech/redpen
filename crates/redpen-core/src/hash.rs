use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn hash_string(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let content = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hash_string_deterministic() {
        let h1 = hash_string("hello world");
        let h2 = hash_string("hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_hash_string_different_inputs() {
        assert_ne!(hash_string("hello"), hash_string("world"));
    }

    #[test]
    fn test_hash_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(b"test content").unwrap();
        drop(f);
        let hash = hash_file(&file_path).unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, hash_string("test content"));
    }
}
