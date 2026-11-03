use super::*;

pub(crate) fn cmd_engine(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let result = crate::closure::engine(&loaded.decls);
    if json {
        let quoted = |items: &[String]| {
            items
                .iter()
                .map(|item| format!("\"{}\"", escape(item)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!("{{");
        println!("  \"held\": [{}],", quoted(&result.held));
        let conditional: Vec<String> = result
            .conditional
            .iter()
            .map(|(name, _)| format!("\"{}\"", escape(name)))
            .collect();
        println!("  \"conditional\": [{}],", conditional.join(", "));
        let blocked: Vec<String> = result
            .blocked
            .iter()
            .map(|(warrant, missing)| {
                format!(
                    "{{\"warrant\": \"{}\", \"missing\": [{}]}}",
                    escape(warrant),
                    quoted(missing)
                )
            })
            .collect();
        println!("  \"blocked\": [{}],", blocked.join(", "));
        let chains: Vec<String> = result
            .chains
            .iter()
            .map(|(name, chain)| format!("\"{}\": \"{}\"", escape(name), escape(chain)))
            .collect();
        println!("  \"chains\": {{{}}},", chains.join(", "));
        let gates: Vec<String> = result
            .gates
            .iter()
            .map(|(name, weak)| format!("\"{}\": [{}]", escape(name), quoted(weak)))
            .collect();
        println!("  \"gates\": {{{}}}", gates.join(", "));
        println!("}}");
        return 0;
    }
    println!(
        "held {}  conditional {}  blocked {}",
        result.held.len(),
        result.conditional.len(),
        result.blocked.len()
    );
    for (name, chain) in &result.chains {
        let gates = result
            .gates
            .iter()
            .find(|(gate, _)| gate == name)
            .map(|(_, weak)| weak.clone())
            .unwrap_or_default();
        let suffix = if gates.is_empty() {
            String::new()
        } else {
            let shown: Vec<String> = gates.iter().take(3).cloned().collect();
            let hidden = gates.len() - shown.len();
            let tail = if hidden > 0 {
                format!("; … {hidden} more")
            } else {
                String::new()
            };
            format!("  gates: {}{tail}", shown.join(", "))
        };
        println!("  held {chain}{suffix}");
    }
    for (name, reasons) in &result.conditional {
        let shown: Vec<String> = reasons.iter().take(2).cloned().collect();
        let hidden = reasons.len() - shown.len();
        let tail = if hidden > 0 {
            format!("; … {hidden} more")
        } else {
            String::new()
        };
        println!("  conditional {name}: {}{tail}", shown.join("; "));
    }
    for (warrant, missing) in &result.blocked {
        println!("  blocked {warrant} needs {}", missing.join(", "));
    }
    0
}
