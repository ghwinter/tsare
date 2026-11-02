//! A record of one check run, and the replay that proves it is still the same run.
//!
//! The record embeds the ledger it read and the report it produced, so a later reader can run the
//! same command again and compare bytes without trusting the machine that wrote the record.  A
//! path is replaced by `<ledger>` on both sides: the record names content, not a place.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::hash::hex;
use crate::json::{self, Json};

/// The only format this build writes and reads.
pub const FORMAT: &str = "tsare-trace 1";

/// A trace larger than this is a corpus, not a document.
const MAX_LEDGER: usize = 1 << 20;

/// One check run: the source, the exact report, and the status it exited with.
pub struct Trace {
    pub command: String,
    pub status: i32,
    pub ledger_sha256: String,
    pub ledger: String,
    pub report: String,
}

impl Trace {
    pub fn new(command: &str, ledger: &str, report: &str, status: i32) -> Trace {
        Trace {
            command: command.to_string(),
            status,
            ledger_sha256: digest(ledger),
            ledger: ledger.to_string(),
            report: report.to_string(),
        }
    }

    /// The trace as one JSON object, keys in a fixed order, so the bytes are stable.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"format\":\"{}\",\"command\":\"{}\",\"status\":{},\"ledger_sha256\":\"{}\",\"ledger\":\"{}\",\"report\":\"{}\"}}\n",
            escape(FORMAT),
            escape(&self.command),
            self.status,
            escape(&self.ledger_sha256),
            escape(&self.ledger),
            escape(&self.report),
        )
    }

    /// Read a trace, refuse one this build did not write, and refuse a ledger that does not match
    /// its recorded hash: the hash is what makes the embedded source the source.
    pub fn parse(text: &str) -> Result<Trace, String> {
        let value = json::parse(text).map_err(|_| "the file is not JSON".to_string())?;
        let format = value.get("format").and_then(Json::as_str).unwrap_or("");
        if format != FORMAT {
            return Err(format!("the record is not a {FORMAT} trace"));
        }
        let command = value
            .get("command")
            .and_then(Json::as_str)
            .ok_or("the record names no command")?
            .to_string();
        let status = value
            .get("status")
            .and_then(Json::as_f64)
            .ok_or("the record has no status")? as i32;
        let ledger_sha256 = value
            .get("ledger_sha256")
            .and_then(Json::as_str)
            .ok_or("the record has no ledger hash")?
            .to_string();
        let ledger = value
            .get("ledger")
            .and_then(Json::as_str)
            .ok_or("the record embeds no ledger")?
            .to_string();
        let report = value
            .get("report")
            .and_then(Json::as_str)
            .ok_or("the record embeds no report")?
            .to_string();
        if digest(&ledger) != ledger_sha256 {
            return Err("the embedded ledger does not match its hash".to_string());
        }
        Ok(Trace {
            command,
            status,
            ledger_sha256,
            ledger,
            report,
        })
    }
}

/// The outcome of a replay: whether the report and the status came back the same.
pub struct Replay {
    pub matched: bool,
    pub detail: String,
}

/// Run `check --json` on `path` and record it.
pub fn record_check(path: &Path) -> Result<Trace, String> {
    let ledger =
        fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if ledger.len() > MAX_LEDGER {
        return Err(format!(
            "{}: the ledger is over {MAX_LEDGER} bytes; a trace is for a document, not a corpus",
            path.display()
        ));
    }
    let exe =
        std::env::current_exe().map_err(|error| format!("cannot find the checker: {error}"))?;
    let output = Command::new(exe)
        .arg("check")
        .arg(path)
        .arg("--json")
        .output()
        .map_err(|error| format!("cannot run the checker: {error}"))?;
    let status = output.status.code().unwrap_or(-1);
    let report = normalize(&String::from_utf8_lossy(&output.stdout), path);
    Ok(Trace::new("check", &ledger, &report, status))
}

/// Re-run the recorded command on the embedded ledger and compare.  The path is a temporary file,
/// so it is normalized away on both sides; what is compared is the report and the status.
pub fn replay(trace: &Trace) -> Result<Replay, String> {
    if trace.command != "check" {
        return Err(format!(
            "the record runs `{}`, which this build cannot replay",
            trace.command
        ));
    }
    let exe =
        std::env::current_exe().map_err(|error| format!("cannot find the checker: {error}"))?;
    let suffix: String = trace.ledger_sha256.chars().take(16).collect();
    let path = std::env::temp_dir().join(format!(
        "tsare-replay-{}-{}.tsare",
        if suffix.is_empty() { "0" } else { &suffix },
        std::process::id()
    ));
    fs::write(&path, &trace.ledger).map_err(|error| format!("{}: {error}", path.display()))?;
    let output = Command::new(exe)
        .arg("check")
        .arg(&path)
        .arg("--json")
        .output();
    let _ = fs::remove_file(&path);
    let output = output.map_err(|error| format!("cannot run the checker: {error}"))?;
    let status = output.status.code().unwrap_or(-1);
    let actual = normalize(&String::from_utf8_lossy(&output.stdout), &path);
    if status != trace.status {
        return Ok(Replay {
            matched: false,
            detail: format!("status {status}, recorded {}", trace.status),
        });
    }
    if actual != trace.report {
        return Ok(Replay {
            matched: false,
            detail: first_diff(&trace.report, &actual),
        });
    }
    Ok(Replay {
        matched: true,
        detail: String::new(),
    })
}

/// Replace the path the report prints, in every spelling JSON gives it, with a stable placeholder.
fn normalize(text: &str, path: &Path) -> String {
    let raw = path.display().to_string();
    let trimmed = raw.trim_end_matches(['\\', '/']);
    let mut out = text.to_string();
    for form in [
        trimmed.to_string(),
        trimmed.replace('\\', "/"),
        trimmed.replace('/', "\\"),
    ] {
        if form.is_empty() {
            continue;
        }
        out = out.replace(&form.replace('\\', "\\\\"), "<ledger>");
        out = out.replace(&form, "<ledger>");
    }
    out
}

/// The first line that differs, so a mismatch names something a reader can look at.
fn first_diff(expected: &str, actual: &str) -> String {
    for (index, (left, right)) in expected.lines().zip(actual.lines()).enumerate() {
        if left != right {
            return format!(
                "line {}:\n  recorded {}\n  replayed {}",
                index + 1,
                left,
                right
            );
        }
    }
    format!(
        "the reports differ after {} line(s)",
        expected.lines().count().min(actual.lines().count())
    )
}

fn digest(text: &str) -> String {
    hex(text.as_bytes())
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if (other as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", other as u32)),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trace_survives_a_round_trip() {
        let trace = Trace::new(
            "check",
            "tsare 1\nfact F { claim: \"x\", evidence: user }\n",
            "{\"errors\":0}\n",
            0,
        );
        let text = trace.to_json();
        let read = Trace::parse(&text).expect("the trace parses");
        assert_eq!(read.command, "check");
        assert_eq!(read.status, 0);
        assert_eq!(read.ledger, trace.ledger);
        assert_eq!(read.report, trace.report);
        assert_eq!(read.ledger_sha256, trace.ledger_sha256);
    }

    #[test]
    fn a_ledger_that_no_longer_matches_its_hash_is_refused() {
        let trace = Trace::new("check", "tsare 1\n", "report\n", 0);
        let tampered = "tsare 1\\nnote N { tag: conjecture, text: \\\"x\\\" }\\n";
        let text = trace.to_json().replace("tsare 1\\n", tampered);
        let error = Trace::parse(&text).err().expect("the hash must be checked");
        assert!(error.contains("does not match"), "{error}");
    }

    #[test]
    fn a_path_is_replaced_in_every_spelling() {
        let path = Path::new("/tmp/work/doc.tsare");
        assert_eq!(
            normalize("file \"/tmp/work/doc.tsare\"", path),
            "file \"<ledger>\""
        );
        let sep = '\\';
        let text = format!("file \"work{sep}{sep}doc.tsare\"");
        let windows = format!("work{sep}doc.tsare");
        assert_eq!(normalize(&text, Path::new(&windows)), "file \"<ledger>\"");
    }

    #[test]
    fn the_first_difference_is_a_line() {
        let diff = first_diff("a\nb\nc\n", "a\nB\nc\n");
        assert!(diff.contains("line 2"), "{diff}");
        assert!(diff.contains("recorded b"), "{diff}");
        assert!(diff.contains("replayed B"), "{diff}");
    }
}
