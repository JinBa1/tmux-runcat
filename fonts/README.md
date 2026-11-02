# RunCat status animation

Run `bash fonts/setup.sh` from the plugin repository root. It builds and installs the clean
JetBrainsMono Nerd Font family and the separate `JetBrainsMono RunCat v7`
family. No manual downloads, pip, or virtual
environment are needed. Python 3.11+ must be available on PATH.

The font sources, sprite pack, pure Python wheels, exact versions, checksums, and
licenses live in [vendor](vendor/README.md). The build verifies its inputs and
produces identical font files on repeated runs. Generated files live in
`${XDG_DATA_HOME:-~/.local/share}/tmux-runcat/`, outside the repository.

To rebuild and install only this feature:

```bash
bash fonts/setup.sh
```

To build without installing or changing terminal settings:

```bash
bash fonts/setup.sh --build-only
```

## Custom base fonts

Select a regular monospaced static TrueType font with `--base-font`. The
bundled fontTools handles the build offline; the original files are read only.

```bash
bash fonts/setup.sh \
  --base-font '/path/to/MyMono-Regular.ttf' \
  --bold-font '/path/to/MyMono-Bold.ttf' \
  --italic-font '/path/to/MyMono-Italic.ttf' \
  --bold-italic-font '/path/to/MyMono-BoldItalic.ttf'
```

Only the regular face is required. Supply optional faces from the same family
when available; missing faces are left for the terminal to synthesize. The
patched family defaults to `<original family> RunCat v7`. Use `--family
'My Mono RunCat'` to name it yourself. `--build-only` works with these options.
Setup prints the selected family, and WSL setup passes it to Windows Terminal.

The same options are available through environment variables, including when
the parent dotfiles setup invokes this script:

| Setup option | Environment variable |
| --- | --- |
| `--base-font` | `RUNCAT_BASE_FONT` |
| `--bold-font` | `RUNCAT_BOLD_FONT` |
| `--italic-font` | `RUNCAT_ITALIC_FONT` |
| `--bold-italic-font` | `RUNCAT_BOLD_ITALIC_FONT` |
| `--family` | `RUNCAT_FONT_FAMILY` |

Explicit command arguments override environment values. For example:

```bash
RUNCAT_BASE_FONT="$HOME/fonts/MyMono-Regular.ttf" ./setup.sh
```

Run that command from your dotfiles repository. To reproduce a custom build,
keep the same font files and options with your setup, along with their licenses.
Identical inputs produce identical fonts. With no custom options, setup keeps
using the pinned JetBrainsMono Nerd Font sources bundled in this repository.

Custom inputs must contain printable ASCII with equal character widths and
TrueType `glyf` outlines. Variable fonts, CFF/OpenType outlines, and web fonts
are rejected with an explanation; use a static TrueType export. Existing
characters and metrics are preserved. Use a Nerd Font base if your status bar
also needs Nerd Font device icons. A base that already uses one of the cat
codepoints is rejected rather than having its glyphs overwritten.

Cat outlines scale to the selected font's cell advance, units per em, cap
height, and line box. Fonts without a declared cap height use the height of
`H`. The base font's line metrics and ordinary character widths stay intact.
The generated `font-family.txt` and `font-names.json` describe the current
build; installers use that manifest to exclude files from earlier selections.

## Windows Terminal / WSL

Setup detects WSL and installs fonts in the current Windows user's font
directory using Windows PowerShell. WSL Windows interoperability must be
enabled. It selects the generated family (`JetBrainsMono RunCat v7` by default)
in stable Windows Terminal's defaults, WSL profiles, and profiles using the
previous default or an existing RunCat family, preserving other
settings. Settings and font registrations are backed up before changes.
Settings containing comments or trailing commas are accepted; changed files
are saved as standard JSON. Other terminal apps need their font selected in
their own settings.

Open a new terminal window if it has cached the old font. Choose
`JetBrainsMono Nerd Font` to use the clean font without the cat glyphs.
Installing fonts only inside WSL cannot supply them to Windows Terminal.

## Linux terminals

Setup installs the fonts in the user's font directory and refreshes fontconfig
when available. Select the printed family in your terminal. For Kitty,
you can instead map only the cat characters while keeping another main font:

```conf
symbol_map U+E400-U+E517,U+EFF0-U+EFFF,U+F600-U+F607,U+F620-U+F637,U+F640-U+F65F,U+F700-U+F8A3,U+100000-U+10009F,U+101000-U+10135B JetBrainsMono RunCat v7
```

## Animation and font construction

Place `#{@runcat-frame}` in your status format. The Rust
worker runs faster according to the last second of typing with no default
speed cap, slows smoothly, and shows a curled-up sleeping pose after two idle
seconds. A native tmux `Any` binding counts ordinary forwarded keys, then
passes the original input to the application. Existing specific bindings keep
precedence; mouse input is forwarded separately, and paste does not inflate
the typing rate. The helper reads only the count, not the key contents.

Each session has its own activity and pose. A per-server lock prevents duplicate
helpers on reload. Animation pauses when clients detach, and the worker exits
when the tmux server stops. The `@runcat-idle`, `@runcat-min-fps`, and
`@runcat-max-fps` tmux options control the delay and speed.
The maximum defaults to `0` (unlimited); a positive value sets an optional cap.

The `pet-cats` skin adds stretching on wake, walking/running with typing,
varied pause gestures, lying down, and two breathing sleeping poses. Select it
with `set -g @runcat-skin pet-cats`; select `classic` for the original silhouette.
New input interrupts resting gestures immediately. Pet Cats' running speed
uses the same uncapped typing-rate calculation as the classic skin.

Pet Cats uses U+F700–U+F8A3 for small/medium/large and U+E400–U+E517
for xlarge: 70 poses, each available in all four sizes. The
runtime and font builder share `pet-cats/animations.tsv`, so the action names,
frame counts, and glyph ordering stay in sync. Native pixels become TrueType
outlines that inherit the status foreground color, with eye cutouts. Existing
base-font color palettes and COLR v0/v1 artwork are preserved. The two sleeping
images gain the same subtle 0/4/8% breathing
motion, without adding text or changing cell widths.

Version 7 includes an original sixteen-frame slim, upright spinning expression
inspired by the OIIA cat meme, plus mirrored poses for every Pet Cats action. Existing
codepoints and geometry stay compatible. Spin glyphs use U+100000–U+10009F;
mirrored glyphs use U+101000–U+10135B. Each pose has one-, two-, three-, and
four-cell variants, with the same foreground colour and line metrics. These
supplementary private-use ranges avoid Nerd Font icon collisions. Font setup
creates Unicode format 12 character maps when a custom base lacks one.

Enable `@runcat-spin-chance` and `@runcat-roam` as described in the main README.
The spinner is built from small procedural cat geometry in `spinning_cat.py`;
the bundled Pet Cats sprites provide the other actions.

The unmodified [Pet Cats Pack](https://luizmelo.itch.io/pet-cat-pack) archive,
its CC0 notice, and the pinned pure Python PNG reader are in `vendor/`. No
download is needed during setup. This skin uses Cat-1; the complete official
archive is retained for provenance. Use `#[fg=cyan]#{@runcat-frame}` in your
status format for a cyan cat, or choose any other tmux foreground color.

Medium poses occupy two adjacent characters: U+EFF0–U+EFF4 for the left halves
and U+EFF5–U+EFF9 for the right halves. U+EFFA–U+EFFF contain three breathing
poses, drawn from `sleeping-cat.svg`. They raise the silhouette by 0%, 4%, and
8% over a three-second cycle. Small poses use U+F600–U+F607 (one cell);
large poses use U+F620–U+F637 (three cells); xlarge poses use U+F640–U+F65F
(four cells). All presets contain five running and three breathing poses.
Both states keep the selected width, preserving the status layout. The large
cat fills more of the existing row height. Xlarge uses the full line box,
including the descent, for about 35% larger Pet Cats sprites than large.
The silhouette fills most of the text
height without changing terminal character widths. Existing JetBrainsMono
glyphs, spacing, weights, and ligature features are preserved.

The source `runcat-tmux.ttf` comes from
[runcat-text](https://github.com/bzglve/runcat-text/blob/master/runcat.ttf),
with the five original poses remapped from U+E900–U+E904 to U+EFF0–U+EFF4.
Its BSD license is in `LICENSE`. Setup builds the two-character poses from this
source and the selected base font files.

Run the build regression checks with:

```bash
python3 fonts/tests/test_build.py
cargo test --locked --offline
python3 tests/live_tmux.py
```
