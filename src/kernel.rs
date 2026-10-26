//! A kernel: a document of principles that later documents are checked against.
//!
//! The compiler already decides whether a claim is checkable. A kernel decides whether a claim
//! is *admissible here*: it names layers, and a claim inside one of those layers must stand on
//! one of the kernel's laws for that layer. It cannot refute what a claim says -- no declaration
//! language can -- but it can refuse to admit a claim as grounded, which is the only honest form
//! of "denied by the kernel".
//!
//! Two codes come from here:
//!
//! - `E-KERNEL-SHADOW` -- the target redefines a name the kernel already declares. The kernel is
//!   a prelude: its names are not up for renegotiation.
//! - `W-UNPRINCIPLED` -- the target makes a claim inside a governed layer without citing any of
//!   that layer's laws. It is a warning: the claim may be right, but it is not standing on the
//!   principle it claims to be inside.

use std::collections::{HashMap, HashSet};

use crate::check::Diagnostic;
use crate::program::{f_id, f_ids};
use crate::syntax::Decl;

pub fn check(kernel: &[Decl], target: &[Decl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let named: HashSet<&str> = kernel.iter().map(|decl| decl.name.as_str()).collect();

    for decl in target {
        if named.contains(decl.name.as_str()) {
            out.push(diag(
                decl,
                "E-KERNEL-SHADOW",
                format!(
                    "{} is declared by the kernel; a document checked against it may use the name but not redefine it",
                    decl.name
                ),
            ));
        }
    }

    // laws the kernel holds in each governed layer
    let mut governed: HashMap<&str, Vec<&str>> = HashMap::new();
    for law in kernel.iter().filter(|decl| decl.kind == "law") {
        if let Some(layer) = f_id(law, "holds_in") {
            governed.entry(layer).or_default().push(law.name.as_str());
        }
    }

    for decl in target {
        let Some(layer) = f_id(decl, "layer") else {
            continue;
        };
        let Some(laws) = governed.get(layer) else {
            continue;
        };
        if laws.is_empty() {
            continue;
        }
        let cited: Vec<String> = f_ids(decl, "from")
            .into_iter()
            .chain(f_ids(decl, "basis"))
            .chain(f_ids(decl, "checked_by"))
            .collect();
        if cited.iter().any(|name| laws.contains(&name.as_str())) {
            continue;
        }
        out.push(diag(
            decl,
            "W-UNPRINCIPLED",
            format!(
                "{} sits in layer {layer}, whose laws are {}, but stands on none of them",
                decl.name,
                laws.join(", ")
            ),
        ));
    }
    out
}

fn diag(decl: &Decl, code: &'static str, message: String) -> Diagnostic {
    Diagnostic {
        code,
        message,
        line: decl.line,
        col: decl.col,
        end_line: decl.end_line,
        end_col: decl.end_col,
        file: decl.file.clone(),
    }
}
