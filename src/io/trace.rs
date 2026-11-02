//! A record of one or more runs, and the replay that proves they are still the same runs.
//!
//! A run is a ledger plus the commands the loop reads it with — `check`, `ask`, `decide` — each
//! with the exact report and status.  A session is the runs in order: `trace --append` adds one,
//! `replay` re-runs them all.  The ledger is embedded and its hash verified, so the record names
//! content, not a place; paths are replaced by `<ledger>` on both sides.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use crate::hash::hex;
use crate::json::{self, Json};

/// The only format this build writes and reads.
pub const FORMAT: &str = "tsare-trace 3";

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

/// One ledger and the steps that read it.
pub struct Run {
    pub ledger_sha256: String,
    pub ledger: String,
    pub steps: Vec<Step>,
}

impl Run {
    pub fn new(ledger: &str, steps: Vec<Step>) -> Run {
        Run {
            ledger_sha256: digest(ledger),
            ledger: ledger.to_string(),
            steps,
        }
    }
}

/// A session: the runs in order, and the tool that wrote them.
pub struct Trace {
    pub tool: String,
    pub runs: Vec<Run>,
}

impl Trace {
    pub fn new(run: Run) -> Trace {
        Trace {
            tool: env!("CARGO_PKG_VERSION").to_string(),
            runs: vec![run],
        }
    }

    pub fn push(&mut self, run: Run) {
        self.runs.push(run);
    }

    /// The trace as one JSON object, keys in a fixed order, so the bytes are stable.
    pub fn to_json(&self) -> String {
        let runs: Vec<String> = self
            .runs
            .iter()
            .map(|run| {
                let steps: Vec<String> = run
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
                    "{{\"ledger_sha256\":\"{}\",\"ledger\":\"{}\",\"steps\":[{}]}}",
                    escape(&run.ledger_sha256),
                    escape(&run.ledger),
                    steps.join(","),
                )
            })
            .collect();
        format!(
            "{{\"format\":\"{}\",\"tool\":\"{}\",\"runs\":[{}]}}\n",
            escape(FORMAT),
            escape(&self.tool),
            runs.join(","),
        )
    }

    /// Read a trace, refuse one this build did not write, and refuse any run whose ledger does not
    /// match its recorded hash: the hash is what makes the embedded source the source.
    pub fn parse(text: &str) -> Result<Trace, String> {
        let value = json::parse(text).map_err(|_| "the file is not JSON".to_string())?;
        let format = value.get("format").and_then(Json::as_str).unwrap_or("");
        if format != FORMAT {
            return Err(format!("the record is not a {FORMAT} trace"));
        }
        let tool = value
            .get("tool")
            .and_then(Json::as_str)
            .ok_or("the record names no tool")?
            .to_string();
        let runs = value
            .get("runs")
            .and_then(Json::as_arr)
            .ok_or("the record has no runs")?;
        if runs.is_empty() {
            return Err("the record has no runs".to_string());
        }
        let mut parsed = Vec::new();
        for run in runs {
            let ledger_sha256 = run
                .get("ledger_sha256")
                .and_then(Json::as_str)
                .ok_or("a run has no ledger hash")?
                .to_string();
            let ledger = run
                .get("ledger")
                .and_then(Json::as_str)
                .ok_or("a run embeds no ledger")?
                .to_string();
            if digest(&ledger) != ledger_sha256 {
                return Err("an embedded ledger does not match its hash".to_string());
            }
            let steps = run
                .get("steps")
                .and_then(Json::as_arr)
                .ok_or("a run has no steps")?;
            if steps.is_empty() {
                return Err("a run has no steps".to_string());
            }
            let mut parsed_steps = Vec::new();
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
                parsed_steps.push(Step {
                    command,
                    status,
                    report,
                });
            }
            parsed.push(Run {
                ledger_sha256,
                ledger,
                steps: parsed_steps,
            });
        }
        Ok(Trace { tool, runs: parsed })
    }
}

/// The outcome of a replay: whether every run, step, report and status came back the same.
pub struct Replay {
    pub matched: bool,
    pub detail: String,
}

/// Run the recorded steps on `path` and record the ledger with them.
pub fn record(path: &Path) -> Result<Run, String> {
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
        let output = spawn(&exe, command, flags, path)?;
        let status = output.status.code().unwrap_or(-1);
        let report = normalize(&String::from_utf8_lossy(&output.stdout), path);
        steps.push(Step {
            command: command.to_string(),
            status,
            report,
        });
    }
    Ok(Run::new(&ledger, steps))
}

/// Re-run every run in order and compare.  The first difference names the run and the step.
pub fn replay(trace: &Trace) -> Result<Replay, String> {
    let exe =
        std::env::current_exe().map_err(|error| format!("cannot find the checker: {error}"))?;
    for (index, run) in trace.runs.iter().enumerate() {
        if let Some(detail) = replay_run(&exe, run, index)? {
            let mut text = format!("run {} {detail}", index + 1);
            if trace.tool != env!("CARGO_PKG_VERSION") {
                text.push_str(&format!(
                    " (recorded with {}, this build is {})",
                    trace.tool,
                    env!("CARGO_PKG_VERSION")
                ));
            }
            return Ok(Replay {
                matched: false,
                detail: text,
            });
        }
    }
    Ok(Replay {
        matched: true,
        detail: String::new(),
    })
}

/// Re-run one run on a temporary copy of its ledger; `Some` names the first step that differs.
fn replay_run(exe: &Path, run: &Run, index: usize) -> Result<Option<String>, String> {
    let suffix: String = run.ledger_sha256.chars().take(16).collect();
    let path = std::env::temp_dir().join(format!(
        "tsare-replay-{}-{}-{}.tsare",
        if suffix.is_empty() { "0" } else { &suffix },
        std::process::id(),
        index
    ));
    fs::write(&path, &run.ledger).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut detail = None;
    for step in &run.steps {
        let Some(&(command, flags)) = STEPS
            .iter()
            .find(|(name, _)| *name == step.command.as_str())
        else {
            detail = Some(format!("`{}` cannot be replayed", step.command));
            break;
        };
        let output = match spawn(exe, command, flags, &path) {
            Ok(output) => output,
            Err(error) => {
                let _ = fs::remove_file(&path);
                return Err(error);
            }
        };
        let status = output.status.code().unwrap_or(-1);
        let actual = normalize(&String::from_utf8_lossy(&output.stdout), &path);
        if status != step.status {
            detail = Some(format!(
                "{}: status {status}, recorded {}",
                step.command, step.status
            ));
            break;
        }
        if actual != step.report {
            detail = Some(format!(
                "{}: {}",
                step.command,
                first_diff(&step.report, &actual)
            ));
            break;
        }
    }
    let _ = fs::remove_file(&path);
    Ok(detail)
}

fn spawn(exe: &Path, command: &str, flags: &[&str], path: &Path) -> Result<Output, String> {
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

    fn run(ledger: &str) -> Run {
        Run::new(
            ledger,
            vec![
                step("check", 0, "{\"errors\":0}\n"),
                step("ask", 0, "nothing to ask\n"),
                step("decide", 0, "{\"decision\":\"closed\"}\n"),
            ],
        )
    }

    #[test]
    fn a_session_survives_a_round_trip() {
        let mut trace = Trace::new(run("tsare 1\nfact F { claim: \"x\", evidence: user }\n"));
        trace.push(run("tsare 1\nfact F { claim: \"x\", evidence: user }\nfact G { claim: \"y\", evidence: user }\n"));
        let text = trace.to_json();
        let read = Trace::parse(&text).expect("the trace parses");
        assert_eq!(read.tool, trace.tool);
        assert_eq!(read.runs.len(), 2);
        assert_eq!(read.runs[0].steps[0].command, "check");
        assert_eq!(read.runs[1].steps[2].report, "{\"decision\":\"closed\"}\n");
        assert_ne!(read.runs[0].ledger_sha256, read.runs[1].ledger_sha256);
    }

    #[test]
    fn a_ledger_that_no_longer_matches_its_hash_is_refused() {
        let trace = Trace::new(run("tsare 1\n"));
        let tampered = "tsare 1\\nnote N { tag: conjecture, text: \\\"x\\\" }\\n";
        let text = trace.to_json().replace("tsare 1\\n", tampered);
        let error = Trace::parse(&text).err().expect("the hash must be checked");
        assert!(error.contains("does not match"), "{error}");
    }

    #[test]
    fn a_trace_without_runs_is_refused() {
        let trace = Trace {
            tool: "0.0.0".to_string(),
            runs: Vec::new(),
        };
        let error = Trace::parse(&trace.to_json())
            .err()
            .expect("runs are required");
        assert!(error.contains("no runs"), "{error}");
    }

    #[test]
    fn a_run_without_steps_is_refused() {
        let trace = Trace {
            tool: "0.0.0".to_string(),
            runs: vec![Run::new("tsare 1\n", Vec::new())],
        };
        let error = Trace::parse(&trace.to_json())
            .err()
            .expect("steps are required");
        assert!(error.contains("no steps"), "{error}");
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
