"""Execute the tutorial's noninteractive shell blocks against an installed binary."""
import os
import pathlib
import re
import shlex
import subprocess
import sys
import tempfile

binary = pathlib.Path(sys.argv[1]).resolve()
text = pathlib.Path("docs/tutorials/first-workspace.md").read_text()
blocks = re.findall(r"```sh\n(.*?)```", text, re.S)
assert blocks, "tutorial must contain shell examples"
# TUI interaction is covered separately by terminal.py.
commands = "\n".join(block for block in blocks if block.strip() != "spacer")
with tempfile.TemporaryDirectory(prefix="spacer tutorial ") as directory:
    env = dict(os.environ)
    for key in list(env):
        if key.startswith("SPACER_") or key.startswith("GIT_"):
            del env[key]
    env.update(PATH=str(binary.parent) + os.pathsep + env.get("PATH", ""),
               HOME=directory, TMPDIR=directory, GIT_CONFIG_NOSYSTEM="1",
               GIT_CONFIG_GLOBAL=os.devnull, GIT_AUTHOR_NAME="Test",
               GIT_AUTHOR_EMAIL="test@example.invalid", GIT_COMMITTER_NAME="Test",
               GIT_COMMITTER_EMAIL="test@example.invalid")
    subprocess.run(["bash", "--noprofile", "--norc", "-e", "-c", commands],
                   cwd=directory, env=env, check=True)
    readme = pathlib.Path("README.md").read_text()
    sections = re.split(r"^## ", readme, flags=re.M)
    selected = [section for section in sections if section.startswith(
        ("Quickstart\n", "Navigate from your shell\n", "Work on a change\n"))]
    assert len(selected) == 3, "README walkthrough sections must be present"
    commands = "\n".join(block for section in selected
                         for block in re.findall(r"```sh\n(.*?)```", section, re.S))
    existing = pathlib.Path(directory) / "existing repository"
    subprocess.run(["git", "init", str(existing)], env=env, check=True, capture_output=True)
    commands = commands.replace("/absolute/path/to/repo", shlex.quote(str(existing)))
    commands = 'eval "$(spacer shell integration sh)"\n' + commands
    subprocess.run(["bash", "--noprofile", "--norc", "-e", "-c", commands],
                   cwd=directory, env=env, check=True)
    print("Documented tutorial and README quickstart passed")
