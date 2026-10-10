"""Refuse publishing a mismatched tag/version or incomplete artifact set."""
import os
import pathlib
import re

versions = []
for path in ("Cargo.toml", "spacer-core/Cargo.toml", "spacer-data/Cargo.toml"):
    versions.append(re.search(r'^version = "([^"]+)"', pathlib.Path(path).read_text(), re.M)[1])
assert len(set(versions)) == 1, versions
assert os.environ["RELEASE_TAG"] == "v" + versions[0], "tag must match all package versions"
targets = ("aarch64-apple-darwin", "x86_64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc")
for target in targets:
    extension = ".zip" if target.endswith("msvc") else ".tar.gz"
    artifact = pathlib.Path("dist") / f"spacer-{versions[0]}-{target}{extension}"
    assert artifact.is_file() and artifact.with_name(artifact.name + ".sha256").is_file(), target
