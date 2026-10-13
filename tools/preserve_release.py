"""Preserve a plain release executable and the exact local recipe that produced it."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import time

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def sources():
    paths = [ROOT / name for name in ("Cargo.toml", "Cargo.lock")]
    paths.extend(ROOT / name for name in ("build.rs", "rust-toolchain", "rust-toolchain.toml")
                 if (ROOT / name).is_file())
    for directory in ("src", "vendor", "assets"):
        paths.extend(path for path in (ROOT / directory).rglob("*")
                     if path.is_file() and not {"target", ".git"}.intersection(path.relative_to(ROOT).parts))
    return {str(path.relative_to(ROOT)): digest(path) for path in sorted(paths)}


def command_output(command):
    return subprocess.run(command, cwd=ROOT, check=True, capture_output=True, text=True).stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--features", default="", help="Explicit opt-in diagnostic build features")
    args = parser.parse_args()
    output = args.output.resolve()
    target = args.target_dir.resolve()
    if target.exists():
        raise ValueError("Use a new target directory, separate from tests and previous builds")
    output.mkdir(parents=True, exist_ok=False)
    recipe = ["cargo", "build", "--release", "--locked", "--offline", "--bin", "native-diff",
              "--target-dir", str(target)]
    feature_args = ["--features", args.features] if args.features else []
    recipe += feature_args
    rustc = command_output(["rustc", "-Vv"])
    settings = ("PATH", "RUSTUP_TOOLCHAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
                "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "SDKROOT",
                "MACOSX_DEPLOYMENT_TARGET", "CC", "CXX", "AR", "CFLAGS", "CXXFLAGS",
                "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTFLAGS",
                "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER",
                "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS")
    config_paths = [parent / ".cargo" / name for parent in (ROOT, *ROOT.parents)
                    for name in ("config", "config.toml")]
    cargo_home = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")))
    config_paths.extend(cargo_home / name for name in ("config", "config.toml"))
    report = {
        "status": "building", "started_ns": time.time_ns(), "root": str(ROOT),
        "command": recipe, "rustc": rustc, "cargo": command_output(["cargo", "-V"]),
        "toolchain": command_output(["rustup", "show", "active-toolchain"]),
        "sdk": command_output(["xcrun", "--show-sdk-path"]),
        "sdk_version": command_output(["xcrun", "--show-sdk-version"]),
        "linker": command_output(["xcrun", "--find", "ld"]),
        "clang": command_output(["xcrun", "clang", "--version"]),
        "environment": {key: os.environ.get(key) for key in settings},
        "cargo_config_hashes": {str(path): digest(path) for path in config_paths if path.is_file()},
        "source_sha256": sources(), "driver_sha256": digest(Path(__file__)),
    }
    try:
        with tarfile.open(output / "build-source.tar.gz", "x:gz") as archive:
            for name in report["source_sha256"]:
                archive.add(ROOT / name, arcname=name, recursive=False)
        with (output / "build.log").open("x") as log:
            subprocess.run(recipe, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        if sources() != report["source_sha256"] or command_output(["rustc", "-Vv"]) != rustc:
            raise ValueError("Build source or compiler changed during preservation")
        if {str(path): digest(path) for path in config_paths if path.is_file()} != report["cargo_config_hashes"]:
            raise ValueError("Cargo configuration changed during preservation")
        binary = output / "native-diff"
        with (target / "release/native-diff").open("rb") as source, binary.open("xb") as destination:
            shutil.copyfileobj(source, destination)
        binary.chmod(0o555)
        report["binary_sha256"] = digest(binary)
        report["fingerprints"] = {
            str(path.relative_to(target / "release/.fingerprint")): json.loads(path.read_text())
            for path in sorted((target / "release/.fingerprint").glob("*/*.json"))
        }
        report["feature_tree"] = command_output(
            ["cargo", "tree", "--locked", "--offline", "-e", "normal,build,features", *feature_args])
        report["status"] = "complete"
        print(f"Preserved {binary}: {report['binary_sha256']}", flush=True)
    except (Exception, KeyboardInterrupt) as error:
        report.update(status="failed", error=str(error))
        raise
    finally:
        report["finished_ns"] = time.time_ns()
        with (output / "build.json").open("x") as stream:
            json.dump(report, stream, indent=2)
            stream.write("\n")


if __name__ == "__main__":
    main()
