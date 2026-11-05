"""Check load/reattach coat selection without changing the real terminal font."""

import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import tempfile
import termios
import threading
import time

PLUGIN = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix="runcat-coat-") as temporary:
        root = Path(temporary)
        socket = str(root / "tmux.sock")
        plugin = root / "plugin with ' spaces"
        (plugin / "fonts").mkdir(parents=True)
        selector = plugin / "fonts/select-coat.sh"
        selector.write_text('''#!/usr/bin/env bash
printf '%s\\n' "$1" >> "$0.calls"
case "$1" in
  random) printf 'tuxedo\\n' ;;
  black) printf 'black\\n' ;;
  *) printf 'missing font\\n' >&2; exit 1 ;;
esac
''')
        calls = selector.with_name(selector.name + ".calls")
        environment = dict(os.environ, TERM="xterm-256color")
        environment.pop("TMUX", None)
        command = ["tmux", "-S", socket]
        master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 100, 0, 0))
        stop = threading.Event()
        client = None

        def drain():
            while not stop.is_set():
                if select.select([master], [], [], .05)[0]:
                    try:
                        os.read(master, 65536)
                    except OSError:
                        return

        reader = threading.Thread(target=drain, daemon=True)
        reader.start()

        def tmux(*args):
            return subprocess.check_output(command + list(args), env=environment, text=True).strip()

        def load():
            return subprocess.run([str(PLUGIN / "target/release/tmux-runcat"), "load", str(plugin),
                                   socket, pid], env=environment, capture_output=True, text=True, check=True)

        def wait_for(predicate, message):
            deadline = time.monotonic() + 4
            while time.monotonic() < deadline:
                if predicate():
                    return
                time.sleep(.03)
            raise AssertionError(message + "\n" + tmux("show-hooks", "-g"))

        def attach():
            nonlocal client
            tmux("set-option", "-g", "@other-attach-hook", "no")
            client = subprocess.Popen(command + ["attach-session", "-t", "coat"],
                                      stdin=slave, stdout=slave, stderr=slave,
                                      env=environment, start_new_session=True)
            wait_for(lambda: tmux("show-option", "-gv", "@other-attach-hook") == "yes",
                     "attach hooks did not run")

        def detach():
            name = tmux("list-clients", "-F", "#{client_name}")
            tmux("detach-client", "-t", name)
            client.wait(timeout=4)

        try:
            tmux("-f", "/dev/null", "new-session", "-d", "-s", "coat", "sleep 120")
            tmux("set-hook", "-g", "client-attached[123]", "set-option -g @other-attach-hook yes")
            pid = tmux("display-message", "-p", "#{pid}")
            load()
            assert tmux("show-option", "-gv", "@runcat-coat") == "font"
            assert not calls.exists(), "default mode changed the terminal font"
            generation = tmux("show-option", "-gv", "@runcat-generation")
            tmux("set-option", "-g", "@runcat-coat", "random")
            load()
            assert calls.read_text().splitlines() == ["random"]
            assert tmux("show-option", "-gv", "@runcat-coat-picked") == "tuxedo"
            assert tmux("show-option", "-gv", "@runcat-generation") != generation
            time.sleep(.25)
            tmux("set-option", "-g", "@runcat-keys", "100")
            time.sleep(.25)
            assert calls.read_text().splitlines() == ["random"], "animation rerolled the coat"
            load()
            assert calls.read_text().splitlines() == ["random", "random"]
            hooks = tmux("show-hooks", "-g", "client-attached")
            assert sum("/target/release/tmux-runcat" in line for line in hooks.splitlines()) == 1, (
                "reload duplicated the coat hook")
            assert "client-attached[123]" in hooks, "another plugin's hook was removed"
            generation = tmux("show-option", "-gv", "@runcat-generation")
            for expected in (3, 4):
                attach()
                wait_for(lambda: len(calls.read_text().splitlines()) == expected,
                         "reattach did not select a coat")
                time.sleep(.1)
                assert len(calls.read_text().splitlines()) == expected, "attach selected twice"
                assert tmux("show-option", "-gv", "@runcat-generation") == generation, (
                    "attach restarted the animation worker")
                detach()
            tmux("set-option", "-g", "@runcat-coat", "font")
            attach()
            time.sleep(.2)
            assert len(calls.read_text().splitlines()) == 4, "font mode rerolled on attach"
            detach()
            tmux("set-option", "-g", "@runcat-coat", "black")
            load()
            assert tmux("show-option", "-gv", "@runcat-coat-picked") == "black"
            tmux("set-option", "-g", "@runcat-coat", "missing")
            result = load()
            assert "keeping the current font" in result.stderr
            assert tmux("show-option", "-gv", "@runcat-coat-picked") == "black"
            time.sleep(.2)
            assert "watch" in tmux("show-messages", "-J"), "selection failure stopped the worker"
            print("PASS: load/reload, repeated real reattach, one coat per event, unchanged worker, preserved hooks, disabled mode, quoted paths and failure recovery")
        finally:
            subprocess.run(command + ["kill-server"], env=environment, capture_output=True)
            if client is not None:
                client.wait(timeout=4)
            stop.set()
            reader.join(timeout=1)
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
