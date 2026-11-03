/// The arguments after the command, split into positional values and options.
pub(crate) struct Args {
    pub(crate) positional: Vec<String>,
    pub(crate) flags: Vec<String>,
    pub(crate) options: std::collections::HashMap<String, String>,
}

const VALUE_FLAGS: &[&str] = &[
    "--top",
    "--depth",
    "--threshold",
    "--out",
    "--append",
    "--trace",
    "--budget",
    "--process",
    "--as",
    "--goals",
    "--k",
    "--proposals",
    "--policy",
    "--attribute",
    "--holds",
    "--against",
    "--max-antecedent",
    "--min-support",
    "--max-intent",
    "--record",
    "--doc",
    "--action",
    "--store",
    "--near",
];

/// Options that take no value.  An option in neither list is a usage error: a silently ignored
/// flag is a flag the caller will believe took effect.
const FLAGS: &[&str] = &[
    "--json",
    "--strict",
    "--sugar",
    "--cache",
    "--all",
    "--one",
    "--layers",
    "--vars",
    "--dry-run",
    "--diff",
    "--check",
    "--recall",
    "--help",
    "--version",
];

impl Args {
    pub(crate) fn parse(raw: &[String]) -> Result<Self, String> {
        let mut positional = Vec::new();
        let mut flags = Vec::new();
        let mut options = std::collections::HashMap::new();
        let mut index = 0;
        while index < raw.len() {
            let item = &raw[index];
            if item.starts_with("--") {
                if let Some((name, value)) = item.split_once('=') {
                    if !VALUE_FLAGS.contains(&name) && !FLAGS.contains(&name) {
                        return Err(format!("unknown option {name}; see --help"));
                    }
                    options.insert(name.to_string(), value.to_string());
                    index += 1;
                    continue;
                }
                // `--apply` is two options with one spelling: `fix --apply OUT` names the file to
                // write, `propose --apply` is a switch.  A following option word means the switch.
                if item == "--apply" {
                    let next = raw.get(index + 1);
                    if next.is_some_and(|value| !value.starts_with("--")) {
                        options.insert(item.clone(), next.unwrap().clone());
                        index += 2;
                    } else {
                        flags.push(item.clone());
                        index += 1;
                    }
                    continue;
                }
                if VALUE_FLAGS.contains(&item.as_str()) {
                    let Some(value) = raw.get(index + 1) else {
                        return Err(format!("{item} needs a value; see --help"));
                    };
                    options.insert(item.clone(), value.clone());
                    index += 2;
                    continue;
                }
                if FLAGS.contains(&item.as_str()) {
                    flags.push(item.clone());
                    index += 1;
                    continue;
                }
                return Err(format!("unknown option {item}; see --help"));
            }
            positional.push(item.clone());
            index += 1;
        }
        Ok(Args {
            positional,
            flags,
            options,
        })
    }

    pub(crate) fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|item| item == flag)
    }

    pub(crate) fn option(&self, flag: &str) -> Option<&str> {
        self.options.get(flag).map(|value| value.as_str())
    }

    pub(crate) fn number(&self, flag: &str, fallback: f64) -> f64 {
        self.option(flag)
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    }

    pub(crate) fn count(&self, flag: &str, fallback: usize) -> usize {
        self.option(flag)
            .and_then(|value| value.parse().ok())
            .unwrap_or(fallback)
    }

    pub(crate) fn at(&self, index: usize) -> Option<&str> {
        self.positional.get(index).map(|item| item.as_str())
    }
}
