use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process;
use std::thread;

struct Config {
  dir: PathBuf,
  bind: String,
  port: u16,
}

fn print_help() {
  println!(
    r#"simple-http-server - Simplest Http Server
        USAGE:
            simple-http-server [OPTIONS]
        
        OPTIONS:
        "#
  );
}

fn parse_args() -> Config {
  let mut dir = PathBuf::from(".");
  let mut bind = String::from("0.0.0.0");
  let mut port: u16 = 8000;
  let mut port_set = false;

  let args: Vec<String> = env::args().collect();
  let mut i = 1;

  while i < args.len() {
    match args[i].as_str() {
      "-d" | "--dir" => {
        i += 1;
        if i >= args.len() {
          eprintln!("Error: {} requires a value", args[i - 1]);
          process::exit(1);
        }
        dir = PathBuf::from(&args[i]);
      }
      "-b" | "--bind" => {
        i += 1;
        if i >= args.len() {
          eprintln!("Error: {} requires a value", args[i - 1]);
          process::exit(1);
        }
        bind = args[i].clone()
      }
      "-h" | "--help" => {
        print_help();
        process::exit(0);
      }
      other => {
        if !other.starts_with('-') && !port_set {
          port = match other.parse() {
            Ok(p) => p,
            Err(_) => {
              eprintln!("error: invalid port number: {}", other);
              process::exit(1);
            }
          };
          port_set = true;
        } else {
          eprintln!("Error: unknown argment(s): {}", other);
          print_help();
          process::exit(1);
        }
      }
    }
    i += 1;
  }
  let dir = match fs::canonicalize(&dir) {
    Ok(p) => p,
    Err(e) => {
      eprintln!("Error: cannot open directory '{}': {}", dir.display(), e);
      process::exit(1);
    }
  };

  if !dir.is_dir() {
    eprintln!("Error: '{}' is not a directory.", dir.display());
    process::exit(1)
  }

  Config { dir, bind, port }
}

fn mime_type(path: &Path) -> &'static str {
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

fn html_escape(s: &str) -> String {
  s.replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

fn resolve_path(root: &Path, url_path: &str) -> Option<PathBuf> {
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

fn percent_decode(s: &str) -> String {
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

fn build_directory_listing(dir: &Path, url_path: &str) -> String {
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

fn send_response(
  stream: &mut TcpStream,
  status: &str,
  content_type: &str,
  body: &[u8],
) -> std::io::Result<()> {
  let header = format!(
    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    status,
    content_type,
    body.len()
  );
  stream.write_all(header.as_bytes())?;
  stream.write_all(body)?;
  stream.flush()
}

fn handle_client(mut stream: TcpStream, root: PathBuf) {
  let mut reader = BufReader::new(stream.try_clone().expect("clone failed"));
  let mut request_line = String::new();

  if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
    return;
  }

  loop {
    let mut line = String::new();
    match reader.read_line(&mut line) {
      Ok(0) | Err(_) => break,
      Ok(_) => {
        if line == "\r\n" || line == "\n" {
          break;
        }
      }
    }
  }
  let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
  if parts.len() < 2 {
    let _ = send_response(&mut stream, "400 Bad Request", "text/plain", b"Bad Request");
    return;
  }

  let method = parts[0];
  let raw_path = parts[1];
  let url_path = raw_path.split("?").next().unwrap_or("/");

  if method != "GET" && method != "HEAD" {
    let _ = send_response(
      &mut stream,
      "405 Method Not Allowed",
      "text/plain",
      b"Method Not Allowed",
    );
    return;
  }

  let target = match resolve_path(&root, url_path) {
    Some(p) => p,
    None => {
      let _ = send_response(&mut stream, "400 Bad Request", "text/plain", b"Bad Request");
      return;
    }
  };

  let canonical = fs::canonicalize(&target).ok();
  let is_inside_root = canonical
    .as_ref()
    .map(|c| c.starts_with(&root))
    .unwrap_or(false);
  if !is_inside_root || !target.exists() {
    let body = b"404 Not Found";
    let _ = send_response(&mut stream, "404 Not Found", "text/plain", body);
    return;
  }

  if target.is_dir() {
    let index = target.join("index.html");
    if index.is_file() {
      serve_file(&mut stream, &index, method);
    } else {
      let listing = build_directory_listing(&target, url_path);
      if method == "HEAD" {
        let _ = send_response(&mut stream, "200 OK", "text/html; charset=utf-8", b"");
      } else {
        let _ = send_response(
          &mut stream,
          "200 OK",
          "text/html; charset=utf-8",
          listing.as_bytes(),
        );
      }
    }
  } else {
    serve_file(&mut stream, &target, method);
  }
}

fn serve_file(stream: &mut TcpStream, path: &Path, method: &str) {
  match fs::read(path) {
    Ok(contents) => {
      let ctype = mime_type(path);
      if method == "HEAD" {
        let _ = send_response(stream, "200 OK", ctype, b"");
      } else {
        let _ = send_response(stream, "200 OK", ctype, &contents);
      }
    }
    Err(_) => {
      let _ = send_response(
        stream,
        "500 Internal Server Error",
        "text/plain",
        b"Internal Server Error",
      );
    }
  }
}

fn main() {
  let config = parse_args();

  let addr = format!("{}:{}", config.bind, config.port);
  let listener = match TcpListener::bind(&addr) {
    Ok(l) => l,
    Err(e) => {
      eprintln!("Error: cannot bind to {}:{}", addr, e);
      process::exit(1);
    }
  };

  println!("Serving directory: {}", config.dir.display());
  println!("Server running at: http://{}", addr);
  println!("(Press Ctrl+C to stop server)");

  for stream in listener.incoming() {
    match stream {
      Ok(stream) => {
        let root = config.dir.clone();
        thread::spawn(move || {
          handle_client(stream, root);
        });
      }
      Err(e) => {
        eprintln!("Connection error: {}", e);
      }
    }
  }
}
