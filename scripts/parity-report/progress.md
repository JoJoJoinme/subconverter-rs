# Parity Progress Log

## 2026-02-12 (Stage Snapshot)

- Baseline moved from `PASS 1 / PARTIAL 5 / FAIL 3 / SKIP 3` to `PASS 4 / PARTIAL 5 / FAIL 0 / SKIP 3`.
- Added semantic parity runner and reports: `scripts/compare_with_subconverter.py`, `scripts/parity-report/compat_report.md`, `scripts/parity-report/compat_report.json`.
- Added missing web endpoints: `/version`, `/getprofile`, `/getruleset` in `src/web_handlers/web_api.rs`.
- Fixed path/import compatibility for profile/ruleset/template loading (`base/` fallback paths).
- Fixed non-trivial format/output issues:
  - QuanX/Surge/Loon/Mellow/Quan INI `{NONAME}` serialization (`set_current`) to remove trailing `=` artifacts.
  - Clash template variable handling and `clash.new_field_name` propagation.
  - Clash script mode now emits `rule-providers` and `script.code` skeleton aligned with upstream shape.
- Current top remaining parity gaps:
  - `sub_script`: script code text still has minor formatting delta(s).
  - `sub_clash` / `alias_clash` / `getprofile`: ruleset content still semantically differs.
  - `sub_singbox`: structural semantic differences remain.

## Next Focus

1. Finish `sub_script` exact code-text alignment.
2. Reconcile clash-family ruleset content deltas.
3. Continue singbox semantic alignment.

## 2026-02-13 (Stage Update)

- Completed `sub_script` parity lift to PASS by aligning Clash script-mode semantics:
  - Generated `rule-providers` and `script.code` in `src/generator/exports/proxy_to_clash.rs`.
  - Matched provider naming/behavior layout (domain/ipcidr/classical split).
  - Matched script text formatting details (including trailing newline behavior).
- Fixed Shadowsocks Clash output to avoid emitting empty `plugin-opts: {}` when options are empty: `src/generator/yaml/clash/output_proxy_types/clash_output_shadowsocks.rs`.
- Current parity: `PASS 5 / PARTIAL 4 / FAIL 0 / SKIP 3`.
- Remaining PARTIAL cases: `sub_clash`, `sub_singbox`, `getprofile`, `alias_clash`.

## 2026-02-13 (Stage Update 2)

- Achieved semantic parity for all valid compare cases when both services use the same release resource set:
  - Result: `PASS 9 / PARTIAL 0 / FAIL 0 / SKIP 3`.
  - `SKIP` cases remain the upstream-invalid ones (`sub_auto`, `surge2clash`, `render`) where original returns 400.
- Fixed `getprofile` compare mode in parity script from text to YAML to avoid formatting-only false diffs.
- Fixed sing-box TLS parity by always emitting `tls.insecure` (default `false`) when TLS is enabled:
  - `src/generator/config/formats/singbox.rs`
- Added parity script preflight port guards to prevent stale-process contamination:
  - `scripts/compare_with_subconverter.py`

## 2026-02-13 (Stage Update 3)

- Standardized parity execution into profiles in `scripts/compare_with_subconverter.py`:
  - `--profile code-parity`: release resources baseline.
  - `--profile repo-parity`: repository resources baseline.
  - `--profile custom`: manual paths.
- Reports now include profile/settings metadata in both JSON and Markdown outputs.
- Added resource drift report script: `scripts/report_resource_diff.py`.
  - Output: `scripts/parity-report/resources/resource_diff.json`
  - Output: `scripts/parity-report/resources/resource_diff.md`
  - Current resource summary: `total 19 / equal 12 / different 7 / missing 0`.
- Current parity snapshots:
  - `scripts/parity-report/code/compat_report.md`: `PASS 9 / PARTIAL 0 / FAIL 0 / SKIP 3`
  - `scripts/parity-report/repo/compat_report.md`: profile snapshot varies with local resource set.

## 2026-02-13 (Stage Update 4)

- Added one-shot suite entrypoint: `scripts/run_parity_suite.py`.
  - Runs `code-parity`, `repo-parity`, and `resource_diff` in sequence.
  - Writes suite summary to:
    - `scripts/parity-report/suite/summary.json`
    - `scripts/parity-report/suite/summary.md`
- Latest suite run snapshot:
  - Code parity: `PASS 9 / PARTIAL 0 / FAIL 0 / SKIP 3`
  - Repo parity: `PASS 5 / PARTIAL 4 / FAIL 0 / SKIP 3`
  - Resource diff: `total 19 / equal 12 / different 7 / missing 0`

## 2026-02-13 (Stage Update 5)

- Added strict gate support to suite runner: `scripts/run_parity_suite.py --strict`.
- Strict rule: fail only when `code-parity` has any `PARTIAL`/`FAIL` or pass-count mismatch (excluding `SKIP`).
- Strict result is now persisted in suite summary outputs:
  - `scripts/parity-report/suite/summary.json`
  - `scripts/parity-report/suite/summary.md`
