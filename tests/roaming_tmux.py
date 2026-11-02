"""Check real tmux layout, resizing, patrol, spinning, and theme reloads."""

import fcntl
import os
from pathlib import Path
import pty
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

        def drain():
            while not stop.is_set():
                if select.select([master], [], [], 0.05)[0]:
                    try:
                        os.read(master, 65536)
                    except OSError:
                        return

        try:
            resize(100)
            run("-f", "/dev/null", "new-session", "-d", "-s", "test")
            for name, value in [("status-left", "LEFT"), ("status-right", "RIGHT"),
                                ("window-status-current-format", "[W]"),
                                ("window-status-format", "[W]"),
                                ("@runcat-skin", "pet-cats"), ("@runcat-size", "large"),
                                ("@runcat-roam", "on"), ("@runcat-spin-chance", "0")]:
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
            wait_for(lambda: bool(option("@runcat-roam-frame")), "pose was not published")

            def lane():
                return run("display-message", "-c", client_name, "-t", "test", "-p", "#{E:@runcat-track}")

            # Width is measured from rendered text, rather than the maximum
            # status-left/right length settings. Cat glyphs occupy one cell each.
            for width in (100, 60, 14, 150):
                resize(width)
                os.kill(client.pid, signal.SIGWINCH)
                wait_for(lambda: run("display-message", "-c", client_name, "-p", "#{client_width}") == str(width),
                         "resize was not seen")
                rendered = lane()
                if width < len("LEFT[W]RIGHT") + 5:
                    assert rendered == "", repr(rendered)
                    continue
                assert len(rendered) == width - len("LEFT[W]RIGHT"), (width, len(rendered), repr(rendered))
                assert len(rendered.strip()) == 3, repr(rendered)

            run("new-window", "-d", "-t", "test", "-n", "extra")
            assert len(lane()) == 150 - len("LEFT[W] [W]RIGHT"), repr(lane())
            run("kill-window", "-t", "test:extra")

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
            observed, positions = set(), []
            deadline = time.monotonic() + 6
            while time.monotonic() < deadline:
                run("set-option", "-F", "-t", "test", "@runcat-keys", "#{e|+:#{@runcat-keys},8}")
                time.sleep(0.06)
                observed.add(option("@runcat-direction"))
                positions.append(int(option("@runcat-position")))
                if option("@runcat-direction") == "left":
                    pose = option("@runcat-roam-frame")
                    assert all(0x101000 <= ord(ch) < 0x10135C for ch in pose), repr(pose)
                if observed == {"left", "right"} and len(positions) > 20:
                    break
            assert observed == {"left", "right"}, observed
            assert max(positions) - min(positions) > 700, positions

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
            wait_for(lambda: option("@runcat-state") != "spin", "spin kept going after typing stopped", timeout=1)
            wait_for(lambda: option("@runcat-state") == "sleep", "spinning delayed sleep")
            position = option("@runcat-position")
            time.sleep(0.25)
            assert option("@runcat-position") == position, "sleeping cat moved"

            # Simulate a theme resetting the status format. The worker must
            # restore its overlay and remove it cleanly when disabled.
            run("set-option", "-g", "status-format[0]", baseline)
            wait_for(lambda: "@runcat-track" in run("show-options", "-gqv", "status-format[0]"),
                     "theme reload removed the track")
            run("set-option", "-g", "@runcat-roam", "off")
            wait_for(lambda: run("show-options", "-gqv", "status-format[0]") == baseline,
                     "disabling roaming did not restore the status format")
            print("PASS: available width, resizing, pacing, direction, spinning, sleep, status commands and theme reload")
        finally:
            stop.set()
            subprocess.run(command + ["kill-server"], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if client is not None:
                client.wait(timeout=3)
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
