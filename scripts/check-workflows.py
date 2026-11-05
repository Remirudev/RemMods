#!/usr/bin/env python3
"""Workflow security rules W1-W11 over .github/workflows/*.yml (SPEC-P5 §9.2). Needs PyYAML.

Usage: python3 -I scripts/check-workflows.py [--self-test]

W1  every remote `uses:` is owner/repo[/path]@<40 hex> followed by `# vX.Y.Z`; local
    ./.github/workflows/*.yml calls are allowed; a `uses:` line in any other form (flow style, a
    comment that is not one version token) fails, and every parsed steps[].uses and jobs.*.uses
    is checked as well
W2  no pull_request_target, workflow_run or issue_comment trigger
W3  no `${{` inside any `run:` value (values reach scripts through env)
W4  top-level permissions are `contents: read` or `{}`; `contents: write` only on the release.yml
    jobs draft, upload, latest-json, sums, publish; no other write scope anywhere
W5  `secrets.` only in jobs with `environment: release`, only in step-level env (no `secrets:`
    on job calls either)
W6  every actions/checkout step sets persist-credentials: false
W7  release.yml: `on` is exactly push.tags ['v*']; environment release exactly on sign, publish,
    aur-push; no caches (actions/cache, Swatinem/rust-cache, a `cache:` input); every setup-node step
    has package-manager-cache: false; no caches also means no actions/cache/restore or /save; every
    npm command that installs (ci, install, i, add, update, rebuild, ...) has --ignore-scripts; a
    job running `tauri build` has no environment and no secrets
W8  artifacts.yml: no secrets, no environment, pull_request has paths, workflow_call input prebuilt
    is a boolean; every job that downloads bundles-* needs build and has an `if:` with always()
W9  release.yml's verify job runs release-check.py --strict with the tag, and ci.yml runs
    release-check.py --self-test (the gate itself is tested there)
W10 no job with an environment or a secrets reference uses a container
W11 in release.yml and artifacts.yml every Linux job running `tauri build` runs
    scripts/seed-appimage-tools.sh in an earlier step and `seed-appimage-tools.sh --verify-cache`
    in a later one (both without continue-on-error and without an `if:` other than the Linux
    check), so a bundler that skipped the seeded tools and downloaded its own fails the job; that
    script pins the type-2 runtime (runtime-x86_64 from a fixed type2-runtime tag, not
    `continuous`, with a sha256), exports LDAI_RUNTIME_FILE to $GITHUB_ENV, so appimagetool never
    downloads the runtime itself, and names the @tauri-apps/cli version whose bundler was checked
    (VERIFIED_CLI); package-lock.json must lock exactly that CLI, so a Dependabot bump fails here
    until someone re-checks the bundler's tool names and updates the seed
Note: PyYAML loads the `on:` key as True.
"""

import argparse
import json
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - reported, not raised
    print("check-workflows: PyYAML is missing (apt-get install python3-yaml)", file=sys.stderr)
    sys.exit(2)

REPO = Path(__file__).resolve().parent.parent
WORKFLOWS = REPO / ".github" / "workflows"
USES_LINE = re.compile(r"^\s*(?:-\s+)?uses:\s*(['\"]?)([^\s'\"#]+)\1\s*(?:#\s*(\S+))?\s*$")
USES_ANY = re.compile(r"^\s*(?:-\s*)?(?:\{\s*)?['\"]?uses['\"]?\s*:")
REMOTE_USES = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/[^@\s]+)?@[0-9a-f]{40}")
LOCAL_USES = re.compile(r"\./\.github/workflows/[A-Za-z0-9_.-]+\.ya?ml")
VERSION_COMMENT = re.compile(r"v\d+\.\d+\.\d+")
FORBIDDEN_TRIGGERS = {"pull_request_target", "workflow_run", "issue_comment"}
RELEASE_WRITE_JOBS = {"draft", "upload", "latest-json", "sums", "publish"}
RELEASE_ENV_JOBS = {"sign", "publish", "aur-push"}
TAURI_BUILD = re.compile(r"\btauri\b(?:\s+--)?\s+build\b")
SEED = "scripts/seed-appimage-tools.sh"
SEED_NAME = "seed-appimage-tools.sh"
SEED_RUNTIME = re.compile(
    r'"runtime-x86_64\|\$RELEASES/AppImage/type2-runtime/releases/download/(?!continuous/)[^/|"]+/runtime-x86_64\|[0-9a-f]{64}"'
)
SEED_RUNTIME_ENV = re.compile(r'LDAI_RUNTIME_FILE=.*>>\s*"\$GITHUB_ENV"')
SEED_VERIFY = "--verify-cache"
SEED_VERIFIED_CLI = re.compile(r"^readonly VERIFIED_CLI=([0-9A-Za-z.+-]+)$", re.MULTILINE)
LOCK_NAME = "package-lock.json"
LOCKED_CLI_PACKAGES = ("node_modules/@tauri-apps/cli", "node_modules/@tauri-apps/cli-linux-x64-gnu")
LINUX_ONLY_IF = {"runner.os == 'Linux'", "${{ runner.os == 'Linux' }}"}
NPM_INSTALLS = re.compile(
    r"\bnpm\s+(?:ci|clean-install|install|i|in|ins|inst|insta|instal|isnt|isnta|isntal|isntall|add|update|up|"
    r"upgrade|udpate|rebuild|rb|it|cit|install-test|install-ci-test|install-clean|ic|link|ln)\b"
)


def triggers(doc):
    on = doc.get("on", doc.get(True))
    if isinstance(on, str):
        return {on: None}
    if isinstance(on, list):
        return {name: None for name in on}
    return on if isinstance(on, dict) else {}


def jobs(doc):
    value = doc.get("jobs")
    return value if isinstance(value, dict) else {}


def steps(job):
    value = job.get("steps")
    return [s for s in value if isinstance(s, dict)] if isinstance(value, list) else []


def environment_name(job):
    env = job.get("environment")
    if isinstance(env, dict):
        return env.get("name")
    return env


def walk(value, path=()):
    yield path, value
    if isinstance(value, dict):
        for key, item in value.items():
            yield from walk(item, path + (key,))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from walk(item, path + (index,))


def has_secret_ref(value):
    return any(isinstance(v, str) and "secrets." in v for _, v in walk(value))


def uses_action(step, action):
    uses = step.get("uses")
    return isinstance(uses, str) and uses.split("@", 1)[0] == action


def uses_action_family(step, action):
    uses = step.get("uses")
    path = uses.split("@", 1)[0] if isinstance(uses, str) else ""
    return path == action or path.startswith(action + "/")


def runs_tauri_build(job):
    return any(isinstance(s.get("run"), str) and TAURI_BUILD.search(s["run"]) for s in steps(job))


def is_linux_job(job):
    texts = [str(job.get("runs-on", ""))]
    strategy = job.get("strategy")
    if isinstance(strategy, dict):
        texts += [str(v) for _, v in walk(strategy.get("matrix", {})) if isinstance(v, str)]
    if job.get("container"):
        return True
    return any("ubuntu" in text or "linux" in text.lower() for text in texts)


def downloads_bundles(job):
    for step in steps(job):
        if uses_action(step, "actions/download-artifact"):
            with_ = step.get("with") or {}
            names = [str(with_.get("name", "")), str(with_.get("pattern", ""))]
            if any(name.startswith("bundles-") for name in names):
                return True
    return False


def needs_list(job):
    needs = job.get("needs", [])
    return [needs] if isinstance(needs, str) else list(needs or [])


def check_file(name, text, doc):
    problems = []

    def fail(rule, message):
        problems.append((rule, name, message))

    for number, line in enumerate(text.splitlines(), 1):
        match = USES_LINE.match(line)
        if not match:
            if USES_ANY.match(line):
                fail("W1", f"line {number}: unparseable `uses:` line (expected `uses: owner/repo@<sha> # vX.Y.Z`)")
            continue
        target, comment = match.group(2), match.group(3)
        if LOCAL_USES.fullmatch(target):
            continue
        if not REMOTE_USES.fullmatch(target):
            fail("W1", f"line {number}: `uses: {target}` is not pinned to a full commit SHA")
        elif not (comment and VERSION_COMMENT.fullmatch(comment)):
            fail("W1", f"line {number}: `uses: {target}` lacks a `# vX.Y.Z` comment")
    for job_id, job in jobs(doc).items():
        uses = job.get("uses")
        if isinstance(uses, str) and not LOCAL_USES.fullmatch(uses) and not REMOTE_USES.fullmatch(uses):
            fail("W1", f"job {job_id}: reusable workflow {uses} is not pinned")
        for index, step in enumerate(steps(job)):
            uses = step.get("uses")
            if uses is None:
                continue
            if not (isinstance(uses, str) and (LOCAL_USES.fullmatch(uses) or REMOTE_USES.fullmatch(uses))):
                fail("W1", f"job {job_id} step {index + 1}: uses {uses!r} is not pinned")

    bad = FORBIDDEN_TRIGGERS & set(triggers(doc))
    if bad:
        fail("W2", f"forbidden triggers: {', '.join(sorted(bad))}")

    for job_id, job in jobs(doc).items():
        for index, step in enumerate(steps(job)):
            if isinstance(step.get("run"), str) and "${{" in step["run"]:
                fail("W3", f"job {job_id} step {index + 1} ({step.get('name', '?')}): `${{{{` inside run")

    top = doc.get("permissions")
    if top not in ({"contents": "read"}, {}):
        fail("W4", f"top-level permissions are {top!r}, expected contents: read or {{}}")
    for job_id, job in jobs(doc).items():
        perms = job.get("permissions")
        if perms is None:
            continue
        if isinstance(perms, str):
            if perms != "read-all":
                fail("W4", f"job {job_id}: permissions {perms!r}")
            continue
        for scope, level in perms.items():
            if level != "write":
                continue
            if scope == "contents" and name == "release.yml" and job_id in RELEASE_WRITE_JOBS:
                continue
            fail("W4", f"job {job_id}: {scope}: write is not allowed here")

    for path, value in walk(doc):
        if not (isinstance(value, str) and "secrets." in value):
            continue
        job_id = path[1] if len(path) > 1 and path[0] == "jobs" else None
        in_step_env = len(path) >= 6 and path[0] == "jobs" and path[2] == "steps" and path[4] == "env"
        if not in_step_env:
            fail("W5", f"secrets reference outside a step env at {'.'.join(map(str, path))}")
        elif environment_name(jobs(doc)[job_id]) != "release":
            fail("W5", f"job {job_id} uses secrets without environment: release")
    for job_id, job in jobs(doc).items():
        if "secrets" in job:
            fail("W5", f"job {job_id} passes secrets to a reusable workflow")

    for job_id, job in jobs(doc).items():
        for step in steps(job):
            if uses_action(step, "actions/checkout"):
                value = (step.get("with") or {}).get("persist-credentials")
                if value is not False and str(value).lower() != "false":
                    fail("W6", f"job {job_id}: checkout without persist-credentials: false")

    if name == "release.yml":
        problems += check_release(name, doc)
    if name == "artifacts.yml":
        problems += check_artifacts(name, doc)

    for job_id, job in jobs(doc).items():
        if job.get("container") and (job.get("environment") or has_secret_ref(job)):
            fail("W10", f"job {job_id} has an environment or secrets and runs in a container")

    if name in ("release.yml", "artifacts.yml"):
        for job_id, job in jobs(doc).items():
            if not (runs_tauri_build(job) and is_linux_job(job)):
                continue
            job_steps = steps(job)
            builds = [i for i, s in enumerate(job_steps) if isinstance(s.get("run"), str) and TAURI_BUILD.search(s["run"])]
            if not any(seed_step(s, verify=False) for s in job_steps[: builds[0]]):
                fail("W11", f"job {job_id} runs tauri build without {SEED} before it")
            if not any(seed_step(s, verify=True) for s in job_steps[builds[-1] + 1 :]):
                fail("W11", f"job {job_id} runs tauri build without `{SEED} {SEED_VERIFY}` after it")
    return problems


def seed_step(step, verify):
    run = step.get("run")
    if not (isinstance(run, str) and SEED in run and (SEED_VERIFY in run) == verify):
        return False
    if step.get("continue-on-error") not in (None, False):
        return False
    return step.get("if") is None or str(step.get("if")).strip() in LINUX_ONLY_IF


def check_release(name, doc):
    problems = []

    def fail(rule, message):
        problems.append((rule, name, message))

    if triggers(doc) != {"push": {"tags": ["v*"]}}:
        fail("W7", f"`on` must be exactly push.tags ['v*'], got {triggers(doc)!r}")
    with_env = {job_id for job_id, job in jobs(doc).items() if job.get("environment")}
    for job_id in with_env - RELEASE_ENV_JOBS:
        fail("W7", f"job {job_id} must not have an environment")
    for job_id in RELEASE_ENV_JOBS:
        if environment_name(jobs(doc).get(job_id, {})) != "release":
            fail("W7", f"job {job_id} must have environment: release")
    for job_id, job in jobs(doc).items():
        for step in steps(job):
            if uses_action_family(step, "actions/cache") or uses_action_family(step, "Swatinem/rust-cache"):
                fail("W7", f"job {job_id}: cache action {step.get('uses')}")
            with_ = step.get("with") or {}
            if "cache" in with_:
                fail("W7", f"job {job_id}: `cache:` input on {step.get('uses')}")
            if uses_action(step, "actions/setup-node") and with_.get("package-manager-cache") is not False:
                fail("W7", f"job {job_id}: setup-node without package-manager-cache: false")
            run = step.get("run")
            if isinstance(run, str):
                for line in run.splitlines():
                    for install in NPM_INSTALLS.finditer(line):
                        command = re.split(r"[;&|]", line[install.start() :], maxsplit=1)[0]
                        if "--ignore-scripts" not in command.split():
                            fail("W7", f"job {job_id}: `{command.strip()}` without --ignore-scripts")
        if runs_tauri_build(job) and (job.get("environment") or has_secret_ref(job)):
            fail("W7", f"job {job_id} runs tauri build with an environment or secrets")
    verify = jobs(doc).get("verify", {})
    if not any(isinstance(s.get("run"), str) and re.search(r"release-check\.py\s+--tag\s+\"\$TAG\"\s+--strict", s["run"])
               for s in steps(verify)):
        fail("W9", "job verify does not run release-check.py --tag \"$TAG\" --strict")
    return problems


def check_artifacts(name, doc):
    problems = []

    def fail(rule, message):
        problems.append((rule, name, message))

    if has_secret_ref(doc) or any("secrets" in job for job in jobs(doc).values()):
        fail("W8", "artifacts.yml must not use secrets")
    for job_id, job in jobs(doc).items():
        if job.get("environment"):
            fail("W8", f"job {job_id} must not have an environment")
    on = triggers(doc)
    pull_request = on.get("pull_request")
    if "pull_request" in on and not (isinstance(pull_request, dict) and pull_request.get("paths")):
        fail("W8", "pull_request trigger without paths")
    call = on.get("workflow_call")
    prebuilt = ((call or {}).get("inputs") or {}).get("prebuilt") if isinstance(call, dict) else None
    if not (isinstance(prebuilt, dict) and prebuilt.get("type") == "boolean"):
        fail("W8", "workflow_call input prebuilt must be a boolean")
    for job_id, job in jobs(doc).items():
        if not downloads_bundles(job):
            continue
        if "build" not in needs_list(job):
            fail("W8", f"job {job_id} downloads bundles-* without needs: build")
        if "always()" not in str(job.get("if", "")):
            fail("W8", f"job {job_id} downloads bundles-* without always() in its if")
    return problems


def check_seed_script(text, lock_text):
    if text is None:
        return [("W11", SEED_NAME, "missing")]
    problems = []
    if not SEED_RUNTIME.search(text):
        problems.append(("W11", SEED_NAME, "no runtime-x86_64 entry pinned to a fixed type2-runtime tag with a sha256"))
    if not SEED_RUNTIME_ENV.search(text):
        problems.append(("W11", SEED_NAME, "LDAI_RUNTIME_FILE is not exported to $GITHUB_ENV"))
    verified = SEED_VERIFIED_CLI.findall(text)
    if len(verified) != 1:
        problems.append(("W11", SEED_NAME, "needs exactly one `readonly VERIFIED_CLI=<version>` line"))
        return problems
    problems += check_locked_cli(lock_text, verified[0])
    return problems


def check_locked_cli(lock_text, verified):
    if lock_text is None:
        return [("W11", LOCK_NAME, "missing")]
    try:
        packages = json.loads(lock_text).get("packages") or {}
    except (ValueError, AttributeError):
        return [("W11", LOCK_NAME, "not a JSON object")]
    problems = []
    for package in LOCKED_CLI_PACKAGES:
        locked = (packages.get(package) or {}).get("version") if isinstance(packages, dict) else None
        if locked != verified:
            problems.append(("W11", LOCK_NAME, (
                f"{package.removeprefix('node_modules/')} is {locked!r}, but {SEED_NAME} was checked against "
                f"VERIFIED_CLI {verified}: re-check the new tauri-bundler's linuxdeploy.rs prepare_tools (tool "
                f"names, URLs, tools folder), update the seed's hashes if needed, then VERIFIED_CLI"
            )))
    return problems


def check_ci_runs_gate_self_test(docs):
    ci = docs.get("ci.yml")
    if ci is None:
        return [("W9", "ci.yml", "missing")]
    runs = [s.get("run", "") for job in jobs(ci[1]).values() for s in steps(job)]
    if not any(isinstance(r, str) and "release-check.py --self-test" in r for r in runs):
        return [("W9", "ci.yml", "no step runs release-check.py --self-test")]
    return []


def check_all(texts):
    docs, problems = {}, []
    for name, text in sorted(texts.items()):
        if name in (SEED_NAME, LOCK_NAME):
            continue
        try:
            docs[name] = (text, yaml.safe_load(text) or {})
        except yaml.YAMLError as error:
            problems.append(("YAML", name, str(error).replace("\n", " ")))
    for name, (text, doc) in docs.items():
        problems += check_file(name, text, doc)
    problems += check_ci_runs_gate_self_test(docs)
    problems += check_seed_script(texts.get(SEED_NAME), texts.get(LOCK_NAME))
    for required in ("release.yml", "artifacts.yml"):
        if required not in docs:
            problems.append(("W7" if required == "release.yml" else "W8", required, "missing"))
    return problems


# ---------------------------------------------------------------- self-test

SHA = "3d3c42e5aac5ba805825da76410c181273ba90b1"
BASE_CI = f"""name: ci
on:
  pull_request:
permissions:
  contents: read
jobs:
  policy:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@{SHA} # v7.0.1
        with:
          persist-credentials: false
      - run: python3 -I scripts/release-check.py --self-test
"""
BASE_RELEASE = f"""name: release
on:
  push:
    tags: ["v*"]
permissions:
  contents: read
jobs:
  verify:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@{SHA} # v7.0.1
        with:
          persist-credentials: false
      - env:
          TAG: ${{{{ github.ref_name }}}}
        run: python3 -I scripts/release-check.py --tag "$TAG" --strict
  draft:
    needs: verify
    runs-on: ubuntu-24.04
    permissions:
      contents: write
    steps:
      - run: gh release create "$TAG" --draft
  build:
    needs: verify
    runs-on: ${{{{ matrix.os }}}}
    strategy:
      matrix:
        include:
          - os: ubuntu-24.04
          - os: windows-2025
    steps:
      - uses: actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0
        with:
          package-manager-cache: false
      - run: npm ci --ignore-scripts
      - if: runner.os == 'Linux'
        run: bash scripts/seed-appimage-tools.sh
      - run: npm run tauri -- build --config src-tauri/tauri.release.conf.json --no-sign
      - if: runner.os == 'Linux'
        run: bash scripts/seed-appimage-tools.sh --verify-cache
  artifacts:
    needs: build
    uses: ./.github/workflows/artifacts.yml
    with:
      prebuilt: true
  sign:
    needs: build
    runs-on: ubuntu-24.04
    environment: release
    steps:
      - env:
          TAURI_SIGNING_PRIVATE_KEY: ${{{{ secrets.TAURI_SIGNING_PRIVATE_KEY }}}}
        run: node_modules/.bin/tauri signer sign "$FILE"
  publish:
    needs: sign
    runs-on: ubuntu-24.04
    environment: release
    permissions:
      contents: write
    steps:
      - run: gh release edit "$TAG" --draft=false
  aur-check:
    needs: publish
    runs-on: ubuntu-24.04
    container: archlinux:latest
    steps:
      - run: namcap PKGBUILD
  aur-push:
    needs: aur-check
    runs-on: ubuntu-24.04
    environment: release
    steps:
      - env:
          AUR_SSH_PRIVATE_KEY: ${{{{ secrets.AUR_SSH_PRIVATE_KEY }}}}
        run: git push
"""
BASE_ARTIFACTS = f"""name: artifacts
on:
  workflow_call:
    inputs:
      prebuilt:
        type: boolean
        default: false
  pull_request:
    paths:
      - packaging/**
permissions:
  contents: read
jobs:
  build:
    if: ${{{{ !inputs.prebuilt }}}}
    runs-on: ubuntu-24.04
    steps:
      - run: bash scripts/seed-appimage-tools.sh
      - run: npm run tauri -- build
      - run: bash scripts/seed-appimage-tools.sh --verify-cache
  linux:
    needs: build
    if: ${{{{ always() && !cancelled() && needs.build.result != 'failure' }}}}
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8.0.2
        with:
          name: bundles-linux
"""
BASE_SEED = """readonly VERIFIED_CLI=2.12.4
readonly TOOLS=(
    "runtime-x86_64|$RELEASES/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64|""" + "a" * 64 + """"
)
readonly RUNTIME=runtime-x86_64
if [[ -n ${GITHUB_ENV:-} ]]; then
    printf 'LDAI_RUNTIME_FILE=%s\\n' "$cache_dir/$RUNTIME" >> "$GITHUB_ENV"
fi
"""
BASE_LOCK = """{"packages": {
  "node_modules/@tauri-apps/cli": {"version": "2.12.4"},
  "node_modules/@tauri-apps/cli-linux-x64-gnu": {"version": "2.12.4"}
}}
"""
CHECKOUT = f"      - uses: actions/checkout@{SHA} # v7.0.1\n        with:\n          persist-credentials: false\n"

MUTATIONS = [
    ("W1", "ci.yml", f"actions/checkout@{SHA} # v7.0.1", "actions/checkout@v7 # v7.0.1"),
    ("W1", "ci.yml", f"actions/checkout@{SHA} # v7.0.1", f"actions/checkout@{SHA}"),
    ("W1", "ci.yml", f"actions/checkout@{SHA} # v7.0.1", f"actions/checkout@{SHA[:-1]} # v7.0.1"),
    ("W1", "ci.yml", f"actions/checkout@{SHA} # v7.0.1", "actions/checkout@main # v7.0.1 (moving)"),
    ("W1", "ci.yml", f"actions/checkout@{SHA} # v7.0.1", "actions/checkout@main  # pinned later"),
    ("W1", "ci.yml", CHECKOUT, "      - {uses: actions/checkout@main, with: {persist-credentials: false}}\n"),
    ("W1", "ci.yml", CHECKOUT, f"      - {{uses: actions/checkout@{SHA}, with: {{persist-credentials: false}}}}\n"),
    ("W1", "ci.yml", CHECKOUT, "      - uses: >-\n          actions/checkout@main\n        with:\n          persist-credentials: false\n"),
    ("W2", "ci.yml", "  pull_request:\n", "  pull_request_target:\n"),
    ("W2", "ci.yml", "  pull_request:\n", "  pull_request:\n  workflow_run:\n    workflows: [ci]\n"),
    ("W3", "ci.yml", "release-check.py --self-test", "release-check.py --self-test ${{ github.ref }}"),
    ("W4", "ci.yml", "permissions:\n  contents: read\n", "permissions: write-all\n"),
    ("W4", "ci.yml", "  policy:\n    runs-on: ubuntu-24.04\n",
     "  policy:\n    runs-on: ubuntu-24.04\n    permissions:\n      contents: write\n"),
    ("W4", "release.yml", "    permissions:\n      contents: write\n    steps:\n      - run: gh release create",
     "    permissions:\n      contents: write\n      packages: write\n    steps:\n      - run: gh release create"),
    ("W4", "release.yml", "  aur-check:\n    needs: publish\n",
     "  aur-check:\n    needs: publish\n    permissions:\n      contents: write\n"),
    ("W5", "release.yml", "    environment: release\n    steps:\n      - env:\n          TAURI",
     "    steps:\n      - env:\n          TAURI"),
    ("W5", "release.yml", "permissions:\n  contents: read\njobs:",
     "permissions:\n  contents: read\nenv:\n  KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}\njobs:"),
    ("W5", "release.yml", "    with:\n      prebuilt: true\n", "    with:\n      prebuilt: true\n    secrets: inherit\n"),
    ("W6", "ci.yml", "persist-credentials: false", "persist-credentials: true"),
    ("W6", "ci.yml", "        with:\n          persist-credentials: false\n", ""),
    ("W7", "release.yml", 'tags: ["v*"]', 'tags: ["*"]'),
    ("W7", "release.yml", '    tags: ["v*"]\n', '    tags: ["v*"]\n  workflow_dispatch:\n'),
    ("W7", "release.yml", "  draft:\n    needs: verify\n    runs-on: ubuntu-24.04\n",
     "  draft:\n    needs: verify\n    runs-on: ubuntu-24.04\n    environment: release\n"),
    ("W7", "release.yml", "          package-manager-cache: false", "          cache: npm\n          package-manager-cache: false"),
    ("W7", "release.yml", "          package-manager-cache: false\n", "          node-version: 26\n"),
    ("W7", "release.yml", "npm ci --ignore-scripts", "npm ci"),
    ("W7", "release.yml", "npm ci --ignore-scripts", "npm install"),
    ("W7", "release.yml", "npm ci --ignore-scripts", "npm i --no-audit"),
    ("W7", "release.yml", "npm ci --ignore-scripts", "npm ci --ignore-scripts && npm rebuild"),
    ("W7", "release.yml", "      - run: npm ci --ignore-scripts\n",
     "      - run: npm ci --ignore-scripts\n      - uses: actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0\n"
     "        with:\n          path: target\n          key: release\n"),
    ("W7", "release.yml", "      - run: npm ci --ignore-scripts\n",
     "      - run: npm ci --ignore-scripts\n      - uses: Swatinem/rust-cache@d1527c8f67ef9af357290475d4af4b0fab8f12e0 # v2.9.3\n"),
    ("W7", "release.yml", "      - run: npm run tauri -- build --config",
     "      - env:\n          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}\n        run: npm run tauri -- build --config"),
    ("W8", "artifacts.yml", "        type: boolean\n", "        type: string\n"),
    ("W8", "artifacts.yml", "  pull_request:\n    paths:\n      - packaging/**\n", "  pull_request:\n"),
    ("W8", "artifacts.yml", "  linux:\n    needs: build\n", "  linux:\n"),
    ("W8", "artifacts.yml", "    if: ${{ always() && !cancelled() && needs.build.result != 'failure' }}\n", ""),
    ("W8", "artifacts.yml", "  linux:\n    needs: build\n", "  linux:\n    needs: build\n    environment: test\n"),
    ("W9", "release.yml", 'release-check.py --tag "$TAG" --strict', 'release-check.py --tag "$TAG"'),
    ("W9", "ci.yml", "release-check.py --self-test", "release-check.py"),
    ("W10", "release.yml", "  aur-push:\n    needs: aur-check\n    runs-on: ubuntu-24.04\n",
     "  aur-push:\n    needs: aur-check\n    runs-on: ubuntu-24.04\n    container: archlinux:latest\n"),
    ("W11", "release.yml", "      - if: runner.os == 'Linux'\n        run: bash scripts/seed-appimage-tools.sh\n", ""),
    ("W11", "artifacts.yml", "      - run: bash scripts/seed-appimage-tools.sh\n      - run: npm run tauri -- build\n",
     "      - run: npm run tauri -- build\n      - run: bash scripts/seed-appimage-tools.sh\n"),
    ("W11", SEED_NAME, "download/20251108/", "download/continuous/"),
    ("W11", SEED_NAME, "runtime-x86_64|" + "a" * 64, "runtime-x86_64|"),
    ("W11", SEED_NAME, ' >> "$GITHUB_ENV"', ""),
    ("W11", "release.yml", "      - if: runner.os == 'Linux'\n        run: bash scripts/seed-appimage-tools.sh --verify-cache\n", ""),
    ("W11", "release.yml", "        run: bash scripts/seed-appimage-tools.sh --verify-cache\n",
     "        continue-on-error: true\n        run: bash scripts/seed-appimage-tools.sh --verify-cache\n"),
    ("W11", "release.yml", "      - if: runner.os == 'Linux'\n        run: bash scripts/seed-appimage-tools.sh --verify-cache\n",
     "      - if: false\n        run: bash scripts/seed-appimage-tools.sh --verify-cache\n"),
    ("W11", "artifacts.yml",
     "      - run: npm run tauri -- build\n      - run: bash scripts/seed-appimage-tools.sh --verify-cache\n",
     "      - run: bash scripts/seed-appimage-tools.sh --verify-cache\n      - run: npm run tauri -- build\n"),
    ("W11", "artifacts.yml", "      - run: bash scripts/seed-appimage-tools.sh\n      - run: npm run tauri -- build\n",
     "      - run: bash scripts/seed-appimage-tools.sh --verify-cache\n      - run: npm run tauri -- build\n"),
    ("W11", SEED_NAME, "readonly VERIFIED_CLI=2.12.4\n", ""),
    ("W11", SEED_NAME, "readonly VERIFIED_CLI=2.12.4\n", "readonly VERIFIED_CLI=2.12.3\n"),
    ("W11", LOCK_NAME, '"node_modules/@tauri-apps/cli": {"version": "2.12.4"}',
     '"node_modules/@tauri-apps/cli": {"version": "2.13.0"}'),
    ("W11", LOCK_NAME, '"node_modules/@tauri-apps/cli-linux-x64-gnu": {"version": "2.12.4"}',
     '"node_modules/@tauri-apps/cli-linux-x64-gnu": {"version": "2.13.0"}'),
    ("W11", LOCK_NAME, '{"packages": {', '[{"packages": {'),
]


def self_test():
    base = {"ci.yml": BASE_CI, "release.yml": BASE_RELEASE, "artifacts.yml": BASE_ARTIFACTS, SEED_NAME: BASE_SEED,
            LOCK_NAME: BASE_LOCK}
    problems = check_all(base)
    if problems:
        raise AssertionError(f"baseline fails: {problems}")
    covered = set()
    for rule, name, old, new in MUTATIONS:
        texts = dict(base)
        if texts[name].count(old) != 1:
            raise AssertionError(f"{rule}: snippet not found exactly once in {name}: {old!r}")
        texts[name] = texts[name].replace(old, new)
        rules = {problem[0] for problem in check_all(texts)}
        if rule not in rules:
            raise AssertionError(f"{rule}: mutation of {name} ({new!r}) not caught; got {sorted(rules)}")
        covered.add(rule)
    missing = {f"W{n}" for n in range(1, 12)} - covered
    if missing:
        raise AssertionError(f"rules without a failing snippet: {sorted(missing)}")
    print(f"check-workflows: self-test passed ({len(MUTATIONS)} failing snippets, W1-W11)")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test()
    files = sorted(list(WORKFLOWS.glob("*.yml")) + list(WORKFLOWS.glob("*.yaml")))
    texts = {path.name: path.read_text(encoding="utf-8") for path in files}
    seed = REPO / SEED
    if seed.is_file():
        texts[SEED_NAME] = seed.read_text(encoding="utf-8")
    lock = REPO / LOCK_NAME
    if lock.is_file():
        texts[LOCK_NAME] = lock.read_text(encoding="utf-8")
    problems = check_all(texts)
    for rule, name, message in problems:
        print(f"check-workflows: {rule} {name}: {message}", file=sys.stderr)
    if problems:
        return 1
    print(f"check-workflows: {len(files)} workflows pass W1-W11")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
