use std::io::Write;
use std::net::TcpStream;

use crate::logging::log_access;

pub fn send_response(
  stream: &mut TcpStream,
  status: &str,
  content_type: &str,
  body: &[u8],
  client: &str,
  method: &str,
  path: &str,
) -> std::io::Result<()> {
  let header = format!(
    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
    status,
    content_type,
    body.len()
  );
  stream.write_all(header.as_bytes())?;
  stream.write_all(body)?;
  let result = stream.flush();
  log_access(client, method, path, status, body.len());
  result
}
