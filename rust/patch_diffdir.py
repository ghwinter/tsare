from pathlib import Path

p = Path(r"D:\GitHub\engine\core\scripts\diff_rust.py")
t = p.read_text(encoding="utf-8")

addition = r'''

def python_dir_pairs(path: Path):
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(d["code"], d["file"], d["line"]) for d in payload["diagnostics"] if d["code"] in CODES}


def rust_dir_pairs(path: Path):
    result = subprocess.run([str(BIN), str(path)], capture_output=True, text=True)
    pairs = set()
    for line in result.stdout.splitlines():
        parts = line.strip().split(" ", 2)
        if len(parts) < 2 or len(parts[0]) < 4 or not all(ch.isupper() or ch == "-" for ch in parts[0]):
            continue
        match = re.match(r"(.+):(\d+)$", parts[1])
        if match and parts[0] in CODES:
            pairs.add((parts[0], Path(match.group(1)).name, int(match.group(2))))
    return pairs
'''

t = t.replace("def main() -> int:", addition + "\ndef main() -> int:", 1)
t = t.replace(
    '    print(f"documents: {len(documents)}, mismatches: {mismatches}")',
    '    for project in sorted((V1 / "projects").glob("*")):\n'
    '        left, right = python_dir_pairs(project), rust_dir_pairs(project)\n'
    '        if left != right:\n'
    '            mismatches += 1\n'
    '            print(f"MISMATCH project {project.name}: only-python={sorted(left - right)[:3]} only-rust={sorted(right - left)[:3]}")\n'
    '        else:\n'
    '            print(f"project {project.name}: {len(left)} diagnostics agree")\n'
    '    print(f"documents: {len(documents)}, mismatches: {mismatches}")',
)
p.write_text(t, encoding="utf-8")
print("directory comparison added")
