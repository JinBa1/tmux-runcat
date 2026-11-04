"""Check real tmux layout, resizing, patrol, spinning, and theme reloads."""

import fcntl
import os
from pathlib import Path
import pty
import re
import select
import shlex
import signal
import struct
import subprocess
import tempfile
import termios
import threading
import time

PLUGIN = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix="runcat-roaming-") as directory:
        socket = str(Path(directory) / "tmux.sock")
        command = ["tmux", "-S", socket]
        environment = dict(os.environ, TERM="xterm-256color")
        environment.pop("TMUX", None)
        master, slave = pty.openpty()
        client = None
        second_client = None
        second_master = second_slave = None
        output = []
        stop = threading.Event()

        def resize(width):
            fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, width, 0, 0))

        def run(*args):
            return subprocess.check_output(command + list(args), env=environment, text=True).rstrip("\r\n")

        def option(name):
            return run("show-option", "-v", "-t", "test", name)

        def wait_for(predicate, message, timeout=4):
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                if predicate():
                    return
                time.sleep(0.03)
            raise AssertionError(message)

        def drain(fd=master):
            while not stop.is_set():
                if select.select([fd], [], [], 0.05)[0]:
                    try:
                        data = os.read(fd, 65536)
                        if not data: return
                        if fd == master: output.append(data)
                    except OSError:
                        return

        try:
            resize(100)
            run("-f", "/dev/null", "new-session", "-d", "-s", "test")
            for name, value in [("status-left", "LEFT"), ("status-right", "RIGHT"),
                                ("window-status-current-format", "[W]"),
                                ("window-status-format", "[W]"),
                                ("@runcat-skin", "pet-cats"), ("@runcat-size", "large"),
                                ("@runcat-roam", "on"), ("@runcat-spin-chance", "0"),
                                ("@runcat-wall-chance", "0"),
                                ("@runcat-color", "cyan")]:
                run("set-option", "-g", name, value)
            baseline = run("show-options", "-gqv", "status-format[0]")
            client = subprocess.Popen(command + ["attach-session", "-t", "test"],
                                      stdin=slave, stdout=slave, stderr=slave,
                                      env=environment, start_new_session=True)
            threading.Thread(target=drain, daemon=True).start()
            wait_for(lambda: bool(run("list-clients", "-F", "#{client_name}")), "client missing")
            client_name = run("list-clients", "-F", "#{client_name}")
            pid = run("display-message", "-p", "#{pid}")
            subprocess.run([str(PLUGIN / "target/release/tmux-runcat"), "load", str(PLUGIN), socket, pid],
                           env=environment, check=True)
            wait_for(lambda: "@runcat-track" in run("show-options", "-gqv", "status-format[0]"),
                     "roaming track was not installed")
            wait_for(lambda: bool(run("display-message", "-c", client_name, "-p", "#{E:@runcat-track}")), "pose was not published")

            def styled_lane(name=None):
                return run("display-message", "-c", name or client_name, "-t", "test", "-p", "#{E:@runcat-track}")

            def lane(name=None):
                return re.sub(r"#\[[^]]*\]", "", styled_lane(name))

            def position(name=None):
                text = lane(name)
                return len(text) - len(text.lstrip()) - 1

            wait_for(lambda: "#[fg=cyan]" in styled_lane(), "cat did not use cyan: " + repr(styled_lane()))
            for color in ("magenta", "#12abcd", "colour123", "cyan"):
                run("set-option", "-g", "@runcat-color", color)
                wait_for(lambda: f"#[fg={color}]" in styled_lane(), "live color did not apply")
                assert "#[default]" in styled_lane(), "cat color was not reset"
            run("set-option", "-g", "@runcat-color", "inherit")
            wait_for(lambda: "#[fg=" not in styled_lane(), "inherit kept an explicit color")
            run("set-option", "-g", "@runcat-color", "cyan")

            # Width is measured from rendered text, rather than the maximum
            # status-left/right length settings. Cat glyphs occupy one cell each.
            for width in (100, 60, 14, 150):
                resize(width)
                os.kill(client.pid, signal.SIGWINCH)
                wait_for(lambda: run("display-message", "-c", client_name, "-p", "#{client_width}") == str(width),
                         "resize was not seen")
                expected = width - len("LEFT[W]RIGHT")
                wait_for(lambda: len(lane()) == (expected if expected >= 5 else 0), "track did not resize")
                rendered = lane()
                if width < len("LEFT[W]RIGHT") + 5:
                    assert rendered == "", repr(rendered)
                    continue
                assert len(rendered) == width - len("LEFT[W]RIGHT"), (width, len(rendered), repr(rendered))
                assert len(rendered.strip()) == 3, repr(rendered)

            run("new-window", "-d", "-t", "test", "-n", "extra")
            wait_for(lambda: len(lane()) == 150 - len("LEFT[W] [W]RIGHT"), "window name/list width did not apply")
            run("kill-window", "-t", "test:extra")

            # Two clients share the animation but have independent paths.
            # Before either reaches an edge, the same stride covers the same
            # number of cells despite the different available widths.
            second_master, second_slave = pty.openpty()
            fcntl.ioctl(second_slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 100, 0, 0))
            second_client = subprocess.Popen(command + ["attach-session", "-t", "test"],
                                            stdin=second_slave, stdout=second_slave, stderr=second_slave,
                                            env=environment, start_new_session=True)
            threading.Thread(target=drain, args=(second_master,), daemon=True).start()
            wait_for(lambda: len(run("list-clients", "-F", "#{client_name}").splitlines()) == 2,
                     "second client did not attach")
            second_name = next(name for name in run("list-clients", "-F", "#{client_name}").splitlines()
                               if name != client_name)
            wait_for(lambda: len(lane(second_name)) == 100 - len("LEFT[W]RIGHT"), "second client lane missing")
            wait_for(lambda: len(lane()) == 150 - len("LEFT[W]RIGHT"), "first client lane changed")
            deadline = time.monotonic() + 2.5
            while time.monotonic() < deadline:
                run("set-option", "-F", "-t", "test", "@runcat-keys", "#{e|+:#{@runcat-keys},1}")
                time.sleep(0.05)
                assert abs(position() - position(second_name)) <= 1, "terminal width changed stride length"
            assert position() > 5, "cat did not travel"
            run("detach-client", "-t", second_name)
            second_client.wait(timeout=3)
            second_pid = second_client.pid
            wait_for(lambda: not run("show-option", "-qv", "-t", "test", f"@runcat-client-{second_pid}"),
                     "detached client lane was not removed")

            # A status command needs time to finish while the cat animates.
            # Forced refreshes used to restart these jobs on every frame.
            left_ready = Path(directory) / "left-ready"
            right_ready = Path(directory) / "right-ready"
            run("set-option", "-g", "status-left",
                f"#(sleep 0.25; touch {shlex.quote(str(left_ready))}; printf LEFT)")
            run("set-option", "-g", "status-right",
                f"#(sleep 0.25; touch {shlex.quote(str(right_ready))}; printf RIGHT)")
            run("set-option", "-F", "-t", "test", "@runcat-keys", "#{e|+:#{@runcat-keys},80}")
            wait_for(lambda: left_ready.exists() and right_ready.exists(),
                     "status commands were restarted by the animation", timeout=2)

            # Synthetic activity updates only the counter; it does not send
            # test characters into a user's shell.
            # A narrow lane gives a quick turn without requiring artificially
            # width-dependent speed. Both directions retain the configured color.
            resize(30)
            os.kill(client.pid, signal.SIGWINCH)
            wait_for(lambda: len(lane()) == 30 - len("LEFT[W]RIGHT"), "narrow resize missing")
            observed, positions = set(), []
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                run("set-option", "-F", "-t", "test", "@runcat-keys", "#{e|+:#{@runcat-keys},3}")
                time.sleep(0.06)
                glyphs = lane().strip()
                mirrored = bool(glyphs) and all(0x101000 <= ord(ch) < 0x10135C for ch in glyphs)
                observed.add("left" if mirrored else "right")
                positions.append(position())
                assert "#[fg=cyan]" in styled_lane(), "direction changed color"
                if observed == {"left", "right"} and len(positions) > 20:
                    break
            assert observed == {"left", "right"}, observed
            assert max(positions) - min(positions) > 5, positions

            run("set-option", "-g", "@runcat-spin-chance", "100")
            states = set()
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                run("set-option", "-F", "-t", "test", "@runcat-keys", "#{e|+:#{@runcat-keys},8}")
                states.add(option("@runcat-state"))
                if "spin" in states:
                    pose = option("@runcat-frame")
                    assert all(0x100000 <= ord(ch) < 0x1000A0 for ch in pose)
                    break
                time.sleep(0.06)
            assert "spin" in states, states
            until = time.monotonic() + 1.7
            while time.monotonic() < until:
                run("set-option", "-F", "-t", "test", "@runcat-keys", "#{e|+:#{@runcat-keys},8}")
                time.sleep(0.06)
                assert option("@runcat-state") == "spin", "spin stopped while typing continued"
                assert "#[fg=cyan]" in styled_lane(), "spin changed color"
            wait_for(lambda: option("@runcat-state") != "spin", "spin kept going after typing stopped", timeout=1)
            wait_for(lambda: option("@runcat-state") == "sleep", "spinning delayed sleep")
            sleeping_position = position()
            time.sleep(0.25)
            assert position() == sleeping_position, "sleeping cat moved"
            assert "#[fg=cyan]" in styled_lane(), "sleep changed color"

            foreground = None
            cat_colors = []
            terminal_text = b"".join(output).decode("utf-8", errors="ignore")
            tokens = re.finditer(r"\x1b\[[0-9;:]*m|[\ue400-\ue517\uf700-\uf8a3\U00100000-\U0010135b]", terminal_text)
            for token in tokens:
                value = token.group()
                if not value.startswith("\x1b"):
                    cat_colors.append(foreground)
                    continue
                codes = [int(code or 0) for code in value[2:-1].replace(":", ";").split(";")]
                index = 0
                while index < len(codes):
                    code = codes[index]
                    if code in (0, 39): foreground = None
                    elif 30 <= code <= 37: foreground = code - 30
                    elif 90 <= code <= 97: foreground = code - 90 + 8
                    elif code in (38, 48) and index + 2 < len(codes):
                        count = 2 if codes[index + 1] == 5 else 4
                        if code == 38:
                            foreground = codes[index + 2] if count == 2 else tuple(codes[index + 2:index + 5])
                        index += count
                    index += 1
            assert len(cat_colors) > 30, "no terminal cat frames captured"
            assert set(cat_colors[-30:]) == {6}, ("rendered cat did not stay cyan", cat_colors[-30:])

            # Debug runs through the three local edge reactions even with
            # wall chance zero, and each pose retains the configured color.
            run("set-option", "-g", "@runcat-spin-chance", "0")
            run("set-option", "-g", "@runcat-wall-debug", "on")
            for action in ("head-bump", "wall-scratch", "parkour"):
                wait_for(lambda: option("@runcat-state") == action,
                         f"debug did not play {action}", timeout=10)
                glyphs = lane().strip()
                assert glyphs and all(0x102000 <= ord(ch) < 0x10326C for ch in glyphs), (
                    action, repr(glyphs))
                assert "#[fg=cyan]" in styled_lane(), "edge reaction changed color"
                wait_for(lambda: option("@runcat-state") != action,
                         f"debug action {action} did not finish", timeout=5)
            run("set-option", "-g", "@runcat-wall-debug", "off")
            wait_for(lambda: option("@runcat-state") == "sleep",
                     "cat did not rest after debug mode")

            # Simulate a theme resetting the status format. The worker must
            # restore its overlay and remove it cleanly when disabled.
            run("set-option", "-g", "status-format[0]", baseline)
            wait_for(lambda: "@runcat-track" in run("show-options", "-gqv", "status-format[0]"),
                     "theme reload removed the track")
            run("set-option", "-g", "@runcat-roam", "off")
            wait_for(lambda: run("show-options", "-gqv", "status-format[0]") == baseline,
                     "disabling roaming did not restore the status format")
            print("PASS: per-client stride/width, resizing, colors, wall reactions, debug cycle, direction, spinning, sleep, status commands and theme reload")
        finally:
            stop.set()
            subprocess.run(command + ["kill-server"], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if client is not None:
                client.wait(timeout=3)
            if second_client is not None:
                second_client.wait(timeout=3)
            os.close(master)
            os.close(slave)
            # The reader exits on PTY hangup before descriptors are closed.
            if second_master is not None: os.close(second_master)
            if second_slave is not None: os.close(second_slave)


if __name__ == "__main__":
    main()
