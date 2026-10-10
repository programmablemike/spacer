"""Exercise an installed or extracted binary in a fresh isolated workspace."""
import os
import pathlib
import subprocess
import sys
import tempfile

binary = str(pathlib.Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory(prefix="spacer smoke ") as directory:
    root = pathlib.Path(directory)
    env = dict(os.environ)
    for key in ("SPACER_SPACE", "SPACER_PROJECT", "SPACER_SHELL_ACTIVE", "GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_INDEX_FILE"):
        env.pop(key, None)
    env.update(SPACER_ROOT=directory, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)

    def run(*args, cwd=root, success=True):
        result = subprocess.run(args, cwd=cwd, env=env, capture_output=True, text=True)
        assert (result.returncode == 0) == success, (args, result.stdout, result.stderr)
        return result.stdout.strip()

    def spacer(*args, **kwargs):
        return run(binary, *args, **kwargs)

    def git(*args, cwd):
        return run("git", "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                   "-c", "commit.gpgsign=false", *args, cwd=cwd)

    spacer("workspace", "init")
    spacer("space", "create", "team")
    spacer("project", "create", "repo", "--space", "team")
    spacer("project", "use", "repo", "--space", "team")
    project = pathlib.Path(spacer("path", "project"))
    git("commit", "--allow-empty", "-m", "Initial commit", cwd=project)
    spacer("change", "start", "feat/smoke", "--project", "repo")
    tree = pathlib.Path(spacer("path", "change", "feat/smoke"))
    assert tree.is_dir() and not tree.is_relative_to(project)
    (tree / "keep.txt").write_text("preserve")
    spacer("change", "finish", "feat/smoke", "--project", "repo", success=False)
    assert (tree / "keep.txt").read_text() == "preserve"
    (tree / "keep.txt").unlink()
    spacer("change", "finish", "feat/smoke", "--project", "repo")
    assert not tree.exists()
    git("show-ref", "--verify", "refs/heads/feat/smoke", cwd=project)
    spacer("project", "unregister", "repo")
    assert (project / ".git").exists()
    spacer("project", "register", "repo", "--path", str(project))
    spacer("project", "delete", "repo")
    spacer("space", "unregister", "team")
    print("Installed lifecycle passed:", spacer("--version"))
