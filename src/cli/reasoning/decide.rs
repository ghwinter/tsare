use super::*;

pub(crate) fn cmd_decide(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let verdict = crate::decide::report(&loaded.decls);
    if json {
        let items: Vec<J> = verdict
            .items
            .iter()
            .map(|item| {
                obj(vec![
                    ("name", J::Str(item.name.clone())),
                    ("kind", J::Str(item.kind.clone())),
                    ("decision", J::Str(item.decision.label().to_string())),
                    ("reason", J::Str(item.reason.to_string())),
                ])
            })
            .collect();
        emit(obj(vec![
            ("decision", J::Str(verdict.decision.label().to_string())),
            ("items", J::Arr(items)),
        ]));
        return 0;
    }
    print!("{}", crate::decide::text(&verdict));
    0
}

/// Every conclusion and how it is known: verified, inferred, or unverified.
pub(crate) fn cmd_epistemic(path: &Path, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let rows = crate::epistemic::report(&loaded.decls);
    let verified = rows
        .iter()
        .filter(|row| row.knowing == crate::epistemic::Knowing::Verified)
        .count();
    let inferred = rows
        .iter()
        .filter(|row| row.knowing == crate::epistemic::Knowing::Inferred)
        .count();
    let unverified = rows.len() - verified - inferred;
    if json {
        let items: Vec<J> = rows
            .iter()
            .map(|row| {
                obj(vec![
                    ("name", J::Str(row.name.clone())),
                    ("kind", J::Str(row.kind.clone())),
                    ("knowing", J::Str(row.knowing.label().to_string())),
                    (
                        "basis",
                        match &row.basis {
                            Some(basis) => J::Str(basis.clone()),
                            None => J::Null,
                        },
                    ),
                ])
            })
            .collect();
        emit(obj(vec![
            ("verified", J::Num(verified as f64)),
            ("inferred", J::Num(inferred as f64)),
            ("unverified", J::Num(unverified as f64)),
            ("rows", J::Arr(items)),
        ]));
        return 0;
    }
    print!("{}", crate::epistemic::text(&rows));
    0
}

pub(crate) fn cmd_cost(path: &Path, top: usize, json: bool) -> i32 {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let cost = crate::cost::report(&program);
    if json {
        let by_kind: Vec<J> = cost
            .by_kind
            .iter()
            .map(|(kind, count)| {
                obj(vec![
                    ("kind", J::Str(kind.clone())),
                    ("count", J::Num(*count as f64)),
                ])
            })
            .collect();
        let rows: Vec<J> = cost
            .rows
            .iter()
            .map(|(name, found)| {
                obj(vec![
                    ("name", J::Str(name.clone())),
                    ("needs", strings(found)),
                ])
            })
            .collect();
        emit(obj(vec![
            ("total", J::Num(cost.total as f64)),
            ("depth", J::Num(cost.depth as f64)),
            ("chain", strings(&cost.chain)),
            ("by_kind", J::Arr(by_kind)),
            ("rows", J::Arr(rows)),
        ]));
        return 0;
    }
    println!(
        "decompression cost: {} external confirmation(s), depth {}",
        cost.total, cost.depth
    );
    if !cost.by_kind.is_empty() {
        let joined: Vec<String> = cost
            .by_kind
            .iter()
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect();
        println!("by kind: {}", joined.join(", "));
    }
    for (name, found) in cost.rows.iter().take(top) {
        println!("  {} {}  {}", pad(name, 18), found.len(), found.join("; "));
    }
    if cost.rows.len() > top {
        println!("  … {} more; raise --top", cost.rows.len() - top);
    }
    if cost.depth > 0 {
        println!("chain: {}", cost.chain.join(" -> "));
    }
    0
}
