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

pub const PROFILE: &str = if cfg!(debug_assertions) {
  "debug"
} else {
  "release"
};

pub fn print_help() {
  println!(
    r#"simple-http-server - A minimal static file HTTP server v{PKG_VERSION}

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

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
  Run(Config),
  ShowHelp,
  ShowVersion,
}

impl std::fmt::Debug for Config {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Config")
      .field("dir", &self.dir)
      .field("bind", &self.bind)
      .field("port", &self.port)
      .finish()
  }
}

impl PartialEq for Config {
  fn eq(&self, other: &Self) -> bool {
    self.dir == other.dir && self.bind == other.bind && self.port == other.port
  }
}

impl Eq for Config {}

pub fn parse_args_from(args: &[String]) -> Result<Action, String> {
  let mut dir = PathBuf::from(".");
  let mut bind = String::from("0.0.0.0");
  let mut port: u16 = 8000;
  let mut port_set = false;

  let mut i = 0;

  while i < args.len() {
    match args[i].as_str() {
      "-d" | "--dir" => {
        i += 1;
        if i >= args.len() {
          return Err(format!("{} requires a value", args[i - 1]));
        }

        dir = PathBuf::from(&args[i]);
      }
      "-b" | "--bind" => {
        i += 1;
        if i >= args.len() {
          return Err(format!("{} requires a value", args[i - 1]));
        }
        bind = args[i].clone();
      }
      "-v" | "--version" => return Ok(Action::ShowVersion),
      "-h" | "--help" => return Ok(Action::ShowHelp),
      other => {
        if !other.starts_with('-') && !port_set {
          port = other
            .parse()
            .map_err(|_| format!("invalid port number: {}", other))?;
          port_set = true;
        } else {
          return Err(format!("unknown argument(s): {}", other));
        }
      }
    }
    i += 1;
  }

  let dir = fs::canonicalize(&dir)
    .map_err(|e| format!("cannot open directory '{}': {}", dir.display(), e))?;

  if !dir.is_dir() {
    return Err(format!("'{}' is not directory", dir.display()));
  }

  Ok(Action::Run(Config { dir, bind, port }))
}

pub fn parse_args() -> Config {
  let args: Vec<String> = env::args().skip(1).collect();
  match parse_args_from(&args) {
    Ok(Action::Run(config)) => config,
    Ok(Action::ShowHelp) => {
      print_help();
      process::exit(0);
    }
    Ok(Action::ShowVersion) => {
      print_version();
      process::exit(0);
    }
    Err(message) => {
      eprintln!("Error: {}", message);
      print_help();
      process::exit(1)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
  }

  #[test]
  fn pkg_name_and_version_are_not_empty() {
    assert!(!PKG_NAME.is_empty());
    assert!(!PKG_VERSION.is_empty());
  }

  #[test]
  fn defaults_when_no_args_given() {
    let action = parse_args_from(&args(&[])).expect("should parse with defaults");
    match action {
      Action::Run(config) => {
        assert_eq!(config.bind, "0.0.0.0");
        assert_eq!(config.port, 8000);
      }
      other => panic!("expected Action::Run, got {:?}", other),
    }
  }

  #[test]
  fn positional_port_is_parsed() {
    let action = parse_args_from(&args(&["3000"])).expect("should parse");
    match action {
      Action::Run(config) => assert_eq!(config.port, 3000),
      other => panic!("expected Action::Run, got {:?}", other),
    }
  }

  #[test]
  fn dir_and_bind_options_are_applied() {
    let tmp = std::env::temp_dir();
    let action = parse_args_from(&args(&[
      "8080",
      "-d",
      tmp.to_str().unwrap(),
      "-b",
      "127.0.0.1",
    ]))
    .expect("should parse");

    match action {
      Action::Run(config) => {
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind, "127.0.0.1");
        assert_eq!(config.dir, fs::canonicalize(tmp).unwrap());
      }
      other => panic!("expected Action::Run, got {:?}", other),
    }
  }

  #[test]
  fn options_can_come_before_or_after_the_port() {
    let tmp = std::env::temp_dir();
    let a = parse_args_from(&args(&["-d", tmp.to_str().unwrap(), "9000"])).unwrap();
    let b = parse_args_from(&args(&["9000", "-d", tmp.to_str().unwrap()])).unwrap();
    assert_eq!(a, b);
  }
  #[test]
  fn help_flag_short_and_long() {
    assert_eq!(parse_args_from(&args(&["-h"])), Ok(Action::ShowHelp));
    assert_eq!(parse_args_from(&args(&["--help"])), Ok(Action::ShowHelp))
  }
  #[test]
  fn version_flag_short_and_long() {
    assert_eq!(parse_args_from(&args(&["-v"])), Ok(Action::ShowVersion));
    assert_eq!(
      parse_args_from(&args(&["--version"])),
      Ok(Action::ShowVersion)
    )
  }
  #[test]
  fn invalid_port_is_an_error() {
    let err = parse_args_from(&args(&["not-a-port"])).unwrap_err();
    assert!(
      err.contains("invalid port number"),
      "unexpected message: {err}"
    );
  }

  #[test]
  fn unknown_flag_is_an_error() {
    let err = parse_args_from(&args(&["--nope"])).unwrap_err();
    assert!(
      err.contains("unknown argument"),
      "unexpected message: {err}"
    );
  }

  #[test]
  fn missing_value_for_dir_is_an_error() {
    let err = parse_args_from(&args(&["-d"])).unwrap_err();
    assert!(
      err.contains("requires a value"),
      "unexpected message: {err}"
    );
  }
  #[test]
  fn missing_value_for_bind_is_an_error() {
    let err = parse_args_from(&args(&["-b"])).unwrap_err();
    assert!(
      err.contains("requires a value"),
      "unexpected message: {err}"
    );
  }
  #[test]
  fn nonexistent_directory_is_an_error() {
    let err = parse_args_from(&args(&["-d", "/no/such/direcotry/hopefully"])).unwrap_err();
    assert!(
      err.contains("cannot open directory"),
      "unexpected message: {err}"
    );
  }
}
