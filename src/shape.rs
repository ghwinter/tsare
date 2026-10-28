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

/// The shape of a value at a given depth.  At depth 0 a reference is its target's kind; at depth
/// `k+1` it is the target's own signature at depth `k`, so the quotient refines: two declarations
/// that look alike at depth 0 stop being alike when what they point at turns out to differ.
pub(crate) fn shape_of(program: &Program, value: &Val, depth: usize) -> String {
    match value {
        Val::Str(_) => "text".to_string(),
        Val::Num(_) => "number".to_string(),
        Val::Id(name) => match program.get(name) {
            Some(decl) if depth > 0 => format!("ref[{}]", signature_at(program, decl, depth - 1)),
            Some(decl) => format!("ref:{}", decl.kind),
            None => "ref:?".to_string(),
        },
        Val::Call(name, _) => format!("call:{name}"),
        Val::List(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|item| shape_of(program, item, depth))
                .collect();
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

fn signature_at(program: &Program, decl: &Decl, depth: usize) -> String {
    let mut fields: Vec<String> = decl
        .fields
        .iter()
        .map(|(key, value)| format!("{key}:{}", shape_of(program, value, depth)))
        .collect();
    fields.sort();
    format!("{}|{}", decl.kind, fields.join(";"))
}

/// Group the declarations by form.  Only classes with at least two members are returned: a form
/// with one instance is that declaration, not a structure.
pub fn shapes(program: &Program) -> Vec<ShapeGroup> {
    shapes_at(program, 0)
}

/// The quotient at a given depth: depth 0 groups by the declaration's own form, depth 1 also by the
/// form of what it points at, and so on.  The classes only get finer with depth, so the sequence is
/// a filtration of the same set -- `shape --depth 0` is the coarsest reading, not a different one.
pub fn shapes_at(program: &Program, depth: usize) -> Vec<ShapeGroup> {
    let mut groups: Vec<ShapeGroup> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for decl in program.decls {
        let signature = signature_at(program, decl, depth);
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

/// One class seen against the layers its members sit in.  A form that appears at more than one layer
/// is a fibre: the same structure claimed twice at different heights, which is either a refinement
/// or a contradiction, and the document is what decides which.
pub struct Fibre {
    pub kind: String,
    pub layers: Vec<String>,
    pub members: Vec<(String, String)>,
}

/// Group the shared forms by the layers they appear in.  Only forms that span at least two layers
/// are returned: a form inside one layer is that layer's business.
pub fn fibres(program: &Program, depth: usize) -> Vec<Fibre> {
    let mut out: Vec<Fibre> = Vec::new();
    for group in shapes_at(program, depth) {
        let mut layers: Vec<String> = Vec::new();
        let mut members: Vec<(String, String)> = Vec::new();
        for name in &group.members {
            let Some(decl) = program.get(name) else {
                continue;
            };
            let layer = match decl.fields.iter().find(|(key, _)| key == "layer") {
                Some((_, Val::Id(target))) => target.clone(),
                Some((_, other)) => render(other),
                None => continue,
            };
            if !layers.contains(&layer) {
                layers.push(layer.clone());
            }
            members.push((name.clone(), layer));
        }
        if layers.len() < 2 {
            continue;
        }
        layers.sort();
        out.push(Fibre {
            kind: group.kind,
            layers,
            members,
        });
    }
    out.sort_by(|left, right| {
        right
            .layers
            .len()
            .cmp(&left.layers.len())
            .then(left.kind.cmp(&right.kind))
    });
    out
}

pub fn fibre_text(fibres: &[Fibre], depth: usize) -> String {
    if fibres.is_empty() {
        return format!("no form spans two layers at depth {depth}\n");
    }
    let mut out = format!(
        "{} form(s) spanning two or more layers at depth {depth}\n\n",
        fibres.len()
    );
    for (index, fibre) in fibres.iter().enumerate() {
        out.push_str(&format!(
            "{}. {}  layers: {}\n",
            index + 1,
            fibre.kind,
            fibre.layers.join(", ")
        ));
        for (name, layer) in fibre.members.iter().take(10) {
            out.push_str(&format!("   {name} in {layer}\n"));
        }
        if fibre.members.len() > 10 {
            out.push_str(&format!("   … {} more\n", fibre.members.len() - 10));
        }
    }
    out
}

pub fn text(groups: &[ShapeGroup], total: usize, top: usize, depth: usize) -> String {
    if groups.is_empty() {
        return format!(
            "no form is shared at depth {depth}: every one of the {total} declaration(s) has its own\n\
             form.  Lower the depth to compare only what the declarations say about themselves.\n"
        );
    }
    let mut out = format!(
        "{} shared form(s) over {total} declaration(s) at depth {depth}\n\n",
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

/// Two documents read as sets of forms, and the difference between them.  This is how a structure is
/// compared with another structure rather than a claim with a claim: what both frameworks have, what
/// only one of them has.  Content never enters it, so the answer is about shape.
pub struct Difference {
    pub shared: Vec<String>,
    pub only_here: Vec<String>,
    pub only_there: Vec<String>,
}

fn forms(program: &Program, depth: usize) -> std::collections::BTreeMap<String, String> {
    let mut out: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for decl in program.decls {
        let signature = signature_at(program, decl, depth);
        out.entry(signature)
            .or_insert_with(|| format!("{} {}", decl.kind, decl.name));
    }
    out
}

pub fn compare(here: &Program, there: &Program, depth: usize) -> Difference {
    let left = forms(here, depth);
    let right = forms(there, depth);
    let shared = left
        .keys()
        .filter(|key| right.contains_key(*key))
        .map(|key| left[key].clone())
        .collect();
    let only_here = left
        .keys()
        .filter(|key| !right.contains_key(*key))
        .map(|key| left[key].clone())
        .collect();
    let only_there = right
        .keys()
        .filter(|key| !left.contains_key(*key))
        .map(|key| right[key].clone())
        .collect();
    Difference {
        shared,
        only_here,
        only_there,
    }
}

pub fn difference_text(difference: &Difference, depth: usize) -> String {
    let show = |items: &[String]| -> String {
        if items.is_empty() {
            return "none".to_string();
        }
        let mut shown: Vec<&str> = items.iter().take(8).map(String::as_str).collect();
        let hidden = items.len() - shown.len();
        let more = if hidden > 0 {
            format!(", … {hidden} more")
        } else {
            String::new()
        };
        shown.sort();
        format!("{}{more}", shown.join(", "))
    };
    format!(
        "forms at depth {depth}: {} shared, {} only here, {} only there\n  shared:     {}\n  only here:  {}\n  only there: {}\n",
        difference.shared.len(),
        difference.only_here.len(),
        difference.only_there.len(),
        show(&difference.shared),
        show(&difference.only_here),
        show(&difference.only_there)
    )
}
