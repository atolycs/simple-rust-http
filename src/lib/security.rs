use std::path::{Component, Path, PathBuf};

pub fn resolve_path(root: &Path, url_path: &str) -> Option<PathBuf> {
  let decoded = percent_decode(url_path);
  let mut result = root.to_path_buf();

  for comp in Path::new(&decoded).components() {
    match comp {
      Component::Normal(part) => result.push(part),
      Component::ParentDir => return None,
      Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
    }
  }
  Some(result)
}

pub fn percent_decode(s: &str) -> String {
  let bytes = s.as_bytes();
  let mut out = Vec::with_capacity(bytes.len());
  let mut i = 0;

  while i < bytes.len() {
    if bytes[i] == b'%' && i + 2 < bytes.len() {
      if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
        if let Ok(byte) = u8::from_str_radix(hex, 16) {
          out.push(byte);
          i += 3;
          continue;
        }
      }
    }
    out.push(bytes[i]);
    i += 1;
  }
  String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn percent_decode_basic() {
    assert_eq!(percent_decode("a%20b"), "a b");
    assert_eq!(percent_decode("no-escape"), "no-escape");
  }

  #[test]
  fn resolve_path_rejects_parent_dir() {
    let root = Path::new("/tmp/root");
    assert!(resolve_path(root, "/../etc/passwd").is_none());
  }

  #[test]
  fn resolve_path_joins_normal_comonents() {
    let root = Path::new("/tmp/root");
    assert_eq!(
      resolve_path(root, "/sub/a.txt"),
      Some(PathBuf::from("/tmp/root/sub/a.txt"))
    );
  }
}
