use simple_http_server::config::Config;
use simple_http_server::server;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

fn wait_for_file_ready(path: &PathBuf, expected: &[u8]) {
  let deadline = Instant::now() + Duration::from_secs(5);
  loop {
    if let Ok(content) = fs::read(path) {
      if content == expected {
        return;
      }
    }

    if Instant::now() >= deadline {
      panic!("file was not ready with in 5 seconds: {}", path.display());
    }

    thread::sleep(Duration::from_millis(10));
  }
}

fn setup_test_dir(name: &str) -> PathBuf {
  let dir = std::env::temp_dir().join(format!("simple_http_server_it_{name}"));
  let _ = fs::remove_dir_all(&dir);

  fs::create_dir_all(dir.join("sub")).expect("failed to create test dir");
  let hello = dir.join("hello.txt");
  let nested = dir.join("sub").join("nested.txt");

  fs::write(&hello, b"hello world").unwrap();
  fs::write(&nested, b"nested file").unwrap();

  wait_for_file_ready(&hello, b"hello world");
  wait_for_file_ready(&nested, b"nested file");

  fs::canonicalize(&dir).expect("failed to canonicalize test directory")
}

fn start_server(dir: PathBuf, port: u16) {
  let config = Config {
    dir,
    bind: "127.0.0.1".to_string(),
    port,
  };
  thread::spawn(move || {
    server::run(config);
  });

  let deadline = std::time::Instant::now() + Duration::from_secs(5);

  loop {
    match TcpStream::connect(("127.0.0.1", port)) {
      Ok(_) => break,
      Err(err) => {
        if std::time::Instant::now() >= deadline {
          panic!("server did not become ready on port {port}: {err}");
        }
        thread::sleep(Duration::from_millis(10));
      }
    }
  }
}

fn raw_request(port: u16, method: &str, path: &str) -> (String, String, Vec<u8>) {
  let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("failed to connect");

  stream
    .set_read_timeout(Some(Duration::from_secs(5)))
    .expect("failed to set read timeout");
  stream
    .set_write_timeout(Some(Duration::from_secs(5)))
    .expect("failed to set write timeout");

  let request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes()).unwrap();

  let mut raw = Vec::new();
  stream.read_to_end(&mut raw).unwrap();

  let sep = b"\r\n\r\n";
  let split_at = raw
    .windows(sep.len())
    .position(|w| w == sep)
    .unwrap_or(raw.len());
  let header_bytes = &raw[..split_at];
  let body = if split_at + sep.len() <= raw.len() {
    raw[split_at + sep.len()..].to_vec()
  } else {
    Vec::new()
  };

  let header_text = String::from_utf8_lossy(header_bytes).into_owned();
  let mut lines = header_text.lines();
  let status_line = lines.next().unwrap_or("").to_string();
  let headers = lines.collect::<Vec<_>>().join("\n");

  (status_line, headers, body)
}

fn status_code(status_line: &str) -> &str {
  status_line.split_whitespace().nth(1).unwrap_or("")
}

#[test]
fn get_existing_file_returns_200_with_body() {
  let dir = setup_test_dir("get_existing_file");
  start_server(dir, 18081);

  let (status, headers, body) = raw_request(18081, "GET", "/hello.txt");
  assert_eq!(status_code(&status), "200");
  assert!(headers.contains("Content-Type: text/plain"));
  assert_eq!(body, b"hello world");
}

#[test]
fn get_nested_file_returns_200() {
  let dir = setup_test_dir("get_nested_file");
  start_server(dir, 18082);

  let (status, _headers, body) = raw_request(18082, "GET", "/sub/nested.txt");
  assert_eq!(status_code(&status), "200");
  assert_eq!(body, b"nested file");
}

#[test]
fn get_missing_file_returns_404() {
  let dir = setup_test_dir("get_missing_file");
  start_server(dir, 18083);

  let (status, _headers, _body) = raw_request(18083, "GET", "/does-not-exist.txt");
  assert_eq!(status_code(&status), "404");
}

#[test]
fn get_directory_without_index_returns_listing() {
  let dir = setup_test_dir("get_directory_listing");
  start_server(dir, 18084);

  let (status, headers, body) = raw_request(18084, "GET", "/");
  assert_eq!(status_code(&status), "200");
  assert!(headers.contains("text/html"));
  let body_text = String::from_utf8_lossy(&body);
  assert!(body_text.contains("hello.txt"));
  assert!(body_text.contains("sub/"));
}

#[test]
fn post_method_is_not_allowed() {
  let dir = setup_test_dir("post_not_allowed");
  start_server(dir, 18085);

  let (status, _headers, _body) = raw_request(18085, "POST", "/");
  assert_eq!(status_code(&status), "405");
}

#[test]
fn head_request_has_no_body() {
  let dir = setup_test_dir("head_request");
  start_server(dir, 18086);

  let (status, _headers, body) = raw_request(18086, "HEAD", "/hello.txt");
  assert_eq!(status_code(&status), "200");
  assert!(body.is_empty());
}

#[test]
fn path_traversal_attempt_is_rejected() {
  let dir = setup_test_dir("path_traversal");
  start_server(dir, 18087);

  let (status, _headers, _body) = raw_request(18087, "GET", "/../../etc/passwd");
  assert_ne!(status_code(&status), "200");
}

#[test]
fn index_html_is_served_for_directory_when_present() {
  let dir = setup_test_dir("index.html");
  fs::write(dir.join("index.html"), b"<h1>welcome</h1>").unwrap();
  start_server(dir, 18088);

  let (status, headers, body) = raw_request(18088, "GET", "/");
  assert_eq!(status_code(&status), "200");
  assert!(headers.contains("text/html"));
  assert_eq!(body, b"<h1>welcome</h1>");
}
