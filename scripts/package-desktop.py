"""Copy a native Tauri installer to dist/ with a platform-neutral SHA-256 file."""
import hashlib
import json
import pathlib
import shutil
import sys

TARGET_EXTENSIONS = {
    "aarch64-apple-darwin": "dmg",
    "x86_64-apple-darwin": "dmg",
    "aarch64-unknown-linux-gnu": "deb",
    "x86_64-unknown-linux-gnu": "deb",
    "x86_64-pc-windows-msvc": "exe",
}

def main():
    target = sys.argv[1]
    extension = TARGET_EXTENSIONS[target]
    bundle_dir = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "desktop/src-tauri/target/release/bundle")
    installers = list(bundle_dir.rglob("*." + extension))
    if len(installers) != 1:
        raise SystemExit(f"expected one {extension} installer under {bundle_dir}, found {len(installers)}")
    version = json.loads(pathlib.Path("desktop/src-tauri/tauri.conf.json").read_text())["version"]
    output = pathlib.Path("dist")
    output.mkdir(exist_ok=True)
    artifact = output / f"spacer-desktop-{version}-{target}.{extension}"
    shutil.copy2(installers[0], artifact)
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    artifact.with_name(artifact.name + ".sha256").write_bytes(f"{digest}  {artifact.name}\n".encode("ascii"))
    print(artifact)

if __name__ == "__main__":
    main()
