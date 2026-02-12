import argparse
import json
import subprocess
import time
from pathlib import Path


CODE_REPORT_JSON = Path("scripts/parity-report/code/compat_report.json")
REPO_REPORT_JSON = Path("scripts/parity-report/repo/compat_report.json")
RESOURCE_REPORT_JSON = Path("scripts/parity-report/resources/resource_diff.json")


def run_cmd(command: list[str]) -> None:
    result = subprocess.run(command, check=False)
    if result.returncode != 0:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(command)}")


def read_json(path: Path) -> dict:
    if not path.exists():
        raise RuntimeError(f"expected report not found: {path}")
    return json.loads(path.read_text(encoding="utf-8"))


def write_suite_reports(summary: dict) -> tuple[Path, Path]:
    out_dir = Path("scripts/parity-report/suite")
    out_dir.mkdir(parents=True, exist_ok=True)

    json_path = out_dir / "summary.json"
    json_path.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")

    md_path = out_dir / "summary.md"
    with md_path.open("w", encoding="utf-8") as f:
        f.write("# Parity Suite Summary\n\n")
        f.write(f"- Generated at: {summary['generated_at']}\n")
        f.write(f"- Code parity: {summary['code_parity']}\n")
        f.write(f"- Repo parity: {summary['repo_parity']}\n")
        f.write(f"- Resource diff: {summary['resource_diff']}\n\n")
        if "strict" in summary:
            f.write(f"- Strict mode: {summary['strict']}\n\n")
        f.write("## Source Reports\n\n")
        f.write(f"- `{CODE_REPORT_JSON}`\n")
        f.write(f"- `{REPO_REPORT_JSON}`\n")
        f.write(f"- `{RESOURCE_REPORT_JSON}`\n")

    return json_path, md_path


def main() -> None:
    parser = argparse.ArgumentParser(description="Run parity suite (code/repo parity + resource diff)")
    parser.add_argument("--skip-code-parity", action="store_true")
    parser.add_argument("--skip-repo-parity", action="store_true")
    parser.add_argument("--skip-resource-diff", action="store_true")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Fail when code-parity summary is not PASS-only (excluding SKIP)",
    )
    args = parser.parse_args()

    if not args.skip_code_parity:
        run_cmd(["python3", "scripts/compare_with_subconverter.py", "--profile", "code-parity"])

    if not args.skip_repo_parity:
        run_cmd(["python3", "scripts/compare_with_subconverter.py", "--profile", "repo-parity"])

    if not args.skip_resource_diff:
        run_cmd(["python3", "scripts/report_resource_diff.py"])

    code_summary = read_json(CODE_REPORT_JSON).get("summary", {}) if CODE_REPORT_JSON.exists() else {}
    repo_summary = read_json(REPO_REPORT_JSON).get("summary", {}) if REPO_REPORT_JSON.exists() else {}
    resource_summary = (
        read_json(RESOURCE_REPORT_JSON).get("summary", {}) if RESOURCE_REPORT_JSON.exists() else {}
    )

    summary = {
        "generated_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "code_parity": code_summary,
        "repo_parity": repo_summary,
        "resource_diff": resource_summary,
    }

    strict_result = {
        "enabled": args.strict,
        "ok": True,
        "reason": "not_enabled",
    }

    if args.strict:
        if args.skip_code_parity:
            strict_result = {
                "enabled": True,
                "ok": False,
                "reason": "code_parity_skipped",
            }
        else:
            pass_count = int(code_summary.get("PASS", 0))
            partial_count = int(code_summary.get("PARTIAL", 0))
            fail_count = int(code_summary.get("FAIL", 0))
            skip_count = int(code_summary.get("SKIP", 0))
            total_count = int(code_summary.get("total", 0))
            expected_pass = total_count - skip_count
            ok = partial_count == 0 and fail_count == 0 and pass_count == expected_pass
            strict_result = {
                "enabled": True,
                "ok": ok,
                "reason": "pass" if ok else "code_parity_not_clean",
                "expected_pass": expected_pass,
                "actual_pass": pass_count,
                "partial": partial_count,
                "fail": fail_count,
                "skip": skip_count,
                "total": total_count,
            }

    summary["strict"] = strict_result

    json_path, md_path = write_suite_reports(summary)

    print(f"[ok] suite summary json: {json_path}")
    print(f"[ok] suite summary md:   {md_path}")
    print(f"[ok] code parity: {code_summary}")
    print(f"[ok] repo parity: {repo_summary}")
    print(f"[ok] resource diff: {resource_summary}")
    if args.strict:
        if strict_result["ok"]:
            print(f"[ok] strict check: {strict_result}")
        else:
            print(f"[error] strict check failed: {strict_result}")
            raise SystemExit(1)


if __name__ == "__main__":
    main()
