import check

lines = [
    "//! Generated from the Python core SPECS table.  Regenerate rather than edit.",
    "",
    "pub type Field = (&'static str, &'static str, bool);",
    "",
    "pub fn spec(kind: &str) -> Option<&'static [Field]> {",
    "    Some(match kind {",
]
for kind, spec in check.SPECS.items():
    lines.append(f'        "{kind}" => &[')
    for field, (ftype, required) in spec.items():
        lines.append(f'            ("{field}", "{ftype}", {str(required).lower()}),')
    lines.append("        ],")
lines += ["        _ => return None,", "    })", "}", ""]
open(r"D:\GitHub\engine\rust\engine-core\src\specs.rs", "w", encoding="utf-8").write("\n".join(lines))
print("wrote specs.rs for", len(check.SPECS), "kinds")
