# meteo — Agent & Developer Notes

## Quick start

```sh
cargo build          # debug build
cargo run -- 78000   # run with a French postcode
cargo run            # run using the postcode saved in config
cargo run -- --fake  # run with deterministic fake data (no network)
```

## Static release builds

### Linux (musl — zero dynamic dependencies)

```sh
rustup target add x86_64-unknown-linux-musl
sudo apt install musl-tools        # Debian/Ubuntu; adapt for other distros
cargo build --release --target x86_64-unknown-linux-musl
# binary: target/x86_64-unknown-linux-musl/release/meteo
```

All crate dependencies are pure Rust (reqwest uses rustls, not OpenSSL), so no
extra `CC` flags or system libraries are needed for musl.

### Windows MSVC (static CRT)

```sh
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
# binary: target\x86_64-pc-windows-msvc\release\meteo.exe
```

The `rustflags = ["-C", "target-feature=+crt-static"]` in `.cargo/config.toml`
ensures `MSVCRT` is linked statically.

### Windows GNU (static CRT)

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# binary: target\x86_64-pc-windows-gnu\release\meteo.exe
```

## Module map

| File | Responsibility |
|------|---------------|
| `src/main.rs` | Entry point: arg parsing, geocode, API fetch, dispatch to `display` |
| `src/config.rs` | TOML config load / save / auto-create default, XDG path resolution |
| `src/geocode.rs` | Postcode → lat/lon/name via api-adresse.data.gouv.fr |
| `src/api.rs` | Hybrid Open-Meteo fetch: AROME (2 d) → ARPEGE (4 d) → best_match (16 d) |
| `src/weather.rs` | Core types: `WeatherEntry`, `WeatherDay`, `WindDir`, `Condition` |
| `src/fake_data.rs` | Deterministic fake 5-day forecast (`--fake` flag, no network) |
| `src/display.rs` | Fixed-width column layout, row formatting, ANSI colour integration |
| `src/graph.rs` | Optional braille side-graphs (temp / pressure / rain) in the right margin |
| `src/color.rs` | 256-colour gradient, terminal capability check, Windows ANSI init |

## Colour system

Gradient uses the 6×6×6 colour cube (indices 16–231): `index = 16 + 36·r + 6·g + b`.

Anchors:
- t = 0.0 → deep blue (r=0, g=0, b=5) → index 21
- t = 0.5 → green     (r=0, g=5, b=0) → index 46
- t = 1.0 → deep red  (r=5, g=0, b=0) → index 196

`t = (value − min) / (max − min)`, clamped to [0, 1].
Wind speed and precipitation at exactly 0 get no background colour.

## Column layout

```
[day:8][hour:4][temp:4][wdir:4][wspd:4][wgst:4][pres:7][rain:6][icon]
```

All widths are in terminal display columns. The arrow characters (↑ ↗ → …)
are narrow (1 column); the emoji icons are wide (2 columns) but sit at the
end of the line so they do not affect column alignment.

## Side-graphs

`src/display.rs` builds every output line into a `Vec<OutLine>` (each tagged with
its `Role` and, for forecast rows, its `Metrics`) and hands it to
`graph::decorate`, which draws one or more line-graphs in the right margin when
the terminal is wide enough.

Because the table is tall and narrow, each graph is a **full-height panel aligned
row-for-row** with the forecast:

- vertical axis = time — one anchor per forecast row, so each line's curve point
  sits next to that row's value (order-agnostic: works with both `--forward` and
  `--reverse`);
- horizontal axis = the metric value — auto-scaled to the data (low → left), with
  the min/max printed on the second header line;
- curves use Unicode braille dots (8 sub-cells per glyph, as in uplot).
  Temperature and rain reuse the same colour gradient as their table columns
  (`temperature` / `water` ranges); pressure has no gradient so it uses one fixed
  hue.

Series are defined in `decorate` in priority order — **temperature, pressure,
rain** — and as many as fit are drawn side by side, sharing one width. In reverse
mode the header (and therefore the titles/axes) is repeated as a footer, giving
the plots a bottom axis too.

Layout constants live at the top of `src/graph.rs`: `GAP` (table → first graph),
`GRAPH_GAP` (between graphs), `MIN_W` (skip a graph below this width) and `MAX_W`
(per-graph cap on wide terminals). Width comes from `terminal_size`; piped output
gets no graph. Set `METEO_COLS=<n>` to force a width (handy for screenshots/tests).

## Weather API — model cascade

`src/api.rs` fetches three models and merges by time priority:

| Layer | Model | Horizon | Coverage |
|-------|-------|---------|----------|
| 1 | `meteofrance_arome_france` | 2 days | Metropolitan France only |
| 2 | `meteofrance_arpege_europe` | 4 days | Europe |
| 3 | Open-Meteo best_match | 16 days | Global |

AROME failure is silent (location outside coverage). ARPEGE failure prints a warning.
The best_match call is mandatory; its failure is a hard error.

WMO → Condition mapping is in `wmo_to_condition()` at the bottom of `src/api.rs`.

## Editing fake data

Edit the constants and formulas in `src/fake_data.rs` to test edge cases
(e.g. negative temperatures, high wind, heavy rain). Values are derived purely
from `day_offset` and `hour`, so output is always reproducible.

## Config defaults

Change factory defaults in `Config::default()` in `src/config.rs`.
The config file is **only created when absent** — editing defaults does not
overwrite an existing user config.
