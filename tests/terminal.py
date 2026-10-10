"""POSIX pseudo-terminal checks for managed shells and TUI restoration.

Run: python3 tests/terminal.py /absolute/path/to/spacer
"""
import os
import pathlib
import pty
import select
import shutil
import subprocess
import sys
import tempfile
import termios
import time

binary = str(pathlib.Path(sys.argv[1]).resolve())
with tempfile.TemporaryDirectory(prefix="spacer terminal '") as directory:
    root = pathlib.Path(directory)
    env = dict(os.environ)
    for key in ("SPACER_SPACE", "SPACER_PROJECT", "SPACER_SHELL_ACTIVE", "ZDOTDIR"):
        env.pop(key, None)
    # Isolate child startup files from real user configuration.
    env.update(SPACER_ROOT=directory, HOME=directory, TERM="xterm-256color",
               PATH=str(pathlib.Path(binary).parent) + os.pathsep + env.get("PATH", ""))
    subprocess.run([binary, "workspace", "init"], env=env, check=True, capture_output=True)
    (root / ".bashrc").write_text("export SPACER_RC_TEST=loaded\n")
    (root / ".zshrc").write_text("export SPACER_RC_TEST=loaded\n")

    def terminal_run(args, commands, expected=0):
        master, slave = pty.openpty()
        before = termios.tcgetattr(slave)
        process = subprocess.Popen([binary, *args], stdin=slave, stdout=slave,
                                   stderr=slave, env=env)
        output = bytearray()
        try:
            # Wait for startup before supplying terminal input.
            deadline = time.monotonic() + 20
            sent = False
            started = time.monotonic()
            while process.poll() is None and time.monotonic() < deadline:
                if select.select([master], [], [], 0.1)[0]:
                    output.extend(os.read(master, 65536))
                if not sent and time.monotonic() - started > 1:
                    os.write(master, commands.encode())
                    sent = True
            if process.poll() is None:
                process.kill()
                raise AssertionError(f"terminal process timed out: {output!r}")
            assert process.wait() == expected, output.decode(errors="replace")
            after = termios.tcgetattr(slave)
            # PENDIN is a kernel retype-pending flag, not a persistent terminal mode.
            after[3] &= ~getattr(termios, "PENDIN", 0)
            before[3] &= ~getattr(termios, "PENDIN", 0)
            assert after == before, f"terminal attributes were not restored: {args}"

        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)

    for shell in ("bash", "zsh", "sh"):
        path = shutil.which(shell)
        if not path:
            continue
        rc = '[ "$SPACER_RC_TEST" = loaded ] || exit 31; ' if shell != "sh" else ""
        terminal_run(["shell", "--shell", path],
                     rc + ("type _spacer >/dev/null 2>&1 || exit 34; " if shell != "sh" else "") + '[ "$SPACER_SHELL_ACTIVE" = 1 ] || exit 32; '
                     'command spacer shell >/dev/null 2>&1 && exit 33; exit 23\n', 23)
    terminal_run([], "q")
    terminal_run([], "\x03")
    (root / ".spacer/config.json").write_text("broken")
    terminal_run([], "", 1)
    print("Managed shell startup/exit, nested refusal, startup files, and TUI terminal restoration passed")
