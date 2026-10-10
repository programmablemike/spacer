"""Run a feature-gated native webview lifecycle in an isolated workspace."""
import argparse
import json
import os
import pathlib
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("binary", type=pathlib.Path)
parser.add_argument("--empty-path", action="store_true", help="verify the package supplies runtime tools")
args = parser.parse_args()
binary = args.binary.resolve()
with tempfile.TemporaryDirectory(prefix="spacer native ") as directory:
    root = pathlib.Path(directory)
    env = dict(os.environ)
    for key in list(env):
        if key.startswith("SPACER_") or key.startswith("GIT_"):
            del env[key]
    env.update(SPACER_DESKTOP_SMOKE_ROOT=str(root), GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
    repository = root / "existing"
    subprocess.run(["git", "init", str(repository)], env=env, check=True, capture_output=True)
    subprocess.run(["git", "-C", str(repository), "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                    "-c", "commit.gpgsign=false", "commit", "--allow-empty", "-m", "Initial"],
                   env=env, check=True, capture_output=True)
    binary_env = dict(env, PATH="") if args.empty_path else env
    result = subprocess.run([str(binary)], env=binary_env, capture_output=True, text=True, timeout=90)
    assert result.returncode == 0 and '"success":true' in result.stdout, (result.stdout, result.stderr)
    assert (repository / ".git").is_dir()
    registry = json.loads((root / ".spacer/config.json").read_text())
    assert registry["projects"] == [], registry
    assert [space["name"] for space in registry["spaces"]] == ["team"], registry
    assert not (root / ".spacer/worktrees/team/app/feat/native").exists()
    subprocess.run(["git", "-C", str(repository), "show-ref", "--verify", "refs/heads/feat/native"],
                   env=env, check=True, capture_output=True)
    print("Native webview lifecycle passed: initialize, register, start, finish, unregister; branch and repository preserved")
