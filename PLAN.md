# meteo — Implementation Plan

## Overview

A Rust CLI tool that renders weather forecasts in the terminal with 256-colour gradients.  
Static binary; targets Linux (musl) and Windows (MSVC/GNU).

---

## 1. Project layout

```
meteo/
├── src/
│   ├── main.rs          # entry point: parse args, load config, render
│   ├── config.rs        # config file (TOML) load / create-default / validate
│   ├── weather.rs       # core data types (WeatherEntry, WeatherDay, Condition, WindDir)
│   ├── fake_data.rs     # deterministic fake forecast generator (dev/demo mode)
│   ├── display.rs       # terminal renderer: column layout, ANSI sequences
│   └── color.rs         # 256-colour gradient: value → terminal bg colour index
├── Cargo.toml
├── PLAN.md
├── README.md
├── AGENT.md
└── CLAUDE.md
```

---

## 2. Data model (`weather.rs`)

```rust
pub enum WindDir { N, NE, E, SE, S, SW, W, NW }
pub enum Condition { Clear, PartlyCloudy, Cloudy, LightRain, Rain, Storm, Snow, Fog }

pub struct WeatherEntry {
    pub hour: u8,              // 0–23
    pub temp_c: f32,           // °C
    pub wind_dir: WindDir,
    pub wind_speed_kmh: f32,   // average
    pub wind_gust_kmh: f32,    // burst
    pub pressure_hpa: f32,
    pub precip_mm: f32,
    pub condition: Condition,
}

pub struct WeatherDay {
    pub date: chrono::NaiveDate,
    pub entries: Vec<WeatherEntry>,
}
```

---

## 3. Config (`config.rs`)

Location (in order of preference):
1. `$XDG_CONFIG_HOME/meteo/config.toml`
2. `~/.config/meteo/config.toml`

Resolved via the `dirs` crate (`dirs::config_dir()`).  
If absent the file is created with compiled-in defaults.

### Schema (TOML)

```toml
language = "fr"        # "fr" | "en"

[temperature]
min = -30.0
max = 45.0

[wind]
min = 0.0
max = 150.0

[water]
min = 0.0
max = 150.0
```

Parsed with `serde` + `toml`.

---

## 4. Display format (`display.rs`)

One row per `WeatherEntry`. Column widths (characters):

| # | Content              | Width | Notes                                   |
|---|----------------------|-------|-----------------------------------------|
| 1 | Day label            | 8     | e.g. `Lun 12 ` or 8 spaces; first row of each day only |
| 2 | Hour                 | 4     | right-aligned, e.g. ` 2h`, `11h`        |
| 3 | Temperature          | 5     | right-aligned °C integer, coloured bg   |
| 4 | Wind direction       | 3     | Unicode arrow, centred                  |
| 5 | Wind speed           | 4     | right-aligned km/h, coloured bg         |
| 6 | Wind gust            | 4     | right-aligned km/h, coloured bg         |
| 7 | Pressure             | 7     | right-aligned hPa integer               |
| 8 | Precipitation        | 6     | right-aligned mm (1 decimal), coloured bg |
| 9 | Condition icon       | 3     | space + emoji                           |

A blank line is printed between days.

Day labels (French): `Lun`, `Mar`, `Mer`, `Jeu`, `Ven`, `Sam`, `Dim`  
Day labels (English): `Mon`, `Tue`, `Wed`, `Thu`, `Fri`, `Sat`, `Sun`

Wind direction arrows: `↑ ↗ → ↘ ↓ ↙ ← ↖`

Condition icons:
- Clear → `🌣`
- PartlyCloudy → `⛅`
- Cloudy → `☁`
- LightRain → `🌦`
- Rain → `🌧`
- Storm → `⛈`
- Snow → `🌨`
- Fog → `🌫`

---

## 5. Colour system (`color.rs`)

Uses ANSI 256-colour background escape `\x1b[48;5;{n}m`.

### Gradient mapping

The 6×6×6 colour cube (indices 16–231): `index = 16 + 36·r + 6·g + b` (r,g,b ∈ 0–5).

Chosen anchors:
- **min** → deep blue: (r=0, g=0, b=5) → index **21**
- **mid** → green:     (r=0, g=5, b=0) → index **46**
- **max** → deep red:  (r=5, g=0, b=0) → index **196**

Interpolation (t = normalised value 0.0–1.0):
- t ∈ [0, 0.5]: blend blue→green (b: 5→0, g: 0→5)
- t ∈ [0.5, 1]: blend green→red (g: 5→0, r: 0→5)

Special rules:
- Wind speed / precipitation value == 0 → **no background colour** (plain text)
- Clamp t to [0, 1] before interpolation

Reset with `\x1b[0m` after each coloured cell.

### 256-colour availability check

At startup: query `$TERM`, `$COLORTERM`, or `tput colors`. If fewer than 256 colours are available, print an error and exit.

---

## 6. Fake data provider (`fake_data.rs`)

Generates ~5 days of 3-hour-interval entries (8 entries/day) anchored to today's date.  
Values are deterministic (seeded by date) so output is reproducible during development.  
No external crate for randomness needed — simple arithmetic progressions with sine-wave variation.

---

## 7. Crate dependencies

```toml
[dependencies]
serde       = { version = "1", features = ["derive"] }
toml        = "0.8"
chrono      = { version = "0.4", default-features = false, features = ["std"] }
dirs        = "5"
```

No terminal-manipulation crate is needed: all output is plain `print!` with embedded ANSI codes.

---

## 8. Static compilation

### Linux (musl)

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

Build command:
```bash
cargo build --release --target x86_64-unknown-linux-musl
```

All dependencies chosen above have no C FFI, so musl works without extra flags.

### Windows

```toml
# Cargo.toml
[profile.release]
lto = true
```

`cargo build --release --target x86_64-pc-windows-gnu` (or MSVC) links the CRT statically by default when `crt-static` is set:

```toml
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

---

## 9. Implementation sequence

1. Scaffold `Cargo.toml`, module stubs, `CLAUDE.md`, `AGENT.md`
2. Implement `weather.rs` (data types only)
3. Implement `color.rs` (gradient function + 256-colour check)
4. Implement `fake_data.rs`
5. Implement `config.rs` (load / create-default)
6. Implement `display.rs` (layout engine, colour integration)
7. Wire together in `main.rs`
8. Add `README.md`
9. Add `.cargo/config.toml` for static targets
10. Smoke-test on Linux; verify column alignment with various fake datasets

---

## 10. Out of scope (Phase 1)

- Live API integration (Open-Meteo, Météo-France, etc.)
- Argument parsing beyond `--help` / `--version`
- Interactive TUI / scrolling
- Unit tests (can be added later)
