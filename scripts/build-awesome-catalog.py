#!/usr/bin/env python3
"""Build native/registry/catalog.json from the awesome-coffeescript list."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "native/registry/awesome-coffeescript.md"
OUT = ROOT / "native/registry/catalog.json"

# Packages we vendor as native-compatible .es ports
VENDORED = {
    "heap": {"path": "packages/heap", "version": "0.1.0"},
    "easie": {"path": "packages/easie", "version": "0.1.0"},
    "shellwords": {"path": "packages/shellwords", "version": "0.1.0"},
    "linear-partition": {"path": "packages/linear-partition", "version": "0.1.0"},
    "normat": {"path": "packages/normat", "version": "0.1.0"},
    "sentimood": {"path": "packages/sentimood", "version": "0.1.0"},
    "priority-queue": {"path": "packages/priority-queue", "version": "0.1.0"},
    "parse-decimal-number": {"path": "packages/parse-decimal-number", "version": "0.1.0"},
}


def main():
    text = README.read_text(encoding="utf-8")
    pat = re.compile(
        r"^\*\s+\[([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)\]\([^)]+\)\s+-\s+(.*)$",
        re.M,
    )
    items = []
    seen = set()
    name_counts = {}
    for m in pat.finditer(text):
        owner, repo, desc = m.group(1), m.group(2), m.group(3).strip()
        key = f"{owner}/{repo}".lower()
        if key in seen:
            continue
        seen.add(key)
        base = repo.lower().replace("_", "-")
        for suffix in (".js", ".coffee", "-coffeescript", "-coffee"):
            if base.endswith(suffix):
                base = base[: -len(suffix)]
                break
        # Disambiguate duplicate package names by prefixing owner when needed
        name = base
        if name in name_counts:
            name_counts[name] += 1
            name = f"{owner.lower()}-{base}"
        else:
            name_counts[name] = 1
        entry = {
            "name": name,
            "repo": f"{owner}/{repo}",
            "git": f"https://github.com/{owner}/{repo}.git",
            "description": desc[:240],
            "status": "catalog",
        }
        if name in VENDORED:
            entry["status"] = "available"
            entry["version"] = VENDORED[name]["version"]
            entry["path"] = VENDORED[name]["path"]
        items.append(entry)

    # Ensure vendored packages exist in catalog even if naming differs
    by_name = {i["name"]: i for i in items}
    for name, meta in VENDORED.items():
        if name not in by_name:
            items.append(
                {
                    "name": name,
                    "repo": None,
                    "git": None,
                    "description": f"EvelentScript-compatible port ({name})",
                    "status": "available",
                    "version": meta["version"],
                    "path": meta["path"],
                }
            )
        else:
            by_name[name]["status"] = "available"
            by_name[name]["version"] = meta["version"]
            by_name[name]["path"] = meta["path"]

    catalog = {
        "source": "https://github.com/uhub/awesome-coffeescript",
        "note": (
            "Most listed projects are Atom packages, frameworks, or depend on "
            "browser/Node APIs. Only packages with status=available are vendored "
            "as native EvelentScript (.es). Others can be linked via git and may "
            "require porting."
        ),
        "packages": sorted(items, key=lambda x: x["name"]),
    }
    OUT.write_text(json.dumps(catalog, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    available = sum(1 for i in items if i.get("status") == "available")
    print(f"Wrote {OUT} ({len(items)} packages, {available} available)")


if __name__ == "__main__":
    main()
