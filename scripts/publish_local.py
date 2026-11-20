#!/usr/bin/env python3
"""Verify or publish the initial zi18n crate with an approved GitHub Actions run."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

REPOSITORY = "LIghtJUNction/zi18n"
REMOTE = f"https://github.com/{REPOSITORY}.git"
WORKFLOW = ".github/workflows/validate.yml"
REGISTRY_URLS = (
    "https://crates.io/api/v1/crates/zi18n",
    "https://index.crates.io/zi/18/zi18n",
)


def output(*argv):
    return subprocess.check_output(argv, text=True).strip()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_run(run_id, candidate):
    run = json.loads(output(
        "gh", "api", "--hostname", "github.com", f"repos/{REPOSITORY}/actions/runs/{run_id}",
    ))
    require(
        run["id"] == run_id
        and run["repository"]["full_name"] == REPOSITORY
        and run["head_repository"]["full_name"] == REPOSITORY
        and run["path"].split("@", 1)[0] == WORKFLOW
        and run["event"] == "workflow_dispatch"
        and run["head_branch"] == "main"
        and run["head_sha"] == candidate
        and run["display_title"] == f"Validate zi18n {candidate}"
        and run["status"] == "completed"
        and run["conclusion"] == "success",
        "A successful manual validation run on main for the exact candidate is required",
    )
    print(f"Verified {run['html_url']}", flush=True)


def verify_clean_main(candidate):
    require(output("git", "rev-parse", "HEAD") == candidate, "Unexpected local source commit")
    require(not output("git", "status", "--porcelain"), "Source checkout must be clean")
    upstream = output("git", "ls-remote", "--exit-code", REMOTE, "refs/heads/main")
    require(upstream == f"{candidate}\trefs/heads/main", "Public main differs from this candidate")


def verify_unclaimed():
    for url in REGISTRY_URLS:
        code = output(
            "curl", "--silent", "--show-error", "--retry", "2", "--max-time", "15",
            "--noproxy", "crates.io,index.crates.io", "--user-agent", f"zi18n-release ({REMOTE})",
            "--output", os.devnull, "--write-out", "%{response_code}", url,
        )
        require(code == "404", f"The name is present or availability is unverified (HTTP {code} at {url})")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate", required=True, help="Exact full commit")
    parser.add_argument("--run-id", required=True, type=int, help="Successful validate.yml run on main")
    parser.add_argument("--work-dir", required=True, type=Path, help="Parent for package evidence")
    parser.add_argument("--publish", action="store_true", help="Upload using native Cargo credentials")
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.candidate), "A full lowercase commit SHA is required")
    require(args.run_id > 0, "Run ID must be positive")

    root = Path(output("git", "rev-parse", "--show-toplevel")).resolve()
    require(Path.cwd().resolve() == root, "Run from the source repository root")
    verify_clean_main(args.candidate)
    verify_run(args.run_id, args.candidate)
    os.environ.setdefault("CARGO_TARGET_DIR", str(args.work_dir.resolve() / "target"))
    os.environ["CARGO_BUILD_JOBS"] = "1"
    subprocess.run(
        [sys.executable, str(root / "scripts" / "verify_package.py"),
         "--candidate", args.candidate, "--work-dir", str(args.work_dir)],
        check=True,
    )
    verify_clean_main(args.candidate)
    verify_unclaimed()
    if args.publish:
        subprocess.run(
            ["cargo", "publish", "--locked", "--registry", "crates-io", "--package", "zi18n"],
            check=True,
        )
    else:
        print("Publication checks passed. Archive and consumer evidence recorded.", flush=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
