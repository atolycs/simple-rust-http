use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;

use crate::config::{Config, PKG_NAME, PKG_VERSION, PROFILE};
use crate::content::{build_directory_listing, mime_type};
use crate::response::send_response;
use crate::security::resolve_path;

pub fn run(config: Config) {
  let addr = format!("{}:{}", config.bind, config.port);
  let listener = match TcpListener::bind(&addr) {
    Ok(l) => l,
    Err(e) => {
      eprintln!("Error: cannnot bind to {}: {}", addr, e);
      process::exit(1);
    }
  };
  println!("{} v{} ({})", PKG_NAME, PKG_VERSION, PROFILE);
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

fn handle_client(mut stream: TcpStream, root: PathBuf) {
  let client = stream
    .peer_addr()
    .map(|a| a.to_string())
    .unwrap_or_else(|_| "-".to_string());

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
    let _ = send_response(
      &mut stream,
      "400 Bad Request",
      "text/plain",
      b"Bad Request",
      &client,
      "-",
      "-",
    );
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
      &client,
      method,
      url_path,
    );
    return;
  }

  let target = match resolve_path(&root, url_path) {
    Some(p) => p,
    None => {
      let _ = send_response(
        &mut stream,
        "400 Bad Request",
        "text/plain",
        b"Bad Request",
        &client,
        method,
        url_path,
      );
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
    let _ = send_response(
      &mut stream,
      "404 Not Found",
      "text/plain",
      body,
      &client,
      method,
      url_path,
    );
    return;
  }

  if target.is_dir() {
    let index = target.join("index.html");
    if index.is_file() {
      serve_file(&mut stream, &index, method, &client, url_path);
    } else {
      let listing = build_directory_listing(&target, url_path);
      if method == "HEAD" {
        let _ = send_response(
          &mut stream,
          "200 OK",
          "text/html; charset=utf-8",
          b"",
          &client,
          method,
          url_path,
        );
      } else {
        let _ = send_response(
          &mut stream,
          "200 OK",
          "text/html; charset=utf-8",
          listing.as_bytes(),
          &client,
          method,
          url_path,
        );
      }
    }
  } else {
    serve_file(&mut stream, &target, method, &client, url_path);
  }
}

fn serve_file(stream: &mut TcpStream, path: &Path, method: &str, client: &str, url_path: &str) {
  match fs::read(path) {
    Ok(contents) => {
      let ctype = mime_type(path);
      if method == "HEAD" {
        let _ = send_response(stream, "200 OK", ctype, b"", client, method, url_path);
      } else {
        let _ = send_response(stream, "200 OK", ctype, &contents, client, method, url_path);
      }
    }
    Err(_) => {
      let _ = send_response(
        stream,
        "500 Internal Server Error",
        "text/plain",
        b"Internal Server Error",
        client,
        method,
        url_path,
      );
    }
  }
}
