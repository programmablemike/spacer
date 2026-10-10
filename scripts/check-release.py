"""Refuse publishing a mismatched tag/version or incomplete artifact set."""
import json
import os
import pathlib
import re

versions = []
for path in ("Cargo.toml", "spacer-core/Cargo.toml", "spacer-data/Cargo.toml", "desktop/src-tauri/Cargo.toml"):
    versions.append(re.search(r'^version = "([^"]+)"', pathlib.Path(path).read_text(), re.M)[1])
for path in ("desktop/package.json", "desktop/src-tauri/tauri.conf.json"):
    versions.append(json.loads(pathlib.Path(path).read_text())["version"])
assert len(set(versions)) == 1, versions
assert os.environ["RELEASE_TAG"] == "v" + versions[0], "tag must match all package versions"
targets = ("aarch64-apple-darwin", "x86_64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc")
for target in targets:
    extension = ".zip" if target.endswith("msvc") else ".tar.gz"
    artifact = pathlib.Path("dist") / f"spacer-{versions[0]}-{target}{extension}"
    assert artifact.is_file() and artifact.with_name(artifact.name + ".sha256").is_file(), target

    desktop_extension = ".exe" if target.endswith("msvc") else ".dmg" if "apple" in target else ".deb"
    desktop = pathlib.Path("dist") / f"spacer-desktop-{versions[0]}-{target}{desktop_extension}"
    assert desktop.is_file() and desktop.with_name(desktop.name + ".sha256").is_file(), target
