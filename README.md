# Ну, погоди!

A clone of the Soviet **Elektronika IM-02 "Nu, pogodi!"** LCD handheld (1984), itself a copy of
Nintendo's Game & Watch *Egg*. Built on [game-toolkit](https://crates.io/crates/game-toolkit),
runs natively and in the browser (WebGPU, WebGL2 fallback).

The wolf catches eggs rolling down four ramps. Miss one and it breaks: a full miss. If the hare
is in the window, a chick hatches and runs off instead: half a miss (blinking icon). Three
misses end the game. Reaching 200 and 500 clears all misses. The pace rises with the score and
eases off for a while after every hundred. When idle, the device is a 12-hour clock with an
alarm, and the wolf plays a silent demo.

The rules are a clean-room reimplementation. The original ROM's timing tables aren't
published, so the pace is tuned by feel (`src/logic/speed.rs`).

## Controls

| Device button | Keys | Mouse / touch |
|---|---|---|
| Upper left / lower left ramp | `Q` / `A` (also `W` / `S`, numpad `7` / `1`) | big red buttons |
| Upper right / lower right ramp | `P` / `L` (also `O` / `K`, numpad `9` / `3`) | big red buttons |
| ИГРА А / ИГРА Б (Game A / B) | `1` / `2` | small buttons |
| ВРЕМЯ (time; hold to show the alarm) | `T` or `3` | small button |
| Set alarm / set time | `Z` / `X` | recessed buttons |
| Mute | `M` | |

- **Game A** uses three ramps; which one is idle depends on your misses. **Game B** uses all four
  and starts faster.
- **Setting the time or alarm:** upper right = hours, lower right = minutes, either left = ДП/ПП
  (AM/PM). Press the same set button again (or ВРЕМЯ) to finish.
- **Alarm on/off:** in alarm-set mode, press ИГРА А. When the alarm rings, press ВРЕМЯ to stop it.
- Debug builds only: `N` adds 10 points.

The high score is kept in `localStorage` (web) or the user data dir (native).

## Build

```sh
task setup        # once: wasm32 target, trunk, atlas-packer
task run          # native
task web          # http://localhost:8080, live rebuild (needs trunk + wasm32 target)
task test         # logic tests
task check        # fmt + clippy; task check:wasm for the browser target
```

## Art

All graphics are PNGs you can redraw. The current set is drawn by `tools/gen-placeholders`
after the real IM-02: grey shell, recessed silver panel, red buttons, and an LCD film with
red-brown henhouses and green bushes.

- `art/segments/*.png`: one image per LCD segment, drawn in solid ink on transparency. The game
  tints them, so only the alpha channel matters. `press_*.png` are the masks that shade held
  buttons.
- `art/lcd_bg.png` (660×416): the printed LCD film (henhouses, ramps, hens, bushes).
- `art/lcd_glass.png` (660×416): drawn over the segments (frame shadow and glare).
- `art/bezel.png` (1120×672): the device casing, including all printed labels.
- `assets/layout.json`: where each segment's center sits on the LCD and where the bezel buttons
  are. Several segments can share one sprite (e.g. all eggs use `egg`).

`task art` only writes missing files (`FORCE=1` regenerates everything, layout included).
After editing, run `task atlas` (needs `atlas-packer`; `task setup` installs it).

The generator bundles DejaVu Sans Bold for the labels (`tools/gen-placeholders/fonts`, see its
`LICENSE`).

## Embedding in SvelteKit

```sh
task web:export DEST=../my-site/static/nupogodi
cp embed/NuPogodi.svelte ../my-site/src/lib/
```

```svelte
<script>
  import NuPogodi from '$lib/NuPogodi.svelte';
</script>

<NuPogodi base="/nupogodi" />
```

The component sizes itself to its container (1120:672 aspect ratio, at most 1120 px wide). The
wasm is about 4.9 MB (2.0 MB gzipped) and is only fetched when the component first mounts.

The game binds to its canvas once and has no teardown. The component therefore keeps one canvas
for the page's lifetime: unmounting detaches it, which pauses the game, and mounting again
re-attaches it. Only one instance can be on a page at a time.

## License

EUPL-1.2
