import ast
import pathlib

for path in sorted(pathlib.Path(".").glob("*.py")):
    tree = ast.parse(path.read_text(encoding="utf-8"))
    imports = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            imports.update(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            imports.add(node.module)
    local = {name for name in imports if pathlib.Path(name + ".py").exists()}
    print(f"{path.name:16} {sorted(local)}")
