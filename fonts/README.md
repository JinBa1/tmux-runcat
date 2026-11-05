# RunCat status animation

Run `bash fonts/setup.sh` from the plugin repository root. It builds and installs the clean
JetBrainsMono Nerd Font family and the separate `JetBrainsMono RunCat v10`
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
patched family defaults to `<original family> RunCat v10`. Use `--family
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
| `--fur` | `RUNCAT_FUR` |

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
enabled. It selects the generated family (`JetBrainsMono RunCat v10` by default)
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
symbol_map U+E400-U+E517,U+EFF0-U+EFFF,U+F600-U+F607,U+F620-U+F637,U+F640-U+F65F,U+F700-U+F8A3,U+100000-U+10009F,U+101000-U+10135B,U+102000-U+10226B,U+103000-U+10326B JetBrainsMono RunCat v10
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

The three edge reactions are a twenty-frame head bump and recovery,
a sixteen-frame climbing scratch, and a twenty-six-frame parkour
rebound. The sixteen-frame OIIA-inspired spin remains available.
Spin glyphs use U+100000–U+10009F; their mirrored glyphs use
U+101000–U+10135B. Wall glyphs use U+102000–U+10226B and
their mirrors use U+103000–U+10326B. Each pose has one-, two-,
three-, and four-cell variants, with the same foreground colour
and line metrics. These supplementary private-use ranges avoid Nerd Font
icon collisions. Font setup
creates Unicode format 12 character maps when a custom base lacks one.

Enable `@runcat-spin-chance` and `@runcat-roam` as described in the main README.
The spinner is built from small procedural cat geometry in `spinning_cat.py`;
the wall reactions use native Cat-1 pixels with added motion in `wall_cat.py`
and `parkour_cat.py`.
The bundled Pet Cats sprites provide the other actions.

Parkour has 26 poses, including an articulated head and torso turn, transfer
from the forepaws to the rear paws, extension, and landing. Its phase durations
in `pet-cats/parkour.tsv` total 2.4 seconds. The runtime uses the same release
and landing boundaries for statusline movement and keeps the planted artwork
oriented to its wall throughout the turn.

The unmodified [Pet Cats Pack](https://luizmelo.itch.io/pet-cat-pack) archive,
its CC0 notice, and the pinned pure Python PNG reader are in `vendor/`. No
download is needed during setup. The default monochrome skin uses Cat-1; the complete official
archive is retained for provenance. Use `#[fg=cyan]#{@runcat-frame}` in your
status format for a cyan cat, or choose any other tmux foreground color.

## Color coats

Color coats are optional. `--fur mono` is the default and creates the normal
foreground-colored cat. Motion settings are independent of the selected coat.

Build and install a colored coat with:

```sh
bash fonts/setup.sh --fur tuxedo
```

| `--fur` | Coat | Default font family |
| --- | --- | --- |
| `tabby` | Warm brown fur and dark stripes | JetBrainsMono RunCat Tabby v1 |
| `orange-tabby` | Ginger fur, rust stripes, pale muzzle | JetBrainsMono RunCat Orange Tabby v1 |
| `silver-tabby` | Cool grey fur and charcoal stripes | JetBrainsMono RunCat Silver Tabby v1 |
| `black` | Black fur with charcoal highlights | JetBrainsMono RunCat Black v1 |
| `tuxedo` | Black coat, white bib, muzzle and socks | JetBrainsMono RunCat Tuxedo v1 |
| `calico` | White coat with orange and black patches | JetBrainsMono RunCat Calico v1 |

Setup installs a separate family for the selected coat and, on WSL,
selects it in Windows Terminal. On Linux select that family in your terminal.
Keep `@runcat-skin pet-cats`; no worker rebuild or restart is needed. The
classic skin keeps its normal foreground outlines. `--build-only` and custom
base-font arguments work as usual; `RUNCAT_FUR=tuxedo` is the environment equivalent.
Use `bash fonts/setup.sh --fur mono` to return to the normal font.
If automatic coat selection is enabled, first set `@runcat-coat font` so the
next attachment does not select a colored family again.
Once families are installed, selecting their names in your terminal changes
coats immediately. On Windows, `install-windows.ps1 -KeepTerminalFont` installs
a built family without selecting it, for adding several coats together.

To install all six using the standard setup command, run this from the repo:

```sh
for coat in tabby orange-tabby silver-tabby black tuxedo calico; do
  bash fonts/setup.sh --fur "$coat"
done
```

Random selection uses only installed coats, so installing a subset is also
supported. Font setup selects each family as it installs it on WSL; use the
tmux settings below to control subsequent selection.

### Manual, fixed, or random selection

Keep `@runcat-skin pet-cats` in your tmux config. The coat selector is optional
and currently supports standard JetBrainsMono coat families in Windows
Terminal on WSL:

```tmux
set -g @runcat-coat font    # no automatic font changes (default)
```

Use `set -g @runcat-coat tuxedo` for a fixed installed coat, or
`set -g @runcat-coat random` for a fresh choice on startup, reload, or
attach/reattach. Put the setting before the plugin's load line. It takes
effect at the next selection event; changing it to `font` prevents future
automatic changes without removing the selected coat.

The random selector avoids an immediate repeat when more than one coat is
installed. It runs in the background on attachment without restarting the
animation worker. The choice is shared across the configured Windows Terminal
profiles, rather than being a separate font per tmux session. Concurrent
attachment events serialize the font update. `@runcat-coat-picked` reports
the last selection, and selection errors leave the worker running.

### Return to a solid foreground color

Stop automatic font changes and select the monochrome family:

```sh
tmux set -g @runcat-coat font
bash fonts/setup.sh --fur mono
```

If `JetBrainsMono RunCat v10` is already installed, select it directly instead
of rebuilding. Save `set -g @runcat-coat font` in your tmux config as well.
Set `@runcat-color cyan` (or another supported tmux foreground) for a roaming
cat; use `inherit` to follow the status foreground. A stationary cat can use
`#[fg=cyan]#{@runcat-frame}#[default]` in its status format.

### Rendering and previews

The coats use COLR v0 / CPAL solid-color layers. Color-capable terminals show
the fixed fur palette; other renderers retain the original monochrome
silhouette. `@runcat-color` affects that fallback, not the colored fur. Cell
widths, line height, original text, icons, and base-font color artwork are
preserved. Each pose is clipped into the same one to four cells as before.

The CC0 pack's Cat-6 supplies the authored stripes for the three tabbies.
Tail centerlines in `pet-cats/tabby-tails.tsv` add rings before wall transforms.
The scratch and bump carry the colored pixels through the existing motion;
custom parkour limbs and head use the same materials. Spin samples markings
in the rotating model's coordinates so the texture turns with the cat.
Black, tuxedo, and calico use Cat-1's gait and shading. Anatomical landmarks in
`pet-cats/coat-regions.tsv` anchor the white bib, socks, and patches before
posing. Calico uses orange on the rear flank, black toward the shoulder, a
split cap, and white underparts. White outer pixels receive subtle shading
to remain visible on light backgrounds. The three tabbies share the same markings.

To export every frame and a local viewer with playback, mirroring, stepping,
slow motion, light/dark backgrounds, and actual font samples:

```sh
PYTHONPATH=fonts/vendor/fonttools-4.67.0-py3-none-any.whl:fonts/vendor/pypng-0.20220715.0-py3-none-any.whl \
  python3 fonts/preview-cats.py /tmp/runcat-coat-review --fur tuxedo \
  --font "$HOME/.local/share/tmux-runcat/generated/RunCat-Regular.ttf"
```

Open `/tmp/runcat-coat-review/index.html` after building the matching coat font. The
viewer includes all 148 poses, both directions, and the production wall-action
timings. Locomotion previews use a representative speed; typing controls the
real speed in tmux. `preview-tabby.py` remains a compatibility entry point.
Coats are selected through the terminal font; `@runcat-skin` remains `pet-cats`.

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
