//! A record of one run, and the replay that proves it is still the same run.
//!
//! A run is the ledger it read plus the commands it ran — `check`, `ask` and `decide` — with the
//! exact report and status of each.  The record embeds the source, so a later reader can re-run
//! the run and compare bytes without trusting the machine that wrote it.  Paths are replaced by
//! `<ledger>` on both sides: the record names content, not a place.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::hash::hex;
use crate::json::{self, Json};

/// The only format this build writes and reads.
pub const FORMAT: &str = "tsare-trace 2";

/// A trace larger than this is a corpus, not a document.
const MAX_LEDGER: usize = 1 << 20;

/// The commands a run records, in order, with the flags their report is compared on.
const STEPS: &[(&str, &[&str])] = &[
    ("check", &["--json"]),
    ("ask", &["--one"]),
    ("decide", &["--json"]),
];

/// One command in a run: what was run, the exact report, and the status.
pub struct Step {
    pub command: String,
    pub status: i32,
    pub report: String,
}

/// A run over one ledger: the source, its hash, and the steps in the order they ran.
pub struct Trace {
    pub ledger_sha256: String,
    pub ledger: String,
    pub steps: Vec<Step>,
}

impl Trace {
    pub fn new(ledger: &str, steps: Vec<Step>) -> Trace {
        Trace {
            ledger_sha256: digest(ledger),
            ledger: ledger.to_string(),
            steps,
        }
    }

    /// The trace as one JSON object, keys in a fixed order, so the bytes are stable.
    pub fn to_json(&self) -> String {
        let steps: Vec<String> = self
            .steps
            .iter()
            .map(|step| {
                format!(
                    "{{\"command\":\"{}\",\"status\":{},\"report\":\"{}\"}}",
                    escape(&step.command),
                    step.status,
                    escape(&step.report),
                )
            })
            .collect();
        format!(
            "{{\"format\":\"{}\",\"ledger_sha256\":\"{}\",\"ledger\":\"{}\",\"steps\":[{}]}}\n",
            escape(FORMAT),
            escape(&self.ledger_sha256),
            escape(&self.ledger),
            steps.join(","),
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
        if digest(&ledger) != ledger_sha256 {
            return Err("the embedded ledger does not match its hash".to_string());
        }
        let steps = value
            .get("steps")
            .and_then(Json::as_arr)
            .ok_or("the record has no steps")?;
        if steps.is_empty() {
            return Err("the record has no steps".to_string());
        }
        let mut out = Vec::new();
        for step in steps {
            let command = step
                .get("command")
                .and_then(Json::as_str)
                .ok_or("a step names no command")?
                .to_string();
            let status = step
                .get("status")
                .and_then(Json::as_f64)
                .ok_or("a step has no status")? as i32;
            let report = step
                .get("report")
                .and_then(Json::as_str)
                .ok_or("a step embeds no report")?
                .to_string();
            out.push(Step {
                command,
                status,
                report,
            });
        }
        Ok(Trace {
            ledger_sha256,
            ledger,
            steps: out,
        })
    }
}

/// The outcome of a replay: whether every report and status came back the same.
pub struct Replay {
    pub matched: bool,
    pub detail: String,
}

/// Run the recorded steps on `path` and record the ledger with them.
pub fn record(path: &Path) -> Result<Trace, String> {
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
    let mut steps = Vec::new();
    for &(command, flags) in STEPS {
        let output = run(&exe, command, flags, path)?;
        let status = output.status.code().unwrap_or(-1);
        let report = normalize(&String::from_utf8_lossy(&output.stdout), path);
        steps.push(Step {
            command: command.to_string(),
            status,
            report,
        });
    }
    Ok(Trace::new(&ledger, steps))
}

/// Re-run every recorded step on the embedded ledger and compare.  The path is a temporary file,
/// so it is normalized away on both sides; what is compared is each report and status.
pub fn replay(trace: &Trace) -> Result<Replay, String> {
    let exe =
        std::env::current_exe().map_err(|error| format!("cannot find the checker: {error}"))?;
    let suffix: String = trace.ledger_sha256.chars().take(16).collect();
    let path = std::env::temp_dir().join(format!(
        "tsare-replay-{}-{}.tsare",
        if suffix.is_empty() { "0" } else { &suffix },
        std::process::id()
    ));
    fs::write(&path, &trace.ledger).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut outcome = Ok(Replay {
        matched: true,
        detail: String::new(),
    });
    for step in &trace.steps {
        let Some(&(command, flags)) = STEPS
            .iter()
            .find(|(name, _)| *name == step.command.as_str())
        else {
            outcome = Err(format!(
                "the record runs `{}`, which this build cannot replay",
                step.command
            ));
            break;
        };
        let output = match run(&exe, command, flags, &path) {
            Ok(output) => output,
            Err(error) => {
                outcome = Err(error);
                break;
            }
        };
        let status = output.status.code().unwrap_or(-1);
        let actual = normalize(&String::from_utf8_lossy(&output.stdout), &path);
        if status != step.status {
            outcome = Ok(Replay {
                matched: false,
                detail: format!(
                    "{}: status {status}, recorded {}",
                    step.command, step.status
                ),
            });
            break;
        }
        if actual != step.report {
            outcome = Ok(Replay {
                matched: false,
                detail: format!("{}: {}", step.command, first_diff(&step.report, &actual)),
            });
            break;
        }
    }
    let _ = fs::remove_file(&path);
    outcome
}

fn run(exe: &Path, command: &str, flags: &[&str], path: &Path) -> Result<Output, String> {
    Command::new(exe)
        .arg(command)
        .arg(path)
        .args(flags)
        .output()
        .map_err(|error| format!("cannot run `{command}`: {error}"))
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

    fn step(command: &str, status: i32, report: &str) -> Step {
        Step {
            command: command.to_string(),
            status,
            report: report.to_string(),
        }
    }

    #[test]
    fn a_trace_survives_a_round_trip() {
        let trace = Trace::new(
            "tsare 1\nfact F { claim: \"x\", evidence: user }\n",
            vec![
                step("check", 0, "{\"errors\":0}\n"),
                step("ask", 0, "nothing to ask\n"),
                step("decide", 0, "{\"decision\":\"closed\"}\n"),
            ],
        );
        let text = trace.to_json();
        let read = Trace::parse(&text).expect("the trace parses");
        assert_eq!(read.ledger, trace.ledger);
        assert_eq!(read.ledger_sha256, trace.ledger_sha256);
        assert_eq!(read.steps.len(), 3);
        assert_eq!(read.steps[0].command, "check");
        assert_eq!(read.steps[2].report, "{\"decision\":\"closed\"}\n");
    }

    #[test]
    fn a_ledger_that_no_longer_matches_its_hash_is_refused() {
        let trace = Trace::new("tsare 1\n", vec![step("check", 0, "report\n")]);
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

    #[test]
    fn a_trace_without_steps_is_refused() {
        let trace = Trace::new("tsare 1\n", Vec::new());
        let error = Trace::parse(&trace.to_json())
            .err()
            .expect("steps are required");
        assert!(error.contains("no steps"), "{error}");
    }
}
