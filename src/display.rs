use chrono::{Datelike, Local, Timelike};

use crate::color::colored_field;
use crate::config::Config;
use crate::graph::{self, OutLine, Role};
use crate::weather::{WeatherDay, WeatherEntry};

pub fn render(days: &[WeatherDay], config: &Config, location: Option<&str>) {
    // Find the most recent displayed hour slot that has already passed today.
    let now       = Local::now();
    let today     = now.date_naive();
    let now_hour  = now.time().hour() as u8;
    let current   = days
        .iter()
        .find(|d| d.date == today)
        .and_then(|d| {
            d.entries.iter()
                .map(|e| e.hour)
                .filter(|&h| h <= now_hour)
                .max()
                .map(|h| (today, h))
        });

    // Build every output line first so the temperature graph can be laid out
    // in the right margin and aligned with the forecast rows.
    let mut out: Vec<OutLine> = Vec::new();

    if let Some(name) = location {
        out.push(OutLine { text: name.to_string(), temp: None, role: Role::Plain });
    }

    let (h1, h2) = wind_header_lines(&config.language);
    out.push(OutLine { text: h1, temp: None, role: Role::GraphTitle });
    out.push(OutLine { text: h2, temp: None, role: Role::GraphAxis });

    let ordered: Vec<_> = if config.reverse_order {
        days.iter().rev().collect()
    } else {
        days.iter().collect()
    };
    for (idx, day) in ordered.iter().enumerate() {
        if idx > 0 {
            out.push(OutLine { text: String::new(), temp: None, role: Role::Body });
        }
        let entries: Vec<_> = if config.reverse_order {
            day.entries.iter().rev().collect()
        } else {
            day.entries.iter().collect()
        };
        for (entry_idx, entry) in entries.iter().enumerate() {
            let is_current = current == Some((day.date, entry.hour));
            out.push(OutLine {
                text: format_row(entry, day, entry_idx == 0, config, is_current),
                temp: Some(entry.temp_c),
                role: Role::Body,
            });
        }
    }

    graph::decorate(&mut out, config);

    for line in &out {
        println!("{}", line.text);
    }
}

fn wind_header_lines(lang: &str) -> (String, String) {
    // Offset to the wind columns:
    // gutter(2) + day(8) + hour(4) + temp(8) + wdir(5) = 27
    let pad = "                           "; // 27 spaces

    let (top, avg_lbl, burst_lbl) = if lang == "fr" {
        ("Vitesse vent", " Moy. ", "Rafale")
    } else {
        (" Wind speed ", " Avg. ", " Burst")
    };

    (format!("{}{}", pad, top), format!("{}{}{}", pad, avg_lbl, burst_lbl))
}

// Column widths (display chars):
//   gutter  day  hour  temp  wdir  wspd  wgst  pres  rain  icon
//     2      8     4     8     5     6     6     7    10    ...

fn format_row(
    entry: &WeatherEntry,
    day: &WeatherDay,
    show_day: bool,
    config: &Config,
    is_current: bool,
) -> String {
    // Gutter — 2 display chars: current-position indicator or blank
    let gutter = if is_current { "🠞 " } else { "  " };

    // Col 1 — day label (8 chars, left-aligned)
    let day_col = if show_day {
        let abbr = day_abbr(day.date.weekday(), &config.language);
        format!("{} {:>2}  ", abbr, day.date.day())
    } else {
        "        ".to_string()
    };

    // Col 2 — hour (4 chars, left-aligned: "2h  ", "11h ")
    let hour_col = format!("{:<4}", format!("{}h", entry.hour));

    // Col 3 — temperature °C (right-aligned number + 2 trailing spaces, coloured)
    let temp_str = format!("{:>4}°C  ", entry.temp_c.round() as i32);
    let temp_col = colored_field(
        &temp_str,
        entry.temp_c,
        config.temperature.min,
        config.temperature.max,
        false,
    );

    // Col 4 — wind direction (5 chars, centred arrow)
    let wdir_col = format!("{:^5}", entry.wind_dir.arrow());

    // Col 5 — wind speed km/h (right-aligned number + 2 trailing spaces, coloured when non-zero)
    let wspd_str = format!("{:>4}  ", entry.wind_speed_kmh.round() as u32);
    let wspd_col = colored_field(
        &wspd_str,
        entry.wind_speed_kmh,
        config.wind.min,
        config.wind.max,
        true,
    );

    // Col 6 — wind gust km/h (right-aligned number + 2 trailing spaces, coloured when non-zero)
    let wgst_str = format!("{:>4}  ", entry.wind_gust_kmh.round() as u32);
    let wgst_col = colored_field(
        &wgst_str,
        entry.wind_gust_kmh,
        config.wind.min,
        config.wind.max,
        true,
    );

    // Col 7 — pressure hPa (7 chars, right-aligned, no colour)
    let pres_col = format!("{:>7}", entry.pressure_hpa.round() as u32);

    // Col 8 — precipitation mm (right-aligned number + 2 trailing spaces, coloured when non-zero)
    let rain_str = format!("{:>6.1}mm  ", entry.precip_mm);
    let rain_col = colored_field(
        &rain_str,
        entry.precip_mm,
        config.water.min,
        config.water.max,
        true,
    );

    // Col 9 — condition icon with a fixed semantic background colour
    let (bg, dark_fg) = entry.condition.bg_color();
    let fg = if dark_fg { "\x1b[30m" } else { "\x1b[97m" };
    let icon = format!("   \x1b[48;5;{}m{} {} \x1b[0m", bg, fg, entry.condition.icon());

    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        gutter, day_col, hour_col, temp_col, wdir_col, wspd_col, wgst_col, pres_col, rain_col, icon
    )
}

fn day_abbr(weekday: chrono::Weekday, lang: &str) -> &'static str {
    if lang == "fr" {
        match weekday {
            chrono::Weekday::Mon => "Lun",
            chrono::Weekday::Tue => "Mar",
            chrono::Weekday::Wed => "Mer",
            chrono::Weekday::Thu => "Jeu",
            chrono::Weekday::Fri => "Ven",
            chrono::Weekday::Sat => "Sam",
            chrono::Weekday::Sun => "Dim",
        }
    } else {
        match weekday {
            chrono::Weekday::Mon => "Mon",
            chrono::Weekday::Tue => "Tue",
            chrono::Weekday::Wed => "Wed",
            chrono::Weekday::Thu => "Thu",
            chrono::Weekday::Fri => "Fri",
            chrono::Weekday::Sat => "Sat",
            chrono::Weekday::Sun => "Sun",
        }
    }
}
