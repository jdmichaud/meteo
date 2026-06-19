mod api;
mod color;
mod config;
mod display;
mod fake_data;
mod geocode;
mod weather;

fn main() {
    #[cfg(target_os = "windows")]
    color::enable_windows_ansi();

    if !color::check_256_colors() {
        eprintln!("Error: a 256-colour terminal is required.");
        eprintln!("Hint: set TERM=xterm-256color or use a modern terminal emulator.");
        std::process::exit(1);
    }

    // ── CLI args ──────────────────────────────────────────────────────────────
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return;
    }

    let use_fake = args.iter().any(|a| a == "--fake");
    let order_arg: Option<bool> = if args.iter().any(|a| a == "--reverse") {
        Some(true)
    } else if args.iter().any(|a| a == "--forward") {
        Some(false)
    } else {
        None
    };
    // First non-flag argument is treated as a postcode.
    let postcode_arg: Option<String> = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .cloned();

    // ── Config ────────────────────────────────────────────────────────────────
    let mut cfg = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    // ── Apply order flag ──────────────────────────────────────────────────────
    if let Some(rev) = order_arg {
        if cfg.reverse_order != rev {
            cfg.reverse_order = rev;
            if let Err(e) = config::save_config(&cfg) {
                eprintln!("Warning: could not save order preference: {e}");
            }
        }
    }

    // ── Fake data shortcut ────────────────────────────────────────────────────
    if use_fake {
        let days = fake_data::generate_fake_forecast(5);
        display::render(&days, &cfg, None);
        return;
    }

    // ── Resolve postcode ──────────────────────────────────────────────────────
    let postcode = match postcode_arg.clone().filter(|s| !s.is_empty()) {
        Some(p) => p,
        None if !cfg.postcode.is_empty() => cfg.postcode.clone(),
        _ => {
            eprintln!("No postcode provided.");
            eprintln!("Usage:  meteo <postcode>");
            if let Some(p) = config::config_path() {
                eprintln!("   or:  set 'postcode' in {}", p.display());
            }
            std::process::exit(1);
        }
    };

    // ── Geocode ───────────────────────────────────────────────────────────────
    let location = match geocode::geocode(&postcode) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Geocoding failed for '{}': {e}", postcode);
            std::process::exit(1);
        }
    };

    // Persist postcode to config when supplied on the command line.
    if postcode_arg.as_deref().is_some() && postcode != cfg.postcode {
        cfg.postcode = postcode.clone();
        if let Err(e) = config::save_config(&cfg) {
            eprintln!("Warning: could not save postcode to config: {e}");
        }
    }

    // ── Fetch weather ─────────────────────────────────────────────────────────
    let days = match api::fetch_forecast(location.latitude, location.longitude) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Weather fetch failed: {e}");
            std::process::exit(1);
        }
    };

    // ── Render ────────────────────────────────────────────────────────────────
    let header = format!("{} ({})", location.name, postcode);
    display::render(&days, &cfg, Some(&header));
}

fn print_help() {
    let config_path = config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.config/meteo/config.toml".to_string());

    println!("meteo — terminal weather forecast");
    println!();
    println!("USAGE");
    println!("  meteo [OPTIONS] [POSTCODE]");
    println!();
    println!("ARGUMENTS");
    println!("  POSTCODE   French postcode (e.g. 75001).");
    println!("             Saved to config on first use; omit on subsequent runs.");
    println!();
    println!("OPTIONS");
    println!("  -h, --help   Show this help message and exit.");
    println!("  --fake       Use deterministic fake data (no network required).");
    println!("  --reverse    Show furthest day first, today last (saved to config).");
    println!("  --forward    Show today first, furthest day last  (saved to config).");
    println!();
    println!("CONFIG  {}", config_path);
    println!("  language        Display language: \"fr\" (default) or \"en\".");
    println!("  postcode        Default location (saved automatically).");
    println!("  reverse_order   Persistent order preference (set via --reverse/--forward).");
    println!("  [temperature]   min / max °C for the colour gradient.");
    println!("  [wind]          min / max km/h for the colour gradient.");
    println!("  [water]         min / max mm for the colour gradient.");
    println!();
    println!("FORECAST");
    println!("  Days 1–2   AROME 1.3 km (Météo-France, metropolitan France).");
    println!("  Days 3–4   ARPEGE ~10 km (Météo-France, Europe).");
    println!("  Days 5–16  Open-Meteo best_match (ECMWF / GFS blend, global).");
}
