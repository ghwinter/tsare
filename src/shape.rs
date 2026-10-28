//! The form of a declaration, apart from its content.
//!
//! Two declarations have the same form when their kinds agree and every field has the same shape --
//! the same literal kind, and for a reference the same kind of thing at the other end.  Content does
//! not enter the signature: a `claim`'s text and a fact's `attrs` are deliberately invisible.  That
//! is what makes the grouping a quotient rather than a judgement about meaning.
//!
//! The quotient is coarse on purpose.  A group is an equivalence class of declarations under
//! "same form"; the fields whose values differ across the class are that class's variables, and the
//! table of assignments is the thing a reader can act on -- `if this substitution holds, the rule
//! still holds`.  tsare does not decide that; it only writes the equality down.

use crate::program::Program;
use crate::syntax::{Decl, Val};

/// One equivalence class of declarations, with the positions that vary.
pub struct ShapeGroup {
    pub kind: String,
    pub signature: String,
    pub members: Vec<String>,
    /// field -> the distinct values it takes across the class, in declaration order.
    pub variables: Vec<(String, Vec<String>)>,
}

/// The shape of a value: its literal kind, and for a reference the kind at the other end.
fn shape_of(program: &Program, value: &Val) -> String {
    match value {
        Val::Str(_) => "text".to_string(),
        Val::Num(_) => "number".to_string(),
        Val::Id(name) => match program.get(name) {
            Some(decl) => format!("ref:{}", decl.kind),
            None => "ref:?".to_string(),
        },
        Val::Call(name, _) => format!("call:{name}"),
        Val::List(items) => {
            let parts: Vec<String> = items.iter().map(|item| shape_of(program, item)).collect();
            format!("list[{}]", parts.join("+"))
        }
        Val::Rec(fields) => {
            let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
            format!("rec{{{}}}", keys.join(","))
        }
        Val::Hole => "hole".to_string(),
    }
}

/// A value rendered for the assignment table: short, and never the whole text of a claim.
fn render(value: &Val) -> String {
    match value {
        Val::Str(text) => {
            let cut: String = text.chars().take(24).collect();
            format!("\"{cut}…\"")
        }
        Val::Num(raw) => raw.clone(),
        Val::Id(name) => name.clone(),
        Val::Call(name, _) => format!("{name}(…)"),
        Val::List(items) => {
            let parts: Vec<String> = items.iter().map(render).collect();
            format!("[{}]", parts.join(", "))
        }
        Val::Rec(fields) => {
            let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
            format!("{{{}}}", keys.join(", "))
        }
        Val::Hole => "?".to_string(),
    }
}

fn signature(program: &Program, decl: &Decl) -> String {
    let mut fields: Vec<String> = decl
        .fields
        .iter()
        .map(|(key, value)| format!("{key}:{}", shape_of(program, value)))
        .collect();
    fields.sort();
    format!("{}|{}", decl.kind, fields.join(";"))
}

/// Group the declarations by form.  Only classes with at least two members are returned: a form
/// with one instance is that declaration, not a structure.
pub fn shapes(program: &Program) -> Vec<ShapeGroup> {
    let mut groups: Vec<ShapeGroup> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for decl in program.decls {
        let signature = signature(program, decl);
        match index.get(&signature) {
            Some(position) => groups[*position].members.push(decl.name.clone()),
            None => {
                index.insert(signature.clone(), groups.len());
                groups.push(ShapeGroup {
                    kind: decl.kind.clone(),
                    signature,
                    members: vec![decl.name.clone()],
                    variables: Vec::new(),
                });
            }
        }
    }
    groups.retain(|group| group.members.len() >= 2);
    for group in &mut groups {
        group.variables = variables(program, group);
    }
    groups.sort_by(|left, right| {
        right
            .members
            .len()
            .cmp(&left.members.len())
            .then(left.kind.cmp(&right.kind))
            .then(left.members.cmp(&right.members))
    });
    groups
}

/// The positions that vary inside a class, and the values they take.
fn variables(program: &Program, group: &ShapeGroup) -> Vec<(String, Vec<String>)> {
    let members: Vec<&Decl> = group
        .members
        .iter()
        .filter_map(|name| program.get(name))
        .collect();
    let Some(first) = members.first() else {
        return Vec::new();
    };
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (key, _) in &first.fields {
        let mut seen: Vec<String> = Vec::new();
        for member in &members {
            let Some((_, value)) = member.fields.iter().find(|(name, _)| name == key) else {
                seen.push("—".to_string());
                continue;
            };
            let shown = render(value);
            if !seen.contains(&shown) {
                seen.push(shown);
            }
        }
        if seen.len() > 1 {
            out.push((key.clone(), seen));
        }
    }
    out
}

pub fn text(groups: &[ShapeGroup], total: usize, top: usize) -> String {
    if groups.is_empty() {
        return format!(
            "no form is shared: {total} declaration(s), {} distinct shape(s)\n",
            total
        );
    }
    let mut out = format!(
        "{} shared form(s) over {total} declaration(s)\n\n",
        groups.len()
    );
    for (index, group) in groups.iter().take(top).enumerate() {
        out.push_str(&format!(
            "{}. {} x{}  {}\n",
            index + 1,
            group.kind,
            group.members.len(),
            group.members.join(", ")
        ));
        for (field, values) in &group.variables {
            let shown: Vec<&str> = values.iter().take(6).map(String::as_str).collect();
            let more = if values.len() > shown.len() {
                ", …"
            } else {
                ""
            };
            out.push_str(&format!("   {field}: {}{more}\n", shown.join(" | ")));
        }
    }
    out.push_str(
        "
The variables are the positions above.  A substitution that leaves the rule standing is a
claim about the whole class, not about one member.
",
    );
    if groups.len() > top {
        out.push_str(&format!(
            "… {} more class(es); raise --top to see them\n",
            groups.len() - top
        ));
    }
    out
}
