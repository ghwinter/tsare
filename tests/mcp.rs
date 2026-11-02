//! The MCP server, exercised the way a client would: JSON-RPC lines on standard input.
//!
//! The tools exist so a model can call the checker without a shell.  A tool that is listed but
//! not dispatched is worse than no tool, so the list and the dispatch are both exercised.

use std::io::Write;
use std::process::{Command, Stdio};

fn session(lines: &[String]) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tsare"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the mcp server");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for line in lines {
            writeln!(stdin, "{line}").expect("write a request");
        }
    }
    let output = child.wait_with_output().expect("wait");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn call(id: u32, name: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{name}","arguments":{arguments}}}}}"#
    )
}

#[test]
fn every_tool_it_lists_is_dispatched() {
    let out = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string(),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#.to_string(),
    ]);
    for name in [
        "check", "argue", "derive", "fix", "explain", "kinds", "ask", "novel", "imply",
    ] {
        assert!(
            out.contains(&format!(r#""name":"{name}""#)),
            "{name} is missing from tools/list:\n{out}"
        );
    }
    assert!(
        !out.contains("unknown tool"),
        "tools/list must not call anything:\n{out}"
    );
}

#[test]
fn the_judgement_tools_answer_with_text() {
    let dir = std::env::temp_dir().join("tsare-mcp-test");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("doc.tsare");
    std::fs::write(
        &path,
        r#"fact F_A { claim: "one", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_B { claim: "two", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_C { claim: "three", evidence: tool("t"), attrs: { items: 1, urgent: 0 } }
fact F_D { claim: "four", evidence: tool("t"), attrs: { items: 5, urgent: 0 } }
"#,
    )
    .expect("write the document");
    let shown = path.display().to_string().replace('\\', "\\\\");
    let out = session(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#.to_string(),
        call(2, "ask", &format!(r#"{{"path":"{shown}","one":true}}"#)),
        call(
            3,
            "novel",
            &format!(r#"{{"path":"{shown}","attribute":"big","holds":["F_A"]}}"#),
        ),
        call(
            4,
            "imply",
            &format!(r#"{{"path":"{shown}","min_support":2}}"#),
        ),
    ]);
    assert!(
        out.contains("question(s)") || out.contains("record the same"),
        "{out}"
    );
    assert!(
        out.contains("novel:") || out.contains("redundant:") || out.contains("definable"),
        "{out}"
    );
    assert!(out.contains("implication"), "{out}");
}
