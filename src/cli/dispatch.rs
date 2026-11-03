use super::*;

pub(crate) fn dispatch(path: &Path, command: &str, args: &Args) -> i32 {
    match command {
        "check" => cmd_check(
            path,
            args.has("--json"),
            args.has("--sugar"),
            args.option("--goals"),
            args.has("--cache"),
            args.option("--against"),
            args.has("--strict"),
        ),
        "engine" => cmd_engine(path, args.has("--json")),
        "status" => cmd_status(path, args.has("--json"), args.option("--trace")),
        "provenance" => cmd_provenance(path, args.has("--json")),
        "trace" => cmd_trace(path, args.option("--out"), args.option("--append")),
        "replay" => cmd_replay(path, args.has("--diff")),
        "grow" => {
            let (Some(library), Some(source), Some(name)) = (args.at(0), args.at(1), args.at(2))
            else {
                println!("usage: tsare grow LIBRARY SOURCE NAME --out FILE");
                return 2;
            };
            let Some(out) = args.option("--out") else {
                println!("usage: tsare grow LIBRARY SOURCE NAME --out FILE");
                return 2;
            };
            cmd_grow(Path::new(library), Path::new(source), name, out)
        }
        "save" => cmd_save(path, args.option("--store").unwrap_or(".tsare/versions")),
        "log" => cmd_log(path),
        "restore" => {
            let Some(out) = args.option("--out") else {
                println!("usage: tsare restore HASH [--store DIR] --out FILE");
                return 2;
            };
            cmd_restore(
                args.at(0).unwrap_or(""),
                args.option("--store").unwrap_or(".tsare/versions"),
                out,
            )
        }
        "suggest" => cmd_suggest(path, args.has("--json")),
        "apply" => cmd_apply(path, args.has("--dry-run")),
        "argue" => cmd_argue(
            path,
            args.option("--against").map(Path::new),
            args.has("--json"),
        ),
        "confront" => {
            let Some(other) = args.at(1) else {
                println!("usage: tsare confront LEFT RIGHT");
                return 2;
            };
            cmd_confront(path, Path::new(other))
        }
        "derive" => cmd_derive(path),
        "debt" => cmd_debt(path, args.has("--all")),
        "impact" => cmd_impact(path),
        "next" => cmd_next(path, args.has("--json")),
        "ask" => cmd_ask(path, args),
        "novel" => cmd_novel(path, args),
        "meet" => cmd_meet(path, args),
        "shape" => cmd_shape(
            path,
            args.count("--top", 8),
            args.count("--depth", 0),
            args.has("--layers"),
            args.option("--against"),
            args.count("--near", 0),
            args.has("--json"),
        ),
        "hint" => cmd_hint(path, args.has("--json")),
        "imply" => cmd_imply(path, args),
        "cost" => cmd_cost(path, args.count("--top", 8), args.has("--json")),
        "decide" => cmd_decide(path, args.has("--json")),
        "epistemic" => cmd_epistemic(path, args.has("--json")),
        "weigh" => cmd_weigh(path, args.number("--threshold", 0.7)),
        "cat" => cmd_cat(path),
        "moves" => cmd_moves(path),
        "assumptions" => cmd_assumptions(path, args.count("--top", 12)),
        "audit" => cmd_audit(path),
        "egraph" => cmd_egraph(path, args.option("--write")),
        "whatif" => {
            let Some(name) = args.at(1) else {
                println!("usage: tsare whatif FILE NAME");
                return 2;
            };
            cmd_whatif(path, name)
        }
        "graph" => cmd_graph(path, args.option("--out")),
        "views" => cmd_views(
            path,
            args.has("--json"),
            args.option("--as"),
            args.has("--check"),
        ),
        "fix" => cmd_fix(path, args.option("--apply")),
        "map" => {
            let Some(right) = args.at(1) else {
                println!("usage: tsare map LEFT RIGHT");
                return 2;
            };
            cmd_map(path, Path::new(right), args.count("--top", 8))
        }
        "delta" => {
            let Some(after) = args.at(1) else {
                println!("usage: tsare delta BEFORE.json AFTER.json");
                return 2;
            };
            cmd_delta(
                path,
                Path::new(after),
                args.has("--json"),
                args.option("--record"),
                args.option("--doc"),
                args.option("--action"),
            )
        }
        "diff" => {
            let Some(after) = args.at(1) else {
                println!("usage: tsare diff BEFORE AFTER");
                return 2;
            };
            cmd_diff(path, Path::new(after), args.has("--json"))
        }
        "merge" => {
            let (Some(ours), Some(theirs)) = (args.at(1), args.at(2)) else {
                println!("usage: tsare merge BASE OURS THEIRS --out FILE [--json]");
                return 2;
            };
            cmd_merge(
                path,
                Path::new(ours),
                Path::new(theirs),
                args.option("--out"),
                args.has("--json"),
            )
        }
        "blame" => {
            let Some(name) = args.at(1) else {
                println!("usage: tsare blame STORE NAME [--json]");
                return 2;
            };
            cmd_blame(path, name, args.has("--json"))
        }
        "chain" => {
            let Some(name) = args.at(1) else {
                println!("usage: tsare chain FILE NAME [--json]");
                return 2;
            };
            cmd_chain(path, name, args.has("--json"))
        }
        "chains" => cmd_chains(path, args.has("--json")),
        "fmt" => cmd_fmt(path),
        "induce" => cmd_induce(path),
        "propose" => {
            let Some(proposals) = args.option("--proposals") else {
                println!("usage: tsare propose FILE --proposals FILE.json [--apply]");
                return 2;
            };
            cmd_propose(
                path,
                proposals,
                args.has("--apply"),
                args.option("--policy").unwrap_or("harmless"),
            )
        }
        "abstract" => cmd_abstract(path, args),
        "fca" => cmd_fca(path),
        "pattern" => {
            let Some(name) = args.at(1) else {
                println!("usage: tsare pattern FILE CONCEPT");
                return 2;
            };
            cmd_pattern(path, name, args.has("--vars"))
        }
        "build" => cmd_build(
            path,
            args.option("--out").unwrap_or("out"),
            args.option("--goals"),
        ),
        "project" => cmd_project(path),
        "outline" => cmd_outline(path),
        _ => {
            println!("unknown command {command}");
            2
        }
    }
}
