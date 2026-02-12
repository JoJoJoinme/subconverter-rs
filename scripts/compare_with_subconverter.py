#!/usr/bin/env python3
import argparse
import base64
import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import asdict, dataclass

import yaml


REPO_RS_CONFIG = "/srv/work/subconverter-rs/base/pref.example.ini"
REPO_RS_WORKDIR = "/srv/work/subconverter-rs"
RELEASE_CONFIG = "/tmp/subconverter-release/subconverter/pref.example.ini"
RELEASE_WORKDIR = "/tmp/subconverter-release/subconverter"


@dataclass
class HttpResult:
    status: int
    content_type: str
    body: str
    error: str


def b64_urlsafe_no_pad(value: str) -> str:
    return base64.urlsafe_b64encode(value.encode("utf-8")).decode("ascii").rstrip("=")


def http_get(url: str, timeout: int = 20) -> HttpResult:
    try:
        req = urllib.request.Request(url=url, method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read()
            return HttpResult(
                status=resp.status,
                content_type=resp.headers.get("content-type", ""),
                body=raw.decode("utf-8", errors="replace"),
                error="",
            )
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace") if e.fp else ""
        return HttpResult(status=e.code, content_type=e.headers.get("content-type", ""), body=body, error="")
    except Exception as e:
        return HttpResult(status=0, content_type="", body="", error=str(e))


def normalize_text(value: str) -> str:
    lines = [line.rstrip() for line in value.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines)


def canonicalize(value):
    if isinstance(value, dict):
        return {k: canonicalize(value[k]) for k in sorted(value.keys())}
    if isinstance(value, list):
        return [canonicalize(v) for v in value]
    return value


def semantic_equal(kind: str, left: str, right: str):
    if kind == "version":
        # Treat version endpoint as semantically equal when both expose
        # "subconverter v* backend" format; exact version number is expected
        # to differ between implementations.
        left_n = normalize_text(left).lower()
        right_n = normalize_text(right).lower()
        ok = (
            left_n.startswith("subconverter v")
            and right_n.startswith("subconverter v")
            and left_n.endswith(" backend")
            and right_n.endswith(" backend")
        )
        return ok, "version"
    if kind == "json":
        return canonicalize(json.loads(left)) == canonicalize(json.loads(right)), "json"
    if kind == "yaml":
        return canonicalize(yaml.safe_load(left)) == canonicalize(yaml.safe_load(right)), "yaml"
    return normalize_text(left) == normalize_text(right), "text"


def wait_ready(url: str, timeout_sec: int = 45) -> bool:
    deadline = time.time() + timeout_sec
    while time.time() < deadline:
        res = http_get(url, timeout=5)
        if 200 <= res.status < 500:
            return True
        time.sleep(1)
    return False


def is_port_in_use(port: int, host: str = "127.0.0.1") -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.settimeout(0.5)
        return sock.connect_ex((host, port)) == 0


def prepare_rs_config(source_config: str, output_config: str):
    with open(source_config, "r", encoding="utf-8") as f:
        content = f.read()

    # Keep import paths compatible with the selected workdir layout.
    # Some layouts use "snippets/..." at config root, while this repo uses
    # "base/snippets/...".
    config_dir = os.path.dirname(os.path.abspath(source_config))
    snippets_dir = os.path.join(config_dir, "snippets")
    base_snippets_dir = os.path.join(config_dir, "base", "snippets")
    if not os.path.isdir(snippets_dir) and os.path.isdir(base_snippets_dir):
        content = content.replace("!!import:snippets/", "!!import:base/snippets/")

    with open(output_config, "w", encoding="utf-8") as f:
        f.write(content)


def start_rust_server(port: int, config_path: str, workdir: str):
    cmd = [
        "/srv/work/subconverter-rs/target/release/subconverter-rs",
        "--config",
        config_path,
        "--address",
        "127.0.0.1",
        "--port",
        str(port),
    ]
    process = subprocess.Popen(
        cmd,
        cwd=workdir,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return process


def start_original_docker(port: int, container_name: str):
    run_cmd = [
        "docker",
        "run",
        "-d",
        "--rm",
        "--name",
        container_name,
        "-p",
        f"{port}:25500",
        "tindy2013/subconverter:latest",
    ]
    out = subprocess.check_output(run_cmd, text=True).strip()
    return out


def start_original_local(port: int, binary: str, config: str, workdir: str):
    cmd = [binary, "-f", config]
    env = dict(os.environ)
    env["PORT"] = str(port)
    process = subprocess.Popen(
        cmd,
        cwd=workdir,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return process


def stop_original_docker(container_name: str):
    subprocess.run(["docker", "rm", "-f", container_name], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def build_cases():
    ss_link = "ss://YWVzLTI1Ni1nY206cGFzc0BleGFtcGxlLmNvbTo0NDM=#NodeA"
    trojan_link = "trojan://password@example.org:443#NodeB"
    mixed_link = f"{ss_link}|{trojan_link}"
    ruleset_url = b64_urlsafe_no_pad("rules/LocalAreaNetwork.list")

    return [
        {"id": "version", "feature": "Version Endpoint", "kind": "version", "path": "/version"},
        {
            "id": "sub_clash",
            "feature": "Sub Basic Clash",
            "kind": "yaml",
            "path": f"/sub?target=clash&url={urllib.parse.quote(ss_link, safe='')}",
        },
        {
            "id": "sub_ss",
            "feature": "Sub Basic SS",
            "kind": "text",
            "path": f"/sub?target=ss&url={urllib.parse.quote(ss_link, safe='')}",
        },
        {
            "id": "sub_quanx",
            "feature": "Sub QuanX",
            "kind": "text",
            "path": f"/sub?target=quanx&url={urllib.parse.quote(ss_link, safe='')}",
        },
        {
            "id": "sub_singbox",
            "feature": "Sub SingBox",
            "kind": "json",
            "path": f"/sub?target=singbox&url={urllib.parse.quote(mixed_link, safe='')}",
        },
        {
            "id": "sub_auto",
            "feature": "Target Auto",
            "kind": "text",
            "path": f"/sub?target=auto&url={urllib.parse.quote(ss_link, safe='')}",
        },
        {
            "id": "sub_script",
            "feature": "Clash Script Param",
            "kind": "yaml",
            "path": f"/sub?target=clash&script=true&url={urllib.parse.quote(ss_link, safe='')}",
        },
        {
            "id": "surge2clash",
            "feature": "Surge2Clash Endpoint",
            "kind": "yaml",
            "path": f"/surge2clash?url={urllib.parse.quote(ss_link, safe='')}",
        },
        {
            "id": "getprofile",
            "feature": "GetProfile Endpoint",
            "kind": "yaml",
            "path": "/getprofile?name=profiles/example_profile.ini&token=password",
        },
        {
            "id": "getruleset",
            "feature": "GetRuleset Endpoint",
            "kind": "text",
            "path": f"/getruleset?type=1&url={urllib.parse.quote(ruleset_url, safe='')}&group=DIRECT",
        },
        {
            "id": "render",
            "feature": "Render Endpoint",
            "kind": "text",
            "path": "/render?path=base/all_base.tpl",
        },
        {
            "id": "alias_clash",
            "feature": "Alias Endpoint Clash",
            "kind": "yaml",
            "path": f"/clash?url={urllib.parse.quote(ss_link, safe='')}",
        },
    ]


def main():
    parser = argparse.ArgumentParser(description="Semantic parity compare: subconverter vs subconverter-rs")
    parser.add_argument("--orig-port", type=int, default=19500)
    parser.add_argument("--rs-port", type=int, default=19501)
    parser.add_argument("--profile", choices=["repo-parity", "code-parity", "custom"], default="custom")
    parser.add_argument("--out-dir")
    parser.add_argument("--rs-config")
    parser.add_argument("--rs-workdir")
    parser.add_argument("--orig-mode", choices=["local", "docker"], default="local")
    parser.add_argument(
        "--orig-bin",
        default="/tmp/subconverter-release/subconverter/subconverter",
        help="Path to original subconverter binary when --orig-mode=local",
    )
    parser.add_argument(
        "--orig-config",
        default=RELEASE_CONFIG,
        help="Path to original subconverter config when --orig-mode=local",
    )
    parser.add_argument(
        "--orig-workdir",
        default=RELEASE_WORKDIR,
        help="Working directory for original subconverter when --orig-mode=local",
    )
    parser.add_argument("--no-start", action="store_true", help="Use already running services")
    parser.add_argument("--keep-running", action="store_true", help="Do not stop services after compare")
    args = parser.parse_args()

    if args.profile == "repo-parity":
        if args.rs_config is None:
            args.rs_config = REPO_RS_CONFIG
        if args.rs_workdir is None:
            args.rs_workdir = REPO_RS_WORKDIR
        if args.out_dir is None:
            args.out_dir = "scripts/parity-report/repo"
    elif args.profile == "code-parity":
        if args.rs_config is None:
            args.rs_config = RELEASE_CONFIG
        if args.rs_workdir is None:
            args.rs_workdir = RELEASE_WORKDIR
        if args.out_dir is None:
            args.out_dir = "scripts/parity-report/code"
    else:
        if args.rs_config is None:
            args.rs_config = REPO_RS_CONFIG
        if args.rs_workdir is None:
            args.rs_workdir = REPO_RS_WORKDIR
        if args.out_dir is None:
            args.out_dir = "scripts/parity-report"

    os.makedirs(args.out_dir, exist_ok=True)

    rust_proc = None
    orig_proc = None
    container_name = f"subconverter-parity-{int(time.time())}"
    rs_temp_config = f"/tmp/subconverter-rs-pref-{int(time.time())}.ini"

    orig_base = f"http://127.0.0.1:{args.orig_port}"
    rs_base = f"http://127.0.0.1:{args.rs_port}"

    if not args.no_start:
        if is_port_in_use(args.orig_port):
            raise RuntimeError(
                f"orig port {args.orig_port} already in use; stop existing process or use --orig-port"
            )
        if is_port_in_use(args.rs_port):
            raise RuntimeError(
                f"rs port {args.rs_port} already in use; stop existing process or use --rs-port"
            )

        prepare_rs_config(args.rs_config, rs_temp_config)
        rust_proc = start_rust_server(args.rs_port, rs_temp_config, args.rs_workdir)
        if args.orig_mode == "docker":
            start_original_docker(args.orig_port, container_name)
        else:
            orig_proc = start_original_local(args.orig_port, args.orig_bin, args.orig_config, args.orig_workdir)

    try:
        if not wait_ready(orig_base + "/version"):
            raise RuntimeError("Original subconverter did not become ready")
        if not wait_ready(rs_base + "/"):
            raise RuntimeError("subconverter-rs did not become ready")

        results = []
        for case in build_cases():
            orig_res = http_get(orig_base + case["path"])
            rs_res = http_get(rs_base + case["path"])

            case_result = {
                "id": case["id"],
                "feature": case["feature"],
                "kind": case["kind"],
                "path": case["path"],
                "original": asdict(orig_res),
                "rust": asdict(rs_res),
            }

            if orig_res.status == 0 or rs_res.status == 0:
                case_result["status"] = "FAIL"
                case_result["note"] = "request_error"
            elif orig_res.status >= 400:
                case_result["status"] = "SKIP"
                case_result["note"] = "original_case_invalid"
            elif rs_res.status >= 400:
                case_result["status"] = "FAIL"
                case_result["note"] = "rust_http_error"
            else:
                try:
                    equal, mode = semantic_equal(case["kind"], orig_res.body, rs_res.body)
                    case_result["compare_mode"] = mode
                    case_result["status"] = "PASS" if equal else "PARTIAL"
                    case_result["note"] = "semantic_match" if equal else "semantic_diff"
                except Exception as e:
                    case_result["status"] = "PARTIAL"
                    case_result["note"] = f"compare_error:{e}"

            results.append(case_result)

        summary = {
            "PASS": sum(1 for r in results if r["status"] == "PASS"),
            "PARTIAL": sum(1 for r in results if r["status"] == "PARTIAL"),
            "FAIL": sum(1 for r in results if r["status"] == "FAIL"),
            "SKIP": sum(1 for r in results if r["status"] == "SKIP"),
            "total": len(results),
        }

        json_path = os.path.join(args.out_dir, "compat_report.json")
        with open(json_path, "w", encoding="utf-8") as f:
            json.dump(
                {
                    "profile": args.profile,
                    "settings": {
                        "rs_config": args.rs_config,
                        "rs_workdir": args.rs_workdir,
                        "orig_mode": args.orig_mode,
                        "orig_config": args.orig_config,
                        "orig_workdir": args.orig_workdir,
                    },
                    "summary": summary,
                    "results": results,
                },
                f,
                ensure_ascii=False,
                indent=2,
            )

        md_path = os.path.join(args.out_dir, "compat_report.md")
        with open(md_path, "w", encoding="utf-8") as f:
            f.write("# Subconverter Parity Report\n\n")
            f.write("Semantic compare between original subconverter and subconverter-rs.\n\n")
            f.write(f"- Profile: {args.profile}\n")
            f.write(f"- Rust config: `{args.rs_config}`\n")
            f.write(f"- Rust workdir: `{args.rs_workdir}`\n")
            f.write(
                f"- Total: {summary['total']}\n- PASS: {summary['PASS']}\n- PARTIAL: {summary['PARTIAL']}\n- FAIL: {summary['FAIL']}\n- SKIP: {summary['SKIP']}\n\n"
            )
            f.write("| Case | Feature | Status | Original | Rust | Note |\n")
            f.write("| --- | --- | --- | --- | --- | --- |\n")
            for r in results:
                f.write(
                    f"| `{r['id']}` | {r['feature']} | {r['status']} | {r['original']['status']} | {r['rust']['status']} | {r['note']} |\n"
                )

        print(f"[ok] report json: {json_path}")
        print(f"[ok] report md:   {md_path}")
        print(f"[ok] summary: {summary}")

    finally:
        if not args.keep_running:
            if rust_proc is not None:
                rust_proc.terminate()
                try:
                    rust_proc.wait(timeout=5)
                except Exception:
                    rust_proc.kill()
            if orig_proc is not None:
                orig_proc.terminate()
                try:
                    orig_proc.wait(timeout=5)
                except Exception:
                    orig_proc.kill()
            if not args.no_start and args.orig_mode == "docker":
                stop_original_docker(container_name)
            if os.path.exists(rs_temp_config):
                os.remove(rs_temp_config)


if __name__ == "__main__":
    main()
