//! The operator contracts: what each command receives, returns and touches.
//!
//! Invariant 2: every operator declares an input, an output and an effect, so a caller can see
//! what a command will read or write before running it.

use crate::json::Json as J;

/// What an operator touches beyond its inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Pure,
    Read,
    Write,
    ReadWrite,
}

impl Effect {
    pub fn label(self) -> &'static str {
        match self {
            Effect::Pure => "pure",
            Effect::Read => "read",
            Effect::Write => "write",
            Effect::ReadWrite => "readwrite",
        }
    }
}

/// One operator's contract.
pub struct Contract {
    pub name: &'static str,
    pub input: &'static str,
    pub output: &'static str,
    pub effect: Effect,
}

pub const CONTRACTS: &[Contract] = &[
    Contract {
        name: "check",
        input: "file",
        output: "diagnostics",
        effect: Effect::Read,
    },
    Contract {
        name: "status",
        input: "file",
        output: "report",
        effect: Effect::Read,
    },
    Contract {
        name: "decide",
        input: "file",
        output: "verdict",
        effect: Effect::Read,
    },
    Contract {
        name: "epistemic",
        input: "file",
        output: "rows",
        effect: Effect::Read,
    },
    Contract {
        name: "ask",
        input: "file",
        output: "questions",
        effect: Effect::Read,
    },
    Contract {
        name: "argue",
        input: "file",
        output: "report",
        effect: Effect::Read,
    },
    Contract {
        name: "chain",
        input: "file",
        output: "link",
        effect: Effect::Read,
    },
    Contract {
        name: "chains",
        input: "file",
        output: "report",
        effect: Effect::Read,
    },
    Contract {
        name: "views",
        input: "file",
        output: "five views",
        effect: Effect::Read,
    },
    Contract {
        name: "diff",
        input: "files",
        output: "diff",
        effect: Effect::Read,
    },
    Contract {
        name: "merge",
        input: "files",
        output: "file",
        effect: Effect::Write,
    },
    Contract {
        name: "blame",
        input: "store",
        output: "blame",
        effect: Effect::Read,
    },
    Contract {
        name: "save",
        input: "file",
        output: "store",
        effect: Effect::Write,
    },
    Contract {
        name: "log",
        input: "store",
        output: "hashes",
        effect: Effect::Read,
    },
    Contract {
        name: "restore",
        input: "store",
        output: "file",
        effect: Effect::ReadWrite,
    },
    Contract {
        name: "grow",
        input: "library+source",
        output: "file",
        effect: Effect::ReadWrite,
    },
    Contract {
        name: "trace",
        input: "file",
        output: "run",
        effect: Effect::ReadWrite,
    },
    Contract {
        name: "replay",
        input: "trace",
        output: "report",
        effect: Effect::Read,
    },
    Contract {
        name: "build",
        input: "file",
        output: "dir",
        effect: Effect::Write,
    },
    Contract {
        name: "provenance",
        input: "file",
        output: "rows",
        effect: Effect::Read,
    },
    Contract {
        name: "fix",
        input: "file",
        output: "candidates",
        effect: Effect::Read,
    },
    Contract {
        name: "apply",
        input: "file",
        output: "file",
        effect: Effect::ReadWrite,
    },
    Contract {
        name: "propose",
        input: "file",
        output: "file",
        effect: Effect::ReadWrite,
    },
    Contract {
        name: "fmt",
        input: "file",
        output: "text",
        effect: Effect::Read,
    },
    Contract {
        name: "contracts",
        input: "nothing",
        output: "table",
        effect: Effect::Pure,
    },
];

/// The contracts as a report.
pub struct ContractsReport;

impl ContractsReport {
    pub fn text(&self) -> String {
        let width = CONTRACTS
            .iter()
            .map(|contract| contract.name.chars().count())
            .max()
            .unwrap_or(10);
        let mut out = String::new();
        for contract in CONTRACTS {
            out.push_str(&format!(
                "{:<width$}  {:<24} -> {:<12} {}",
                contract.name,
                contract.input,
                contract.output,
                contract.effect.label(),
                width = width
            ));
            out.push('\n');
        }
        out
    }

    pub fn json(&self) -> J {
        let items: Vec<J> = CONTRACTS
            .iter()
            .map(|contract| {
                J::Obj(vec![
                    ("name".to_string(), J::Str(contract.name.to_string())),
                    ("input".to_string(), J::Str(contract.input.to_string())),
                    ("output".to_string(), J::Str(contract.output.to_string())),
                    (
                        "effect".to_string(),
                        J::Str(contract.effect.label().to_string()),
                    ),
                ])
            })
            .collect();
        J::Obj(vec![("contracts".to_string(), J::Arr(items))])
    }
}

impl crate::out::Report for ContractsReport {
    fn text(&self) -> String {
        ContractsReport::text(self)
    }

    fn json(&self) -> J {
        ContractsReport::json(self)
    }
}
