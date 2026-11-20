#!/usr/bin/env python3
"""Verify a clean Cargo archive and run a separate consumer of its extracted source."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile
import tomllib

REPOSITORY = "https://github.com/LIghtJUNction/zi18n"
PACKAGE_PATHS = (
    "src", "tests", "examples", "docs/design.md", "README.md", "LICENSE",
    "Cargo.toml", "Cargo.lock",
)
SMOKE_OUTPUT = "zi18n packaged consumer: 1 file {opaque}\n"
SMOKE_SOURCE = r"""
use zi18n::{Argument, Catalog, Catalogs, Locale, Message, PluralCategory, PluralForm, format_to};

const ENGLISH: &[Message<'static>] = &[Message::plural("files", &[
    PluralForm::new(PluralCategory::One, "{count} file {label}"),
    PluralForm::new(PluralCategory::Other, "{count} files {label}"),
])];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let en = Locale::parse("en")?;
    let zh = Locale::parse("zh-Hant-HK")?;
    let catalogs = [Catalog::new(en, ENGLISH)?];
    let fallbacks = [en];
    let messages = Catalogs::new(&catalogs, &fallbacks)?;
    let matched = messages.lookup(zh, "files")?;
    assert_eq!(matched.locale(), en);
    assert!(messages.lookup(zh, "missing").is_err());
    let template = matched.select_cardinal_integer(1)?.unwrap();
    let arguments = [Argument::new("count", &1_u64), Argument::new("label", &"{opaque}")];
    let mut output = String::new();
    format_to(&mut output, template, &arguments)?;
    assert_eq!(output, "1 file {opaque}");
    #[cfg(feature = "alloc")]
    assert_eq!(zi18n::format(template, &arguments)?, output);
    print!("zi18n packaged consumer: {output}\n");
    Ok(())
}
"""


def require(condition, message):
    if not condition:
        raise ValueError(message)


def extract_crate(archive, directory, tracked, revision):
    packaged = tracked | {"Cargo.toml.orig", ".cargo_vcs_info.json"}
    seen = set()
    with tarfile.open(archive, "r:gz") as crate:
        for member in crate:
            path = PurePosixPath(member.name)
            require(path.parts[:1] == (directory.name,), f"Unexpected crate member {member.name}")
            name = "/".join(path.parts[1:])
            require(member.isfile() and name in packaged and name not in seen,
                    f"Unexpected crate member {member.name}")
            seen.add(name)
            payload = crate.extractfile(member).read()
            file = directory / name
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_bytes(payload)
    require(seen == packaged, f"Missing archive members {sorted(packaged - seen)}")
    vcs = json.loads((directory / ".cargo_vcs_info.json").read_text(encoding="utf-8"))
    require(vcs["git"]["sha1"] == revision and not vcs["git"].get("dirty", False)
            and vcs["path_in_vcs"] == "", "Archive VCS identity does not match the clean root")
    return sorted(seen)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--work-dir", required=True, type=Path, help="Parent for unique evidence")
    parser.add_argument("--candidate", help="Require this exact Git commit")
    args = parser.parse_args()
    root = Path.cwd().resolve()
    work_dir = args.work_dir.resolve()
    require(not work_dir.is_relative_to(root), "Evidence directory must be outside the source")
    commands = []

    def run(cwd, *argv, capture=False):
        completed = subprocess.run(argv, cwd=cwd, capture_output=capture, check=False)
        commands.append({"cwd": str(cwd), "argv": argv, "exit_code": completed.returncode})
        if completed.returncode:
            if capture:
                print(completed.stderr.decode("utf-8", errors="replace"), end="")
            completed.check_returncode()
        return completed.stdout

    revision = run(root, "git", "rev-parse", "HEAD", capture=True).decode().strip()
    tree = run(root, "git", "rev-parse", "HEAD^{tree}", capture=True).decode().strip()
    require(args.candidate is None or args.candidate == revision, "Unexpected candidate commit")
    git_root = run(root, "git", "rev-parse", "--show-toplevel", capture=True).decode().strip()
    require(Path(git_root).resolve() == root, "Run from the source repository root")
    require(not run(root, "git", "status", "--porcelain", capture=True), "Source must be clean")
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    package = manifest["package"]
    require(package["name"] == "zi18n" and package["repository"] == REPOSITORY,
            "Unexpected package identity")
    require(manifest["features"] == {"default": [], "alloc": [], "std": ["alloc"]},
            "Unexpected features in the original manifest")

    work_dir.mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="zi18n-package-", dir=work_dir))
    target = Path(os.environ.get("CARGO_TARGET_DIR", str(work / "target"))).resolve()
    require(not target.is_relative_to(root), "CARGO_TARGET_DIR must be outside the source")
    os.environ["CARGO_TARGET_DIR"] = str(target)
    os.environ["CARGO_BUILD_JOBS"] = "1"

    cargo_version = run(root, "cargo", "--version", capture=True).decode().strip()
    rustc_version = run(root, "rustc", "--version", capture=True).decode().strip()
    run(root, "cargo", "package", "--locked", "--offline")
    stem = "zi18n-" + package["version"]
    archive = target / "package" / f"{stem}.crate"
    archived_bytes = archive.read_bytes()
    packaged_copy = work / archive.name
    packaged_copy.write_bytes(archived_bytes)
    unpacked = work / stem
    tracked = set(run(root, "git", "ls-tree", "-r", "--name-only", "-z", revision, "--",
                      *PACKAGE_PATHS, capture=True).decode().rstrip("\0").split("\0"))
    archive_files = extract_crate(packaged_copy, unpacked, tracked, revision)
    for name in sorted(tracked):
        extracted_name = "Cargo.toml.orig" if name == "Cargo.toml" else name
        committed = run(root, "git", "show", f"{revision}:{name}", capture=True)
        require((unpacked / extracted_name).read_bytes() == committed, f"Git mismatch for {name}")

    consumer = work / "consumer"
    (consumer / "src").mkdir(parents=True)
    (consumer / "src" / "main.rs").write_text(SMOKE_SOURCE.lstrip(), encoding="utf-8")
    (consumer / "Cargo.toml").write_text(
        f"""[package]
name = "zi18n-package-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
zi18n = {{ path = "../{stem}", version = "={package['version']}", default-features = false }}

[features]
alloc = ["zi18n/alloc"]
std = ["alloc", "zi18n/std"]
""",
        encoding="utf-8",
    )
    run(consumer, "cargo", "generate-lockfile", "--offline")
    for mode, expected_features in (
        ("core", []), ("alloc", ["alloc"]), ("std", ["alloc", "std"]),
    ):
        arguments = () if mode == "core" else ("--features", mode)
        raw = run(consumer, "cargo", "metadata", "--locked", "--offline", "--format-version", "1",
                  *arguments, capture=True)
        (work / f"metadata-{mode}.json").write_bytes(raw)
        metadata = json.loads(raw)
        packages = {entry["name"]: entry for entry in metadata["packages"]}
        require(set(packages) == {"zi18n-package-consumer", "zi18n"},
                "Unexpected dependency graph for the packaged consumer")
        library = packages["zi18n"]
        require(library["source"] is None and library["version"] == package["version"]
                and Path(library["manifest_path"]).resolve() == unpacked / "Cargo.toml",
                "Cargo did not resolve the extracted package")
        require(not library["dependencies"], "The archive declares dependencies")
        require(all(Path(item["src_path"]).resolve().is_relative_to(unpacked)
                    for item in library["targets"]), "A packaged target resolves outside the archive")
        node = next(item for item in metadata["resolve"]["nodes"] if item["id"] == library["id"])
        require(not node["deps"] and sorted(node["features"]) == expected_features,
                f"Unexpected library resolution for {mode}")
        output = run(consumer, "cargo", "run", "--locked", "--offline", "--quiet", *arguments,
                     capture=True).decode("utf-8")
        require(output == SMOKE_OUTPUT, f"Unexpected consumer output for {mode}")
        (work / f"consumer-{mode}.stdout").write_text(output, encoding="utf-8")

    report = {
        "status": "passed", "source": str(root), "commit": revision, "tree": tree,
        "cargo": cargo_version, "rustc": rustc_version, "consumer": str(consumer),
        "archive": str(packaged_copy), "archive_sha256": hashlib.sha256(archived_bytes).hexdigest(),
        "archive_bytes": len(archived_bytes), "archive_files": archive_files,
        "archive_contents_match_git": True, "resolved_library_manifest": str(unpacked / "Cargo.toml"),
        "consumer_modes": ["core", "alloc", "std"], "consumer_output": SMOKE_OUTPUT, "commands": commands,
    }
    evidence = work / "verification.json"
    evidence.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {archive.name} from {revision}")
    print(f"SHA-256 {report['archive_sha256']}")
    print(f"Evidence {evidence}")


if __name__ == "__main__":
    main()
