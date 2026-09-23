use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

pub struct Config {
  pub dir: PathBuf,
  pub bind: String,
  pub port: u16,
}

pub const PKG_NAME: &str = env!("CARGO_PKG_NAME");
pub const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_help() {
  println!(
    r#"simple-http-server - A minimal static file HTTP server

USAGE:
  simple-http-server [PORT] [OPTIONS]
ARGS:
  [PORT]  Port number (default: 8000)

OPTIONS:
  -d, --dir <DIR>   Directory to serve (default: ".")
  -b, --bind <ADDR> IP Address to bind (default: "0.0.0.0")
  -v, --version     Show version information
  -h, --help        Show this help message

EXAMPLE:
  simple-http-server 3000 -d ./public

"#
  );
}

pub fn print_version() {
  println!("{} {}", PKG_NAME, PKG_VERSION)
}

pub fn parse_args() -> Config {
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
          process::exit(1)
        }
        dir = PathBuf::from(&args[i])
      }

      "-b" | "--bind" => {
        i += 1;
        if i >= args.len() {
          eprintln!("Error: {} requires a value", args[i - 1]);
          process::exit(1)
        }

        bind = args[i].clone();
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
              eprintln!("Error: invalid port number: {}", other);
              process::exit(1);
            }
          };
          port_set = true;
        } else {
          eprintln!("Error: unknown argument(s): {}", other);
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
    eprintln!("Error: '{}' is not directory", dir.display());
    process::exit(1);
  }

  Config { dir, bind, port }
}
