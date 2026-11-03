//! Command-line front end: one argument model, one dispatch path, one help table.
//!
//! The commands live in four files that follow the groups on the help screen -- checking,
//! reasoning, repairing, emitting.  Nothing here decides anything about a document; every command
//! loads, calls one library function and prints.  When a command starts to look like it has an
//! opinion, the opinion belongs in the library, not here.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::program::{self, Program};
use crate::syntax::{self, Decl, Val};
mod checking;
mod emitting;
mod reasoning;
mod repairing;

use checking::*;
use emitting::*;
use reasoning::*;
use repairing::*;

pub fn run() -> i32 {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if let Some(position) = raw.iter().position(|arg| arg == "--explain") {
        let Some(code) = raw.get(position + 1) else {
            println!("usage: tsare --explain CODE");
            return 2;
        };
        return match crate::explain::explain(code) {
            Some(text) => {
                println!("{code}\n");
                println!("{text}");
                0
            }
            None => {
                println!("no explanation for {code}");
                1
            }
        };
    }
    if raw.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!(
            "tsare {} (document format {})",
            env!("CARGO_PKG_VERSION"),
            crate::syntax::FORMAT_VERSION
        );
        return 0;
    }
    if raw.is_empty() || raw.iter().any(|arg| arg == "--help" || arg == "-h") {
        usage();
        return 0;
    }
    let (command, rest) = if is_command(&raw[0]) {
        (raw[0].clone(), raw[1..].to_vec())
    } else if looks_like_a_document(&raw[0]) {
        ("check".to_string(), raw.clone())
    } else {
        println!("unknown command {}; `tsare help` lists them", raw[0]);
        return 2;
    };
    let args = match Args::parse(&rest) {
        Ok(args) => args,
        Err(message) => {
            println!("{message}");
            return 2;
        }
    };
    if command == "mcp" {
        return crate::mcp::serve();
    }
    let Some(entry) = registry::find(&command) else {
        println!("unknown command {command}; `tsare help` lists them");
        return 2;
    };
    let path = if entry.needs_target {
        let Some(target) = args.at(0) else {
            println!("usage: tsare {command} FILE");
            return 2;
        };
        PathBuf::from(target)
    } else {
        PathBuf::new()
    };
    registry::dispatch(&path, &command, &args)
}

mod args;
mod help;
mod registry;
mod support;

use self::help::{is_command, looks_like_a_document, usage};

pub(crate) use self::args::Args;
pub(crate) use self::support::*;
