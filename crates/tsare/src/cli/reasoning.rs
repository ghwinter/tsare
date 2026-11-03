//! Commands that reason over the whole graph: what holds, what is attacked, what to ask next.

use super::*;

/// The `--json` envelope these commands share, so that three of them do not each invent one.
use crate::json::Json as J;

fn obj(fields: Vec<(&str, J)>) -> J {
    J::Obj(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

fn strings(items: &[String]) -> J {
    J::Arr(items.iter().map(|item| J::Str(item.clone())).collect())
}

fn emit(value: J) {
    println!("{}", crate::json::serialize(&value));
}

fn optional(text: Option<String>) -> J {
    match text {
        Some(text) => J::Str(text),
        None => J::Null,
    }
}

fn merge_text(merged: &crate::merge::Merged) -> String {
    let mut text = String::from("tsare 2\n");
    for view in &merged.views {
        text.push('\n');
        text.push_str(&crate::format::declaration(&decl_of(view)));
    }
    text
}

fn decl_of(view: &crate::merge::View) -> crate::syntax::Decl {
    crate::syntax::Decl {
        kind: view.kind.clone(),
        name: view.name.clone(),
        fields: view.fields.clone(),
        field_span: vec![(0, 0, 0, 0); view.fields.len()],
        kind_line: 1,
        kind_col: 1,
        line: 1,
        col: 1,
        end_line: 1,
        end_col: 1,
        file: String::new(),
        ref_pos: Vec::new(),
    }
}

fn link_json(link: &crate::chain::Link) -> J {
    let children: Vec<J> = link.children.iter().map(link_json).collect();
    obj(vec![
        ("name", J::Str(link.name.clone())),
        ("kind", J::Str(link.kind.clone())),
        ("detail", J::Str(link.detail.clone())),
        ("children", J::Arr(children)),
    ])
}

fn print_link(link: &crate::chain::Link, depth: usize) {
    let pad = "  ".repeat(depth);
    if link.detail.is_empty() {
        println!("{pad}{} {}", link.kind, link.name);
    } else {
        println!("{pad}{} {} — {}", link.kind, link.name, link.detail);
    }
    for child in &link.children {
        print_link(child, depth + 1);
    }
}

mod argument;
mod catalog;
mod decide;
mod derive;
mod engine;
mod structure;

pub(crate) use argument::{
    cmd_argue, cmd_assumptions, cmd_confront, cmd_debt, cmd_impact, cmd_retract, cmd_weigh,
    cmd_whatif,
};
pub(crate) use catalog::{cmd_cat, cmd_fca, cmd_kinds, cmd_moves};
pub(crate) use decide::{cmd_cost, cmd_decide, cmd_epistemic};
pub(crate) use derive::{
    cmd_abstract, cmd_ask, cmd_derive, cmd_hint, cmd_imply, cmd_meet, cmd_next, cmd_novel,
    cmd_shape,
};
pub(crate) use engine::cmd_engine;
pub(crate) use structure::{
    cmd_arrows, cmd_blame, cmd_chain, cmd_chains, cmd_delta, cmd_diff, cmd_merge,
};
