use super::*;

pub(crate) fn cmd_derive(path: &Path) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let analysis = crate::derive::analyze(&program);
    println!(
        "grounded: {} proposition(s) from {} facts",
        analysis.grounded.len(),
        analysis.facts.len()
    );
    for name in &analysis.grounded {
        println!("  derived {name}");
    }
    if analysis.conditional.is_empty() {
        println!("every supported conclusion is grounded in facts");
    } else {
        println!("supported but not grounded: {}", analysis.conditional.len());
        for name in &analysis.conditional {
            println!("  {name}");
            let reasons = analysis.reasons.get(name).cloned().unwrap_or_default();
            for reason in reasons.iter().take(3) {
                println!("    {reason}");
            }
            if reasons.len() > 3 {
                println!("    … {} more", reasons.len() - 3);
            }
        }
    }
    Outcome::Ok
}

pub(crate) fn cmd_next(path: &Path, json: bool) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let rows = crate::impact::decisions(&program);
    if json {
        let decisions: Vec<J> = rows
            .iter()
            .map(|row| {
                obj(vec![
                    ("label", J::Str(row.label.clone())),
                    ("node", J::Str(row.node.clone())),
                    ("touches", J::Num(row.count as f64)),
                ])
            })
            .collect();
        emit(obj(vec![("decisions", J::Arr(decisions))]));
        return Outcome::Ok;
    }
    if rows.is_empty() {
        println!("no open decisions");
        return Outcome::Ok;
    }
    let width = rows
        .iter()
        .map(|row| row.label.chars().count())
        .max()
        .unwrap_or(0);
    for row in &rows {
        println!("{}  touches {} claim(s)", pad(&row.label, width), row.count);
    }
    Outcome::Ok
}

pub(crate) fn cmd_ask(path: &Path, args: &Args) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let asked = crate::ask::questions(&program);
    let one = args.has("--one");
    if args.has("--json") {
        let asked = if one {
            &asked[..asked.len().min(1)]
        } else {
            &asked[..]
        };
        let items: Vec<crate::json::Json> = asked
            .iter()
            .map(|item| {
                let mut fields = vec![
                    ("kind", crate::json::Json::Str(item.kind.to_string())),
                    ("subject", crate::json::Json::Str(item.subject.clone())),
                    ("decides", crate::json::Json::Num(item.decides as f64)),
                    ("question", crate::json::Json::Str(item.question.clone())),
                ];
                if let Some(fact) = &item.fact {
                    fields.push(("fact", crate::json::Json::Str(fact.clone())));
                }
                if let Some(key) = &item.key {
                    fields.push(("key", crate::json::Json::Str(key.clone())));
                }
                if let Some(value) = &item.value {
                    fields.push(("value", crate::json::Json::Str(value.clone())));
                }
                crate::json::Json::Obj(
                    fields
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v))
                        .collect(),
                )
            })
            .collect();
        println!(
            "{}",
            crate::json::serialize(&crate::json::Json::Obj(vec![(
                "questions".to_string(),
                crate::json::Json::Arr(items)
            )]))
        );
        return Outcome::Ok;
    }
    print!("{}", crate::ask::text(&program, one));
    Outcome::Ok
}

pub(crate) fn cmd_novel(path: &Path, args: &Args) -> Outcome {
    let Some(attribute) = args.option("--attribute") else {
        println!("novel needs --attribute NAME; add --holds F_A,F_B for the facts it is true of");
        return Outcome::Usage;
    };
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let holds: Vec<String> = args
        .option("--holds")
        .map(|list| {
            list.split(',')
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect()
        })
        .unwrap_or_default();
    for id in &holds {
        if !program.has(id) {
            println!("no such declaration: {id}");
            return Outcome::Failed;
        }
    }
    let verdict = crate::novel::decide(&program, &holds);
    print!("{}", crate::novel::text(&verdict, attribute));
    Outcome::Ok
}

pub(crate) fn cmd_abstract(path: &Path, args: &Args) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let support = args.count("--min-support", 2);
    let max_intent = args.count("--max-intent", 3);
    let (found, truncated) = crate::abstraction::candidates(&program, support, max_intent);
    print!("{}", crate::abstraction::text(&found, truncated));
    Outcome::Ok
}

/// The forms a document's declarations share, and the positions that vary inside each form.
/// A form with one instance is not a structure, so only shared ones are printed.
/// The meet of two declarations: the most general form both satisfy, and the draft that fills the
/// remaining positions.  Content is invisible here -- this generalises over shapes.
pub(crate) fn cmd_meet(path: &Path, args: &Args) -> Outcome {
    let (Some(left), Some(right)) = (args.at(1), args.at(2)) else {
        return fail("usage: meet PATH LEFT RIGHT".to_string());
    };
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let (Some(first), Some(second)) = (
        crate::generalize::pattern_of(&program, left),
        crate::generalize::pattern_of(&program, right),
    ) else {
        return fail(format!(
            "meet: {left} and {right} must both be declared in the document"
        ));
    };
    match crate::generalize::meet(&first, &second) {
        None => {
            println!(
                "{left} and {right} have no meet: {} and {}",
                first.kind, second.kind
            );
            Outcome::Failed
        }
        Some(joined) => {
            println!("{}", crate::generalize::text(&joined));
            println!();
            print!("{}", crate::generalize::draft(&joined, "NAME"));
            Outcome::Ok
        }
    }
}
pub(crate) fn cmd_shape(
    path: &Path,
    top: usize,
    depth: usize,
    layers: bool,
    against: Option<&str>,
    near: usize,
    json: bool,
) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    if near > 0 && against.is_none() {
        println!("--near needs --against: there is nothing to be near without a second document");
        return Outcome::Usage;
    }
    // `--layers` reads one document's forms and `--near` reads two documents' forms: an answer to
    // both questions is two answers, and answering only one of them silently is not one of them.
    if near > 0 && layers {
        println!("--near and --layers read different things; ask for one at a time");
        return Outcome::Usage;
    }
    if let Some(other) = against {
        let other = match load(Path::new(other)) {
            Ok(loaded) => loaded,
            Err(error) => return fail(error),
        };
        let there = Program::new(&other.decls);
        let difference = crate::shape::compare(&program, &there, depth);
        let misses = crate::shape::near_misses(&program, &there, depth, near);
        if json {
            let mut fields = vec![
                ("depth", J::Num(depth as f64)),
                ("shared", strings(&difference.shared)),
                ("only_here", strings(&difference.only_here)),
                ("only_there", strings(&difference.only_there)),
            ];
            if near > 0 {
                let rows: Vec<J> = misses
                    .iter()
                    .map(|miss| {
                        obj(vec![
                            ("here", J::Str(miss.here.clone())),
                            ("there", J::Str(miss.there.clone())),
                            ("score", J::Num((miss.score * 100.0).round() / 100.0)),
                        ])
                    })
                    .collect();
                fields.push(("near", J::Arr(rows)));
            }
            emit(obj(fields));
            return Outcome::Ok;
        }
        print!("{}", crate::shape::difference_text(&difference, depth));
        if near > 0 {
            print!("{}", crate::shape::near_text(&misses));
        }
        return Outcome::Ok;
    }
    if layers {
        let fibres = crate::shape::fibres(&program, depth);
        if json {
            let items: Vec<J> = fibres
                .iter()
                .map(|fibre| {
                    let members: Vec<J> = fibre
                        .members
                        .iter()
                        .map(|(name, layer)| {
                            obj(vec![
                                ("name", J::Str(name.clone())),
                                ("layer", J::Str(layer.clone())),
                            ])
                        })
                        .collect();
                    obj(vec![
                        ("kind", J::Str(fibre.kind.clone())),
                        ("layers", strings(&fibre.layers)),
                        ("members", J::Arr(members)),
                    ])
                })
                .collect();
            emit(obj(vec![
                ("depth", J::Num(depth as f64)),
                ("fibres", J::Arr(items)),
            ]));
            return Outcome::Ok;
        }
        print!("{}", crate::shape::fibre_text(&fibres, depth));
        return Outcome::Ok;
    }
    let groups = crate::shape::shapes_at(&program, depth);
    if json {
        let forms: Vec<J> = groups
            .iter()
            .map(|group| {
                let variables: Vec<J> = group
                    .variables
                    .iter()
                    .map(|(field, values)| {
                        obj(vec![
                            ("field", J::Str(field.clone())),
                            ("values", strings(values)),
                        ])
                    })
                    .collect();
                obj(vec![
                    ("kind", J::Str(group.kind.clone())),
                    ("signature", J::Str(group.signature.clone())),
                    ("members", strings(&group.members)),
                    ("variables", J::Arr(variables)),
                ])
            })
            .collect();
        emit(obj(vec![
            ("depth", J::Num(depth as f64)),
            ("total", J::Num(loaded.decls.len() as f64)),
            ("forms", J::Arr(forms)),
        ]));
        return Outcome::Ok;
    }
    print!(
        "{}",
        crate::shape::text(&groups, loaded.decls.len(), top, depth)
    );
    Outcome::Ok
}
pub(crate) fn cmd_imply(path: &Path, args: &Args) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let max = args.count("--max-antecedent", 2);
    let support = args.count("--min-support", 3);
    let (found, search) = crate::imply::implications(&program, max, support);
    if args.has("--json") {
        let items: Vec<crate::json::Json> = found
            .iter()
            .map(|item| {
                let mut fields = vec![
                    (
                        "antecedent".to_string(),
                        crate::json::Json::Arr(
                            item.antecedent
                                .iter()
                                .map(|pair| crate::json::Json::Str(pair.clone()))
                                .collect(),
                        ),
                    ),
                    (
                        "consequent".to_string(),
                        crate::json::Json::Str(item.consequent.clone()),
                    ),
                    (
                        "support".to_string(),
                        crate::json::Json::Num(item.support as f64),
                    ),
                    (
                        "objects".to_string(),
                        crate::json::Json::Num(item.objects as f64),
                    ),
                ];
                if let Some(witness) = &item.converse_witness {
                    fields.push((
                        "converse_fails".to_string(),
                        crate::json::Json::Str(witness.clone()),
                    ));
                }
                crate::json::Json::Obj(fields)
            })
            .collect();
        println!(
            "{}",
            crate::json::serialize(&crate::json::Json::Obj(vec![(
                "implications".to_string(),
                crate::json::Json::Arr(items)
            )]))
        );
        return Outcome::Ok;
    }
    print!("{}", crate::imply::text(&found, &search));
    Outcome::Ok
}

pub(crate) fn cmd_hint(path: &Path, json: bool) -> Outcome {
    let loaded = match load(path) {
        Ok(loaded) => loaded,
        Err(error) => return fail(error),
    };
    let program = Program::new(&loaded.decls);
    let found = crate::hint::hints(&program);
    if json {
        let rows: Vec<J> = found
            .iter()
            .map(|hint| {
                let free: Vec<J> = hint
                    .free
                    .iter()
                    .map(|(field, values)| {
                        obj(vec![
                            ("field", J::Str(field.clone())),
                            ("values", J::Num(*values as f64)),
                        ])
                    })
                    .collect();
                obj(vec![
                    ("kind", J::Str(hint.kind.clone())),
                    ("members", strings(&hint.members)),
                    ("free", J::Arr(free)),
                    ("layers", strings(&hint.layers)),
                    ("unnamed", J::Bool(hint.unnamed)),
                    ("template", J::Str(hint.template.clone())),
                ])
            })
            .collect();
        emit(obj(vec![("hints", J::Arr(rows))]));
        return Outcome::Ok;
    }
    print!("{}", crate::hint::text(&found));
    Outcome::Ok
}
