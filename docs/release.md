# Release validation

The `Validate zi18n release candidate` workflow in `.github/workflows/validate.yml` accepts a full commit SHA through manual dispatch or a reusable call. The native matrix covers Linux, macOS, Windows, and Rust 1.85.0 on Linux. Additional jobs cover the two no-std targets, formatting, Clippy, rustdoc, and the extracted package.

Run the package check from a clean source root. Python 3.11 or later is required. The output directory must be outside the source tree.

```sh
python3 scripts/verify_package.py \
  --candidate "$(git rev-parse HEAD)" \
  --work-dir /var/tmp/zi18n-package-evidence
```

The verifier performs a normal Cargo package build, compares archive files with the committed source, and runs a new consumer against the extracted package. Its report records the source commit, tree, archive SHA-256, resolved paths, and output for each feature mode. The consumer uses the local archive and makes no registry request.

After the coordinator approves the candidate, dispatch the workflow from `main` with that exact SHA. The successful manual run can be checked by passing its ID and candidate to the local publication script.

```sh
python3 scripts/publish_local.py \
  --candidate "$candidate" --run-id "$run_id" \
  --work-dir /var/tmp/zi18n-package-evidence
```

The script verifies the successful run, the current public `main`, the clean local package, and initial crates.io name availability. Adding `--publish` uses the existing native Cargo login to upload to crates.io. Tag and GitHub Release decisions are handled separately by the coordinator.
