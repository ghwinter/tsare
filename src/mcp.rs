//! A minimal MCP server over stdio: `initialize`, `tools/list`, `tools/call`.
//!
//! Model Context Protocol servers speak JSON-RPC 2.0, one object per line, on standard
//! input and output.  This server exposes the deterministic operations and nothing else: no
//! model is called, and every tool answers with the same text the command line prints.

use std::io::{BufRead, Write};

use crate::json::{self, Json};

fn reply(id: &Json, result: String) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{result}}}",
        json::serialize(id)
    )
}

fn error(id: &Json, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{code},\"message\":\"{}\"}}}}",
        json::serialize(id),
        crate::cli::escape(message)
    )
}

fn text_result(text: String, failed: bool) -> String {
    format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":\"{}\"}}],\"isError\":{failed}}}",
        crate::cli::escape(&text)
    )
}

const TOOLS: &str = r#"{"tools":[
{"name":"check","description":"Parse and check a .mf document or directory; returns the diagnostics as text.","inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}},
{"name":"argue","description":"Which conclusions survive attack.","inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}},
{"name":"derive","description":"Which conclusions are grounded in facts.","inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}},
{"name":"fix","description":"Verified repair candidates, with what each would cost.","inputSchema":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}},
{"name":"explain","description":"What a diagnostic code means and the smallest fix.","inputSchema":{"type":"object","properties":{"code":{"type":"string"}},"required":["code"]}},
{"name":"kinds","description":"The obligation table: what each declaration kind owes.","inputSchema":{"type":"object","properties":{}}}
]}"#;

fn argument<'a>(params: &'a Json, key: &str) -> Option<&'a str> {
    params
        .get("arguments")
        .and_then(|arguments| arguments.get(key))
        .and_then(|value| value.as_str())
}

fn call(name: &str, params: &Json) -> Result<String, (String, bool)> {
    match name {
        "kinds" => {
            let mut out = String::new();
            for (kind, obligation, discharge) in crate::program::TABLE {
                out.push_str(&format!("{kind}: {obligation} — {discharge}\n"));
            }
            Ok(out)
        }
        "explain" => {
            let code = argument(params, "code").unwrap_or("");
            match crate::explain::explain(code) {
                Some(text) => Ok(format!("{code}\n\n{text}\n")),
                None => Err((format!("no explanation for {code}"), true)),
            }
        }
        other => {
            let path = argument(params, "path").unwrap_or("");
            let loaded =
                crate::cli::load(std::path::Path::new(path)).map_err(|error| (error, true))?;
            let program = crate::program::Program::new(&loaded.decls);
            match other {
                "check" => {
                    let mut session = crate::session::Session::new(path.to_string())
                        .with_source(&loaded.text)
                        .with_sources(&loaded.sources);
                    session.extend(crate::check::check(&loaded.decls));
                    let mut out = session.text();
                    out.push_str(&session.summary());
                    out.push('\n');
                    Ok(out)
                }
                "argue" => {
                    let rows = crate::argue::conclusions(&program);
                    let mut out = String::new();
                    for row in rows {
                        out.push_str(&format!(
                            "{:<24} {:<10} {}\n",
                            row.name, row.status, row.why
                        ));
                    }
                    Ok(out)
                }
                "derive" => {
                    let analysis = crate::derive::analyze(&program);
                    let mut out = format!(
                        "grounded: {} proposition(s) from {} facts\n",
                        analysis.grounded.len(),
                        analysis.facts.len()
                    );
                    for name in &analysis.grounded {
                        out.push_str(&format!("  derived {name}\n"));
                    }
                    if !analysis.conditional.is_empty() {
                        out.push_str(&format!(
                            "supported but not grounded: {}\n",
                            analysis.conditional.len()
                        ));
                        for name in &analysis.conditional {
                            out.push_str(&format!("  {name}\n"));
                        }
                    }
                    Ok(out)
                }
                "fix" => {
                    let (accepted, (errors, warnings)) = crate::fix::verified(&loaded.decls);
                    let mut out = format!("before: {errors} errors, {warnings} warnings\n");
                    if accepted.is_empty() {
                        out.push_str("no verified candidate reduces this\n");
                    }
                    for item in accepted {
                        out.push_str(&format!(
                            "  + {}  ({:+} errors, {:+} warnings)\n",
                            item.description, item.delta.0, item.delta.1
                        ));
                    }
                    Ok(out)
                }
                unknown => Err((format!("unknown tool {unknown}"), true)),
            }
        }
    }
}

pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { return 0 };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(request) = json::parse(&line) else {
            let _ = writeln!(stdout, "{}", error(&Json::Null, -32700, "parse error"));
            continue;
        };
        let id = request.get("id").cloned().unwrap_or(Json::Null);
        let method = request
            .get("method")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Json::Null);
        let response = match method {
            "initialize" => reply(
                &id,
                format!(
                    "{{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{{\"tools\":{{}}}},\"serverInfo\":{{\"name\":\"tsare\",\"version\":\"{}\"}}}}",
                    env!("CARGO_PKG_VERSION")
                ),
            ),
            "tools/list" => reply(&id, TOOLS.to_string()),
            "tools/call" => {
                let name = params.get("name").and_then(|value| value.as_str()).unwrap_or("");
                match call(name, &params) {
                    Ok(text) => reply(&id, text_result(text, false)),
                    Err((text, failed)) => reply(&id, text_result(text, failed)),
                }
            }
            other if other.starts_with("notifications/") => continue,
            other => error(&id, -32601, &format!("unknown method {other}")),
        };
        if writeln!(stdout, "{response}").is_err() {
            return 1;
        }
        let _ = stdout.flush();
    }
    0
}
