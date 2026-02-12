import argparse
import json
import os
from pathlib import Path


def normalize_lines(content: str) -> list[str]:
    return [line.strip() for line in content.replace("\r\n", "\n").replace("\r", "\n").split("\n") if line.strip()]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def parse_ruleset_paths(rulesets_file: Path) -> list[str]:
    lines = normalize_lines(read_text(rulesets_file))
    paths: list[str] = []
    for line in lines:
        if line.startswith(";") or line.startswith("#"):
            continue
        parts = line.split(",", 1)
        if len(parts) != 2:
            continue
        path = parts[1].strip()
        if path.startswith("[]"):
            continue
        paths.append(path)
    return paths


def compare_file_pair(left: Path, right: Path) -> dict:
    left_exists = left.exists()
    right_exists = right.exists()
    result = {
        "left": str(left),
        "right": str(right),
        "exists_left": left_exists,
        "exists_right": right_exists,
    }
    if not left_exists or not right_exists:
        result["equal"] = False
        return result

    left_lines = normalize_lines(read_text(left))
    right_lines = normalize_lines(read_text(right))
    left_set = set(left_lines)
    right_set = set(right_lines)

    result.update(
        {
            "equal": left_lines == right_lines,
            "left_line_count": len(left_lines),
            "right_line_count": len(right_lines),
            "extra_in_left": len(left_set - right_set),
            "extra_in_right": len(right_set - left_set),
            "sample_extra_in_left": sorted(list(left_set - right_set))[:8],
            "sample_extra_in_right": sorted(list(right_set - left_set))[:8],
        }
    )
    return result


def main():
    parser = argparse.ArgumentParser(description="Compare repo resource files against release resource files")
    parser.add_argument("--repo-root", default="/srv/work/subconverter-rs/base")
    parser.add_argument("--release-root", default="/tmp/subconverter-release/subconverter")
    parser.add_argument("--out-dir", default="scripts/parity-report/resources")
    args = parser.parse_args()

    repo_root = Path(args.repo_root)
    release_root = Path(args.release_root)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    compare_entries: list[dict] = []

    snippet_files = ["snippets/rulesets.txt", "snippets/groups.txt"]
    for rel_path in snippet_files:
        compare_entries.append(
            {
                "kind": "snippet",
                "path": rel_path,
                **compare_file_pair(repo_root / rel_path, release_root / rel_path),
            }
        )

    ruleset_paths = parse_ruleset_paths(release_root / "snippets/rulesets.txt")
    for rel_path in ruleset_paths:
        compare_entries.append(
            {
                "kind": "ruleset",
                "path": rel_path,
                **compare_file_pair(repo_root / rel_path, release_root / rel_path),
            }
        )

    summary = {
        "total": len(compare_entries),
        "equal": sum(1 for e in compare_entries if e.get("equal") is True),
        "different": sum(1 for e in compare_entries if e.get("equal") is False),
        "missing": sum(
            1 for e in compare_entries if not e.get("exists_left", False) or not e.get("exists_right", False)
        ),
    }

    json_path = out_dir / "resource_diff.json"
    json_path.write_text(
        json.dumps(
            {
                "repo_root": str(repo_root),
                "release_root": str(release_root),
                "summary": summary,
                "entries": compare_entries,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )

    md_path = out_dir / "resource_diff.md"
    with md_path.open("w", encoding="utf-8") as f:
        f.write("# Resource Diff Report\n\n")
        f.write("Repo resources compared with release resources.\n\n")
        f.write(f"- Repo root: `{repo_root}`\n")
        f.write(f"- Release root: `{release_root}`\n")
        f.write(
            f"- Total files: {summary['total']}\n- Equal: {summary['equal']}\n- Different: {summary['different']}\n- Missing: {summary['missing']}\n\n"
        )
        f.write("| Kind | Path | Equal | Repo lines | Release lines | Repo-only | Release-only |\n")
        f.write("| --- | --- | --- | ---: | ---: | ---: | ---: |\n")
        for e in compare_entries:
            f.write(
                f"| {e['kind']} | `{e['path']}` | {e.get('equal', False)} | {e.get('left_line_count', '-') } | {e.get('right_line_count', '-') } | {e.get('extra_in_left', '-') } | {e.get('extra_in_right', '-') } |\n"
            )

    print(f"[ok] report json: {json_path}")
    print(f"[ok] report md:   {md_path}")
    print(f"[ok] summary: {summary}")


if __name__ == "__main__":
    main()
