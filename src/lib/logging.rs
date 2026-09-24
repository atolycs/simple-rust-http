use chrono::{DateTime, Local};
use std::path::Path;

pub fn display_path(path: &Path) -> String {
  let path = path.to_string_lossy();

  #[cfg(windows)]
  {
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
      return format!(r"\\{}", path);
    }

    if let Some(path) = path.strip_prefix(r"\\?\") {
      return path.to_string();
    }
  }
  path.into_owned()
}

// fn civil_from_days(z: i64) -> (i64, u32, u32) {
//   let z = z + 719468;
//   let era = if z >= 0 { z } else { z - 146096 } / 146097;
//   let doe = (z - era * 146097) as u64;
//   let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
//   let y = yoe as i64 + era * 400;
//   let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
//   let mp = (5 * doy + 2) / 153;
//   let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
//   let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
//   let y = if m <= 2 { y + 1 } else { y };
//   (y, m, d)
// }
//
// fn clf_timestamp(secs: i64) -> String {
//   const MONTHS: [&str; 12] = [
//     "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
//   ];
//
//   let days = secs.div_euclid(86400);
//   let secs_of_day = secs.rem_euclid(86400);
//   let (y, m, d) = civil_from_days(days);
//   let hh = secs_of_day / 3600;
//   let mm = (secs_of_day % 3600) / 60;
//   let ss = secs_of_day % 60;
//
//   format!(
//     "{:02}/{}/{:04}:{:02}:{:02}:{:02} +0000",
//     d,
//     MONTHS[(m - 1) as usize],
//     y,
//     hh,
//     mm,
//     ss
//   )
// }

// fn clf_timestamp_now() -> String {
//   let secs = SystemTime::now()
//     .duration_since(UNIX_EPOCH)
//     .map(|d| d.as_secs())
//     .unwrap_or(0) as i64;
//   clf_timestamp(secs)
// }
// fn clf_timestamp_now() -> String {
//   Local::now().format("%d/%b/%Y:%H:%M:%S %z").to_string()
// }

fn clf_timestamp_now() -> String {
  format_clf_timestamp(Local::now())
}

fn format_clf_timestamp(dt: DateTime<Local>) -> String {
  dt.format("%d/%b/%Y:%H:%M:%S %z").to_string()
}

pub fn format_access_log_line(
  timestamp: &str,
  client: &str,
  method: &str,
  path: &str,
  status: &str,
  body_len: usize,
  user_agent: &str,
) -> String {
  let status_code = status.split_whitespace().next().unwrap_or("-");
  format!(
    "{} - - [{}] \"{} {} HTTP/1.1\" {} {} \"-\" \"{}\"",
    client, timestamp, method, path, status_code, body_len, user_agent
  )
}

pub fn log_access(
  client: &str,
  method: &str,
  path: &str,
  status: &str,
  body_len: usize,
  user_agent: &str,
) {
  println!(
    "{}",
    format_access_log_line(
      &clf_timestamp_now(),
      client,
      method,
      path,
      status,
      body_len,
      user_agent
    )
  );
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::TimeZone;

  #[test]
  fn format_clf_timestamp_known_value() {
    let dt = Local.with_ymd_and_hms(2026, 9, 23, 8, 3, 36).unwrap();
    let formatted = format_clf_timestamp(dt);

    assert!(formatted.starts_with("23/Sep/2026:08:03:36"))
  }

  #[test]
  fn clf_timestamp_now_has_expecte_shape() {
    let ts = clf_timestamp_now();
    let parts: Vec<&str> = ts.split(' ').collect();
    assert_eq!(parts.len(), 2, "expected \"<date> <offset>\", got: {ts}");
    assert_eq!(parts[1].len(), 5, "offset should look like +0900: {ts}");

    let date_parts: Vec<&str> = parts[0].split(':').collect();
    assert_eq!(
      date_parts.len(),
      4,
      "expected DD/Mon/YYYY:HH:MM:SS, got: {ts}"
    );
  }
  #[test]
  fn log_access_does_not_panic() {
    log_access(
      "127.0.0.1:12345",
      "GET",
      "/index.html",
      "200 OK",
      1234,
      "curl/8.5.0",
    );
    log_access("-", "-", "-", "400 Bad Request", 0, "-");
  }
}
