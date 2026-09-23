use std::fs;
use std::path::Path;

pub fn mime_type(path: &Path) -> &'static str {
  match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
    "html" | "htm" => "text/html; charset=utf-8",
    "css" => "text/css; charset=utf-8",
    "js" => "application/javascript; charset=utf-8",
    "json" => "application/json; charset=utf-8",
    "png" => "image/png",
    "jpg" | "jpeg" => "image/jpeg",
    "gif" => "image/gif",
    "svg" => "image/svg+xml",
    "ico" => "image/x-icon",
    "txt" => "text/plain; charset=utf-8",
    "pdf" => "application/pdf",
    "zip" => "application/zip",
    "mp4" => "video/mp4",
    "mp3" => "audio/mpeg",
    "wasm" => "application/wasm",
    _ => "application/octet-stream",
  }
}

pub fn html_escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

pub fn build_directory_listing(dir: &Path, url_path: &str) -> String {
  let mut entries: Vec<_> = fs::read_dir(dir)
    .map(|rd| rd.filter_map(|e| e.ok()).collect())
    .unwrap_or_else(|_| Vec::new());
  entries.sort_by_key(|e| e.file_name());

  let mut body = String::new();

  body.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\">");
  body.push_str(&format!(
    "<title>Index of {}</title></head><body>",
    html_escape(url_path)
  ));
  body.push_str(&format!("<h1>Index of {}</h1><ul>", html_escape(url_path)));

  if url_path != "/" {
    body.push_str("<li><a href=\"../\">../</a></li>");
  }

  for entry in entries {
    let name = entry.file_name().to_string_lossy().into_owned();
    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
    let suffix = if is_dir { "/" } else { "" };
    let href = format!("{}{}", name, suffix);

    body.push_str(&format!(
      "<li><a href=\"{}\">{}{}</a></li>",
      href,
      html_escape(&name),
      suffix
    ));
  }
  body.push_str("</ul></body></html>");
  body
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn mime_type_known_extension() {
    assert_eq!(mime_type(Path::new("a.html")), "text/html; charset=utf-8");
    assert_eq!(
      mime_type(Path::new("a.unknown")),
      "application/octet-stream"
    );
  }

  #[test]
  fn html_escape_escapes_special_chars() {
    assert_eq!(html_escape("<a>&\"b\""), "&lt;a&gt;&amp;&quot;b&quot;");
  }
}
