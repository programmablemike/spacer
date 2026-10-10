"""Create a checksummed native archive and smoke-test its extracted binary."""
import hashlib
import pathlib
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile

binary = pathlib.Path(sys.argv[1]).resolve()
target = sys.argv[2]
version = re.search(r'^version = "([^"]+)"', pathlib.Path("Cargo.toml").read_text(), re.M)[1]
output = pathlib.Path("dist")
output.mkdir(exist_ok=True)
name = f"spacer-{version}-{target}"
with tempfile.TemporaryDirectory() as directory:
    staging = pathlib.Path(directory) / name
    staging.mkdir()
    shutil.copy2(binary, staging / binary.name)
    for filename in ("LICENSE", "README.md", "RELEASE_NOTES.md", "llms.txt", "logo.svg", "logo-icon.svg"):
        shutil.copy2(filename, staging / filename)
    shutil.copytree("docs", staging / "docs")
    if target.endswith("msvc"):
        archive = output / (name + ".zip")
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as bundle:
            for file in staging.rglob("*"):
                if file.is_file():
                    bundle.write(file, file.relative_to(staging.parent))
    else:
        archive = output / (name + ".tar.gz")
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(staging, arcname=name)
    checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(f"{checksum}  {archive.name}\n")
    extracted = pathlib.Path(directory) / "extracted"
    shutil.unpack_archive(archive, extracted)
    subprocess.run([sys.executable, "tests/smoke.py", str(extracted / name / binary.name)], check=True)
    print(archive)
