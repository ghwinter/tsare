from pathlib import Path

p = Path(r"D:\GitHub\engine\rust\engine-core\src\main.rs")
t = p.read_text(encoding="utf-8")
if "const SKIP: " not in t:
    t = t.replace(
        "];",
        '];\n\nconst SKIP: &[&str] = &["tool", "doc", "code", "user", "model", "observation"];',
        1,
    )
t = t.replace(
    "                if by_name.contains_key(&name) {",
    '                if SKIP.contains(&name.as_str()) || by_name.contains_key(&name) {',
)
p.write_text(t, encoding="utf-8")
print("patched skip", "SKIP" in t)
