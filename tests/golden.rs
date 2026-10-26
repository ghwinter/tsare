//! Golden snapshots: the exact text every command prints for a fixture document.
//!
//! The snapshots were taken from the binary while it still agreed, command for command and
//! byte for byte, with the reference implementation in `obligo` (tag `v1`).  The Python is no
//! longer needed to run the compiler, but its output is still the standard here.
//!
//! Set `UPDATE_GOLDEN=1` to rewrite the snapshots after an intended change, then read the
//! diff before committing it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_tsare")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn documents() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("fixtures directory")
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| path.extension().map(|ext| ext == "mf").unwrap_or(false))
        .collect();
    out.sort();
    out
}

/// Paths are machine-specific, so the snapshots name the fixture directory instead.  JSON
/// escapes the separator, so both spellings are replaced before slashes are unified.
///
/// The prefixes are trimmed of a trailing separator first: on Windows `temp_dir()` ends with a
/// backslash and on unix it does not, so replacing the untrimmed string swallowed the separator
/// on one platform and kept it on the other.  Trimming makes both produce `<temp>/...`.
fn normalize(text: &str) -> String {
    let fixtures = fixtures().display().to_string();
    let temp = std::env::temp_dir().display().to_string();
    let mut out = text.to_string();
    for (needle, label) in [(&fixtures, "<fixtures>"), (&temp, "<temp>")] {
        let trimmed = needle.trim_end_matches(['\\', '/']);
        out = out.replace(trimmed, label);
        out = out.replace(&trimmed.replace('\\', "\\\\"), label);
    }
    out.replace('\\', "/")
        .replace("<fixtures>//", "<fixtures>/")
        .replace("<temp>//", "<temp>/")
}

fn run(args: &[&str]) -> String {
    let output = Command::new(binary())
        .args(args)
        .output()
        .expect("run the binary");
    normalize(&String::from_utf8_lossy(&output.stdout))
}

fn first_after(text: &str, marker: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(marker) {
            let name: String = rest
                .chars()
                .skip_while(|c| c.is_whitespace())
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

fn golden(name: &str, produced: &str) {
    let path = fixtures().join("golden").join(format!("{name}.txt"));
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("golden directory");
        std::fs::write(&path, produced).expect("write golden");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden {}: run with UPDATE_GOLDEN=1",
            path.display()
        )
    });
    if expected != produced {
        let expected_lines: Vec<&str> = expected.lines().collect();
        let produced_lines: Vec<&str> = produced.lines().collect();
        for index in 0..expected_lines.len().max(produced_lines.len()) {
            let left = expected_lines.get(index).copied().unwrap_or("<none>");
            let right = produced_lines.get(index).copied().unwrap_or("<none>");
            if left != right {
                panic!(
                    "{} differs at line {}\n  golden: {left:?}\n  now:    {right:?}",
                    path.display(),
                    index + 1
                );
            }
        }
        panic!("{} differs", path.display());
    }
}

fn stem(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().to_string()
}

const COMMANDS: &[(&str, &[&str])] = &[
    ("check", &[]),
    ("check-json", &["--json"]),
    ("engine-json", &["--json"]),
    ("suggest-json", &["--json"]),
    ("apply-dry", &["--dry-run"]),
    ("argue", &[]),
    ("derive", &[]),
    ("debt", &[]),
    ("debt-all", &["--all"]),
    ("impact", &[]),
    ("next", &[]),
    // `ask` is not in this matrix: no fixture has a gap or a pile, so its snapshot would be the
    // same six times.  The test below pins the interesting case; the module's unit tests pin
    // the rest.
    ("cost", &[]),
    ("weigh", &[]),
    ("cat", &[]),
    ("outline", &[]),
    ("moves", &[]),
    ("assumptions", &[]),
    ("audit", &[]),
    ("egraph", &[]),
    ("fix", &[]),
    ("induce", &[]),
];

#[test]
fn every_command_on_every_fixture() {
    for path in documents() {
        let shown = path.display().to_string();
        let name = stem(&path);
        for (label, flags) in COMMANDS {
            let mut args: Vec<&str> = vec![label.split('-').next().unwrap()];
            args.extend_from_slice(flags);
            args.push(&shown);
            golden(&format!("{name}.{label}"), &run(&args));
        }

        // whatif needs a declaration to remove, pattern needs a concept
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        if let Some(declaration) =
            first_after(&text, "fact ").or_else(|| first_after(&text, "warrant "))
        {
            golden(
                &format!("{name}.whatif"),
                &run(&["whatif", &shown, &declaration]),
            );
        }
        if let Some(concept) = first_after(&text, "concept ") {
            golden(
                &format!("{name}.pattern"),
                &run(&["pattern", &shown, &concept]),
            );
            golden(
                &format!("{name}.pattern-vars"),
                &run(&["pattern", &shown, &concept, "--vars"]),
            );
        }
    }
}

#[test]
fn graph_writes_the_same_network() {
    for path in documents() {
        let name = stem(&path);
        let out = std::env::temp_dir().join(format!("engine-golden-{name}.net.mmd"));
        let _ = std::fs::remove_file(&out);
        let produced = run(&[
            "graph",
            &path.display().to_string(),
            "--out",
            &out.display().to_string(),
        ]);
        let written = normalize(&std::fs::read_to_string(&out).unwrap_or_default());
        golden(&format!("{name}.graph-stdout"), &produced);
        golden(&format!("{name}.graph-file"), &written);
        let _ = std::fs::remove_file(&out);
    }
}

#[test]
fn build_emits_the_same_files() {
    for path in documents() {
        let name = stem(&path);
        let out = std::env::temp_dir().join(format!("engine-golden-build-{name}"));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        let produced = run(&[
            "build",
            &path.display().to_string(),
            "--out",
            &out.display().to_string(),
        ]);
        golden(&format!("{name}.build-stdout"), &produced);
        for file in [
            "OBLIGATIONS.md",
            "LEARNED.md",
            "OPEN.md",
            "NOTES.md",
            "predicates.py",
        ] {
            let text = std::fs::read_to_string(out.join(file)).unwrap_or_default();
            golden(&format!("{name}.build.{file}"), &normalize(&text));
        }
        let _ = std::fs::remove_dir_all(&out);
    }
}

#[test]
fn map_aligns_every_pair() {
    let documents = documents();
    for left in &documents {
        for right in &documents {
            if left >= right {
                continue;
            }
            let produced = run(&[
                "map",
                &left.display().to_string(),
                &right.display().to_string(),
            ]);
            if produced == "no shared structure\n" {
                continue;
            }
            golden(&format!("{}-{}.map", stem(left), stem(right)), &produced);
        }
    }
}

/// The help screen and the dispatch table are two lists, and a command that is only in the
/// first is treated as a path: `tsare ask doc.mf` then says the file `ask` does not exist.
/// This walks the help screen and fails on exactly that message.
#[test]
fn every_command_in_the_help_screen_is_dispatched() {
    let help = run(&["--help"]);
    // a copy, not the fixture: `graph` writes next to the document it read, and a test that
    // dirties the source tree is a test the CI cleanliness step then fails on.
    let dir = std::env::temp_dir().join("engine-golden-dispatch");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let probe = dir.join("probe.mf");
    std::fs::copy(&documents()[0], &probe).expect("copy the fixture");
    let fixture = probe.display().to_string();
    // `mcp` speaks a protocol on standard input and would wait for a client here
    let skip = ["mcp"];
    let mut names: Vec<String> = Vec::new();
    for line in help.lines() {
        // command lines are indented exactly two spaces; the usage block is indented further
        if !line.starts_with("  ") || line.starts_with("   ") {
            continue;
        }
        let name = line.split_whitespace().next().unwrap_or("");
        let shaped = !name.is_empty()
            && name.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            && !skip.contains(&name);
        if shaped {
            names.push(name.to_string());
        }
    }
    assert!(names.len() > 20, "the help screen lists the commands");
    for name in names {
        let output = Command::new(binary())
            .args([name.as_str(), fixture.as_str()])
            .output()
            .expect("run the binary");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !text.contains("cannot find the file"),
            "`{name}` is in --help but the dispatcher treated it as a path:\n{text}"
        );
    }
}

/// `imply` reports the laws the attribute table already holds, and refuses the ones a single
/// counterexample kills.  The five facts here are the falsifying test: `items=1` implies
/// `urgent=0`, and `urgent=0` does not imply `items=1`.
#[test]
fn imply_reports_the_laws_the_table_already_holds() {
    let dir = std::env::temp_dir().join("engine-golden-imply");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("imply.mf");
    std::fs::write(
        &path,
        r#"fact F_1 { claim: "1", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_2 { claim: "2", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_3 { claim: "3", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_4 { claim: "4", evidence: tool("t"), attrs: { items: 5, urgent: 1 } }
fact F_5 { claim: "5", evidence: tool("t"), attrs: { items: 5, urgent: 0 } }
"#,
    )
    .expect("write the document");
    let shown = path.display().to_string();
    golden(
        "imply.laws",
        &run(&[
            "imply",
            &shown,
            "--max-antecedent",
            "1",
            "--min-support",
            "2",
        ]),
    );
    golden(
        "imply.json",
        &run(&[
            "imply",
            &shown,
            "--json",
            "--max-antecedent",
            "1",
            "--min-support",
            "2",
        ]),
    );
}

/// `ask` needs a document with a gap, and no fixture has one: the six fixtures all list a
/// complete extent.  So the gap case is written here rather than added as a seventh fixture,
/// which would multiply the snapshots of every other command.
#[test]
fn ask_names_the_attribute_that_would_decide_a_missing_element() {
    let dir = std::env::temp_dir().join("engine-golden-ask");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("ask.mf");
    std::fs::write(
        &path,
        r#"fact F_A { claim: "a", evidence: tool("t"), attrs: { kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first } }
fact F_GAP { claim: "g", evidence: tool("t"), attrs: { kind: second } }
concept C_THINGS { name: "things", grounded_by: [F_A, F_B, F_GAP] }
partition P_SOME { dimension: C_THINGS, elements: [F_A, F_B], closure: open }
"#,
    )
    .expect("write the document");
    golden("ask.missing", &run(&["ask", &path.display().to_string()]));
}

/// A collision is the question with no partition in sight: two facts record the same evidence
/// and the same attributes.  `--json --one` is the shape a loop consumes, so it gets a
/// snapshot of its own.
#[test]
fn ask_answers_a_collision_in_json() {
    let dir = std::env::temp_dir().join("engine-golden-ask-json");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("collision.mf");
    std::fs::write(
        &path,
        r#"fact F_A { claim: "one", evidence: tool("survey"), attrs: { asked: 32, total: 40 } }
fact F_B { claim: "two", evidence: tool("survey"), attrs: { total: 40, asked: 32 } }
"#,
    )
    .expect("write the document");
    golden(
        "ask.collision-json",
        &run(&["ask", &path.display().to_string(), "--json", "--one"]),
    );
}

/// `novel` decides whether a proposed attribute is a new distinction or a recombination of the
/// recorded one.  Two facts with the same record are the witness; the snapshot pins both the
/// verdict and the sentence it prints.
#[test]
fn novel_decides_whether_a_predicate_is_new() {
    let dir = std::env::temp_dir().join("engine-golden-novel");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("novel.mf");
    std::fs::write(
        &path,
        r#"fact F_A { claim: "a", evidence: tool("t"), attrs: { size: 3, kind: first } }
fact F_B { claim: "b", evidence: tool("t"), attrs: { kind: first, size: 3 } }
fact F_C { claim: "c", evidence: tool("t"), attrs: { size: 9, kind: second } }
"#,
    )
    .expect("write the document");
    let shown = path.display().to_string();
    golden(
        "novel.new",
        &run(&["novel", &shown, "--attribute", "big", "--holds", "F_A"]),
    );
    golden(
        "novel.redundant",
        &run(&["novel", &shown, "--attribute", "big", "--holds", "F_A,F_B"]),
    );
}
