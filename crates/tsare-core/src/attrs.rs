use super::*;

pub(crate) fn string_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| match value {
            Val::Str(text) => Some(text.as_str()),
            _ => None,
        })
}

pub(crate) fn id_field<'a>(decl: &'a Decl, key: &str) -> Option<&'a str> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| match value {
            Val::Id(text) => Some(text.as_str()),
            _ => None,
        })
}

/// Words that make a sentence universal.  This is a surface cue, not the criterion: the criterion
/// is whether the domain is bound with `over:`.  The cue is deliberately generous on `都`, which is
/// also an adverb: a false positive here costs one line (`over:` naming the set, or a reword), and
/// naming the set is the improvement the warning is asking for anyway.
///
/// English has spaces, so an English cue is matched as a word and not as a substring: `all ` used
/// to fire inside `small` and `stall`, while a capitalised `All` did not fire at all.  Chinese has
/// no such boundary; the character cues stay substring tests.
pub(crate) fn universal_text(text: &str) -> bool {
    // "at all" and "after all" are idioms, not universal claims
    let lowered = text
        .to_lowercase()
        .replace("at all", "")
        .replace("after all", "");
    if ["所有", "全部", "每", "都"]
        .iter()
        .any(|word| lowered.contains(word))
    {
        return true;
    }
    lowered
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|token| token == "all" || token == "always" || token.starts_with("every"))
}

pub(crate) fn num_field(decl: &Decl, key: &str) -> Option<f64> {
    decl.fields
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| tsare_lang::program::num_of(value))
}

pub(crate) fn field_pos(decl: &Decl, key: &str) -> (usize, usize) {
    for (index, (name, _)) in decl.fields.iter().enumerate() {
        if name == key {
            return decl
                .field_span
                .get(index)
                .map(|span| (span.0, span.1))
                .unwrap_or((decl.line, decl.col));
        }
    }
    (decl.line, decl.col)
}

pub(crate) fn list_field(decl: &Decl, key: &str) -> Vec<String> {
    field_ids(decl, key)
}

/// The attribute keys a fact records.  Two facts are comparable only over a shared key.
pub(crate) fn attr_keys(decl: &Decl) -> HashSet<String> {
    match decl
        .fields
        .iter()
        .find(|(name, _)| name == "attrs")
        .map(|(_, value)| value)
    {
        Some(Val::Rec(fields)) => fields.iter().map(|(key, _)| key.clone()).collect(),
        _ => HashSet::new(),
    }
}
