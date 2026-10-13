"""Pin structural inputs before changing syntax delivery; no performance selection."""

import json
from pathlib import Path
import subprocess
import tempfile

from patch_reference import decode
from trace_reference import digest

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "evidence/delivery-v1/cases"


def patch(name, files):
    with tempfile.TemporaryDirectory() as directory:
        work = Path(directory)
        for side in ["old", "new"]:
            (work / side).mkdir()
        for name_in_patch, old, new in files:
            for side, text in [("old", old), ("new", new)]:
                (work / side / name_in_patch).write_text(text)
        result = subprocess.run(
            ["git", "-c", "core.quotePath=true", "diff", "--no-index", "--no-ext-diff",
             "--no-textconv", "--no-color", "--unified=100000", "old", "new"],
            cwd=work, capture_output=True, check=False)
        if result.returncode != 1:
            raise RuntimeError(result.stderr.decode())
        with (OUT / f"{name}.diff").open("xb") as output:
            output.write(result.stdout)


def main():
    patch("many-small", [(f"file-{i:03}.rs", f"fn item_{i}() {{ old(); }}\n",
                          f"fn item_{i}() {{ new(); }}\n") for i in range(80)])
    old = "/* multiline\n" + "".join(f"comment {i} 日本語\n" for i in range(2000))
    old += "*/\nexport const value = 1;\n"
    patch("continuous", [("continuous.ts", old, old.replace("comment 1000", "changed 1000")
                         .replace("value = 1", "value = 2"))])
    patch("guard", [("guard.rs", "fn old() {}\n", "\n".join(
        ["x" * 9999, "x" * 10000, "x" * 10001, "fn tail() {}", ""]))])
    with (OUT / "position.diff").open("xb") as output:
        output.write((ROOT / "fixtures/position/review.patch").read_bytes())
    cases = []
    for name in ["position", "many-small", "continuous", "guard", "ratatui"]:
        path = OUT / f"{name}.diff"
        reference = decode(path)
        ref_path = OUT / f"{name}.reference.json"
        with ref_path.open("x") as output:
            json.dump(reference, output, separators=(",", ":"))
        cases.append({"id": name, "path": str(path.relative_to(ROOT)),
                      "sha256": digest(path), "reference": str(ref_path.relative_to(ROOT)),
                      "reference_sha256": digest(ref_path),
                      "dimensions": {k: v for k, v in reference.items() if k != "files"}})
    with (OUT / "manifest.json").open("x") as output:
        json.dump({"selection": "Pinned before event-loop candidate changes. No trial-based replacement.",
                   "upstream": {"repository": "https://github.com/ratatui/ratatui",
                                "base": "1c3dbd1ff4fad2696543bf332a7e413d1bb69988",
                                "head": "e665c36cb14752a61cd777fbd06dbef8474f2add"},
                   "generator_sha256": digest(Path(__file__)), "cases": cases}, output, indent=2)


if __name__ == "__main__":
    main()
