from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = p.read_text(encoding="utf-8")
t = t.replace(
    'const NOT_REFS: &[&str] = &["user", "model", "observation", "min", "max", "intuition", "preference", "conjecture"];',
    'const REF_KEYS: &[&str] = &[\n'
    '    "from", "to", "grounded_by", "against", "uses", "assumes", "answers", "separates",\n'
    '    "rival", "witness", "basis", "attacks", "checked_by", "holds_in", "exposes",\n'
    '    "closes_with", "parent", "questions",\n'
    "];",
)
t = t.replace(
    '            if key == "evidence" || key == "measured_by" {\n                continue;\n            }',
    '            if !REF_KEYS.contains(&key.as_str()) {\n                continue;\n            }',
)
t = t.replace(
    '                if NOT_REFS.contains(&name.as_str()) || by_name.contains_key(&name) {',
    "                if by_name.contains_key(&name) {",
)
p.write_text(t, encoding="utf-8")
print("patched", "REF_KEYS" in t)
