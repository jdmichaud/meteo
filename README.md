# meteo

Terminal weather forecast for France, powered by Météo-France (AROME, ARPEGE) and Open-Meteo.

## Installation

```sh
cargo install --path . --root ~/.local
```

## Usage

```sh
meteo 78000    # first run
meteo          # subsequent runs
meteo --help   # all options
```

## Requirements

- 256-colour terminal
- Internet connection (except with `--fake`)

## Configuration

`~/.config/meteo/config.toml` — created automatically on first run.

---

See [AGENT.md](AGENT.md) for developer notes.
