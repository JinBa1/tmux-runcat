# tmux-runcat

A small cat for your tmux status bar. It wakes when you type, runs faster as
your typing speeds up without a speed cap, slows down, and curls up to sleep
after two idle seconds.
Each session has independent activity. Choose the classic silhouette or the
expressive Pet Cats skin, which stretches, walks, runs, grooms, meows, and
lies down before sleeping. When sleeping, the cat breathes gently
over a three-second cycle. Choose a one-, two-, three-, or four-cell cat with
`@runcat-size`. Its width stays fixed in both states at the selected size.

The loader and animation worker are one Rust executable with no third-party
Rust dependencies. Key counting uses tmux commands; no process is started per
keystroke, and the plugin never reads or stores key contents.

## Requirements

- tmux (tested against 3.6) on Linux or WSL
- Rust 1.89+ and Cargo to build; the running plugin needs only tmux
- Bash for the `.tmux` entry point
- Python 3.11+, tar with xz support, and sha256sum for the one-time font setup
- On WSL: Windows interoperability and Windows PowerShell for font installation

## Install

```sh
git clone https://github.com/maxnaro/tmux-runcat.git ~/.config/tmux/plugins/tmux-runcat
cd ~/.config/tmux/plugins/tmux-runcat
cargo build --release --locked --offline
bash fonts/setup.sh
```

The pinned font sources and build tools are bundled, so building the binary
and fonts needs no network after cloning. Font setup installs both the clean
JetBrainsMono Nerd Font family and `JetBrainsMono RunCat v10`, which adds
the running and sleeping poses while preserving existing glyphs and spacing.

On WSL, setup installs fonts on Windows and selects the combined font in stable
Windows Terminal. On Linux, select `JetBrainsMono RunCat v10` in your terminal.
See [font setup](fonts/README.md) for details and backups.

To use your own base font, supply a regular monospaced static TrueType file:

```sh
bash fonts/setup.sh --base-font '/path/to/MyMono-Regular.ttf'
```

Setup creates a separate `<original family> RunCat v10` family, using the base
font's spacing and height for all cat sizes. Add `--bold-font`, `--italic-font`,
and `--bold-italic-font` for matching faces, or `--family` to choose the output
family name. These are font setup options; terminal fonts cannot be selected
through a tmux option. See [custom fonts](fonts/README.md#custom-base-fonts)
for environment variables, supported formats, and reproducible setup.

Include `#{@runcat-frame}` wherever you want the cat, and load the plugin after
your status settings:

```tmux
set -g status-right '#{@runcat-frame} | #h | %H:%M '
run-shell '~/.config/tmux/plugins/tmux-runcat/runcat.tmux "#{socket_path}" "#{pid}"'
```

Pass the socket and PID as shown to support custom tmux sockets during startup.
Reload with `tmux source-file ~/.config/tmux/tmux.conf`.

For TPM, add `set -g @plugin 'maxnaro/tmux-runcat'` to your plugin list. After
TPM clones it, build the binary, run font setup, add the status placeholder,
and reload tmux. The entry point uses the current `TMUX` environment when no
socket arguments are supplied.

## Options

Set these before loading the plugin; existing values are preserved. Values
can also be changed with `set-option` while the animation is running.

| Option | Default | Meaning |
| --- | --- | --- |
| `@runcat-idle` | `2` | Idle seconds before sleeping (0.1–3600) |
| `@runcat-min-fps` | `3` | Minimum running speed (1–20 FPS) |
| `@runcat-max-fps` | `0` | `0` or `unlimited` removes the cap; a positive FPS value sets an optional limit |
| `@runcat-size` | `medium` | `small` (1 cell), `medium` (2 cells), `large` (3 cells), or `xlarge` (4 cells) |
| `@runcat-skin` | `classic` | `classic` or `pet-cats` |
| `@runcat-spin-chance` | `0` | Pet Cats: percent chance per second of running to start an OIIA-style spin (0–100) |
| `@runcat-roam` | `off` | `on` lets the cat pace through the free space between the window list and right status |
| `@runcat-wall-chance` | `35` | Pet Cats while roaming: chance to react at an end (0–100); bumps require running |
| `@runcat-wall-debug` | `off` | Pet Cats while roaming: automatically run and play each edge reaction in order |

```tmux
set -g @runcat-idle 3
set -g @runcat-min-fps 4
set -g @runcat-max-fps 0
set -g @runcat-size medium
set -g @runcat-skin pet-cats
```

The Pet Cats skin uses the first cat from [Pet Cats Pack by LuizMelo](https://luizmelo.itch.io/pet-cat-pack),
bundled under CC0. Install the v10 fonts before selecting it. Both skins inherit
the status foreground color, including on terminals with color-font support.
For example, `#[fg=cyan]#{@runcat-frame}` draws a cyan cat; eye details remain
cutouts. Both skins work with custom base fonts and all four sizes. Skin changes
apply live.

| Activity | Pet Cats behavior |
| --- | --- |
| First key after sleep | A short stretch, with continued typing still counted |
| Gentle typing | Walk |
| 8+ keys in the last second | Run, with uncapped speed following typing rate |
| Brief pause | Rotate between grooming, idle, scratching, meowing, and sitting |
| Approaching the idle timeout | Lie down |
| Idle timeout | Sleep with subtle breathing |
| Another 30 seconds of rest | Alternate sleeping pose |

The sleep timeout is still `@runcat-idle` (2 seconds by default). Wake and
bedtime transitions shorten for smaller timeout values. New input interrupts
pause gestures and lying down immediately. There is no Zzz text or audio.
Reactions use activity counts; no special Enter or Backspace bindings are added.
The current action is exposed as `#{@runcat-state}` for optional status formats
and troubleshooting.

### Spinning and roaming

Install the v10 font, then enable both features:

```tmux
set -g @runcat-skin pet-cats
set -g @runcat-spin-chance 18
set -g @runcat-roam on
set -g @runcat-wall-chance 35
```

While running, the cat gets one chance per second to start an OIIA-inspired
spin in place. Its rotation speeds up and slows down with typing, up to
`@runcat-max-fps` when that cap is set. It keeps
rotating as long as you type, then resumes its usual
rest behavior when typing pauses. Spins have an eight-second cooldown between
starts and never delay the idle timeout. Set the chance to `0` to disable
them. This is a visual expression; it does not play audio.

Roaming uses the standard single-row tmux status layout, including Powerline.
The cat moves while walking or running, faces its direction of travel, and
turns around at either edge. It rests where it stopped. The available path is
measured for each client from its terminal width and rendered left status,
window list, and right status, so resizing and changing window names resize
the path. The cat hides when there is no room. Theme reloads preserve roaming.
Remove `#{@runcat-frame}` from any stationary status segment when enabling
roaming, so the cat is drawn once. Custom layouts without the standard
right-aligned status block keep their existing layout.

At either end, the Pet Cats skin has a 35% chance to react before turning.
A walking cat may stretch upward and scratch. A running cat may scratch,
bump its head and stumble back, or leap at the edge and bounce back.
While running, head bump and parkour are each twice as likely as scratch;
walking keeps scratch at its previous frequency. Set
`@runcat-wall-chance 0` to disable reactions or `100` to trigger one at every
end. During parkour, it turns at the edge before running back.

To inspect every edge animation without typing, enable debug mode:

```tmux
set -g @runcat-skin pet-cats
set -g @runcat-roam on
set -g @runcat-wall-debug on
```

The cat runs between the ends and plays head bump, wall scratch, and
parkour in that order, then repeats. Debug mode ignores
`@runcat-wall-chance` and disables spinning while active. Set
`@runcat-wall-debug off` to resume normal behavior. Turning it on again starts
the sequence from head bump.

For terminals that support synchronized updates, add the tmux `sync` terminal
feature before attaching a client. This presents fast status redraws together
and avoids cursor flashes between partial updates. For Windows Terminal with
`TERM=xterm-256color`, add this to your tmux configuration:

```tmux
set -as terminal-features ',xterm-256color:sync'
```

Then attach a new client.

Running speed follows the last second's key rate: the minimum plus 0.75 FPS
per key per second, with smooth acceleration and deceleration. There is no
default maximum. Frames follow their own deadlines rather than a fixed 20 Hz
update loop, with elapsed animation time preserved when a redraw is late.
Actual redraw throughput depends on tmux and the terminal.

After updating an existing server from an older version, run
`tmux set -g @runcat-max-fps 0` to remove its previously saved limit.

Size updates apply immediately, including while sleeping. Numeric aliases
`1`, `2`, `3`, and `4` are accepted; invalid values use `medium`. All sizes share
one font installation. The large preset uses more of the available row height,
and `xlarge` uses the full existing line box, including the descent. The rest
of the terminal keeps its normal font size. Install the v10 font to use `xlarge`.

## Behavior

The plugin reserves the root table's `Any` binding for counting ordinary keys
and forwarding the original input. Specific bindings, including prefixes and
your shortcuts, keep precedence. Mouse input is forwarded without counting it
as typing, and bracketed paste bypasses the counter. Copy-mode, popup, and tmux
command-prompt keys do not contribute to the typing rate.

An existing custom root `Any` binding causes the loader to stop with an error;
it is never silently overwritten. Other key tables and status formats are
preserved. Repeated loads replace the worker generation, so code updates take
effect while one worker per server remains active. Workers stop when the
server stops and pause animation when clients detach.

## Use as a dotfiles submodule

```sh
git submodule add git@github.com:maxnaro/tmux-runcat.git tmux/.config/tmux/plugins/tmux-runcat
git submodule update --init --recursive
cargo build --release --locked --offline --manifest-path tmux/.config/tmux/plugins/tmux-runcat/Cargo.toml
bash tmux/.config/tmux/plugins/tmux-runcat/fonts/setup.sh
```

Commit plugin changes in this repository first, then update its pointer in the
parent dotfiles repository. Rebuild the executable after updating the plugin.

## Development

```sh
cargo fmt --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline
cargo build --release --locked --offline
python3 fonts/tests/test_build.py
python3 tests/live_tmux.py
python3 tests/roaming_tmux.py
bash -n runcat.tmux fonts/setup.sh
```

The live test starts an isolated tmux server and PTY, checking Pet Cats actions,
live skin changes, fast redraws,
input forwarding, idle/wake behavior, mouse events, prefix shortcuts, live
settings, and reloads.
Font tests verify repeatable builds, preserved character mappings, spacing,
font features, custom base fonts, and running/sleeping glyph geometry.
On Windows, `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
fonts/tests/test-windows-terminal.ps1` checks terminal settings using temporary
files. Build outputs stay in
`target/` and the user's data directory. Set `RUNCAT_BINARY` to run an alternate
executable through the shell entry point.

Third-party font sources and their licenses are in `fonts/` and `fonts/vendor/`.
