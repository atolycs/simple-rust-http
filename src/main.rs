use simple_http_server::{config, server};

fn main() {
  let config = config::parse_args();
  server::run(config);
}
