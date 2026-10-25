from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\check.rs")
t = p.read_text(encoding="utf-8")

# E-NOTE-REF: a reference that lands on a note
t = t.replace(
    '''                if let Some(target_kind) = by_name.get(name.as_str()) {
                    if let Some(expected) = want.as_deref() {''',
    '''                if let Some(target_kind) = by_name.get(name.as_str()) {
                    if *target_kind == "note" {
                        out.push(diag(
                            "E-NOTE-REF",
                            format!("{}.{key} leans on the note {name}, which is not a commitment", decl.name),
                            decl,
                        ));
                        continue;
                    }
                    if let Some(expected) = want.as_deref() {''',
)

# W-CONFIDENCE-MODEL: a model-supplied fact that claims confidence
t = t.replace(
    '''                if let Some(confidence) = num_field(decl, "confidence") {
                    if !(0.0..=1.0).contains(&confidence) {''',
    '''                if id_field(decl, "evidence") == Some("model") {
                    if let Some(confidence) = num_field(decl, "confidence") {
                        if confidence >= 0.8 {
                            sem(
                                decl,
                                "W-CONFIDENCE-MODEL",
                                format!(
                                    "fact {} is model-supplied but claims confidence {}",
                                    decl.name, confidence
                                ),
                                Some("confidence"),
                                out,
                            );
                        }
                    }
                }
                if let Some(confidence) = num_field(decl, "confidence") {
                    if !(0.0..=1.0).contains(&confidence) {''',
)

p.write_text(t, encoding="utf-8")
print("note-ref and confidence-model added")
