use std::collections::BTreeMap;

use chrono::{Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use serde::Deserialize;

use crate::weather::{Condition, WeatherDay, WeatherEntry, WindDir};

// Hours we render per day (must match fake_data and display).
const DISPLAY_HOURS: &[u32] = &[2, 5, 8, 11, 14, 17, 20, 23];

// ── Open-Meteo response shapes ────────────────────────────────────────────────

#[derive(Deserialize)]
struct ApiResponse {
    hourly: HourlyData,
}

#[derive(Deserialize)]
struct HourlyData {
    time:               Vec<String>,
    temperature_2m:     Vec<Option<f64>>,
    precipitation:      Vec<Option<f64>>,
    weathercode:        Vec<Option<i64>>,
    windspeed_10m:      Vec<Option<f64>>,
    winddirection_10m:  Vec<Option<f64>>,
    windgusts_10m:      Vec<Option<f64>>,
    surface_pressure:   Vec<Option<f64>>,
}

struct RawEntry {
    temp_c:          f32,
    precip_mm:       f32,
    wmo_code:        i64,
    wind_speed_kmh:  f32,
    wind_dir_deg:    f32,
    wind_gust_kmh:   f32,
    pressure_hpa:    f32,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Fetch a hybrid forecast: AROME (days 1-2) → ARPEGE (days 3-4) → best_match (days 5+).
pub fn fetch_forecast(lat: f64, lon: f64) -> Result<Vec<WeatherDay>, Box<dyn std::error::Error>> {
    let today = Local::now().date_naive();

    // Start with best_match as the baseline for all 16 days.
    let mut merged: BTreeMap<NaiveDateTime, RawEntry> = BTreeMap::new();
    for (dt, e) in fetch_model(lat, lon, None, 16)? {
        merged.insert(dt, e);
    }

    // Override days 1-4 with ARPEGE when available (France + Europe).
    let arpege_cutoff = today + Duration::days(4);
    match fetch_model(lat, lon, Some("meteofrance_arpege_europe"), 4) {
        Ok(data) => {
            for (dt, e) in data {
                if dt.date() < arpege_cutoff {
                    merged.insert(dt, e);
                }
            }
        }
        Err(_) => eprintln!("Warning: ARPEGE unavailable, using best_match for days 1-4."),
    }

    // Override days 1-2 with AROME when available (metropolitan France only).
    let arome_cutoff = today + Duration::days(2);
    if let Ok(data) = fetch_model(lat, lon, Some("meteofrance_arome_france"), 2) {
        for (dt, e) in data {
            if dt.date() < arome_cutoff {
                merged.insert(dt, e);
            }
        }
    }
    // AROME failure is silent — location may simply be outside its coverage area.

    // Build WeatherDay list.
    // AROME days (first 2): keep every hour — that resolution is worth showing.
    // ARPEGE / best_match days: keep only the 3-hourly display slots.
    let mut days_map: BTreeMap<NaiveDate, Vec<WeatherEntry>> = BTreeMap::new();
    for (dt, raw) in &merged {
        let h = dt.time().hour();
        let date = dt.date();
        let keep = if date < arome_cutoff {
            true
        } else {
            DISPLAY_HOURS.contains(&h)
        };
        if !keep { continue; }
        days_map
            .entry(date)
            .or_default()
            .push(raw_to_entry(raw, h as u8));
    }

    Ok(days_map
        .into_iter()
        .map(|(date, entries)| WeatherDay { date, entries })
        .collect())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn fetch_model(
    lat: f64,
    lon: f64,
    model: Option<&str>,
    forecast_days: u32,
) -> Result<BTreeMap<NaiveDateTime, RawEntry>, Box<dyn std::error::Error>> {
    let mut url = format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat:.4}&longitude={lon:.4}\
         &hourly=temperature_2m,precipitation,weathercode,\
                 windspeed_10m,winddirection_10m,windgusts_10m,surface_pressure\
         &wind_speed_unit=kmh\
         &timezone=Europe%2FParis\
         &forecast_days={forecast_days}"
    );
    if let Some(m) = model {
        url.push_str(&format!("&models={m}"));
    }

    let resp: ApiResponse = reqwest::blocking::get(&url)?
        .error_for_status()?
        .json()?;

    let h = &resp.hourly;
    let mut map = BTreeMap::new();
    for i in 0..h.time.len() {
        let dt = NaiveDateTime::parse_from_str(&h.time[i], "%Y-%m-%dT%H:%M")?;
        map.insert(dt, RawEntry {
            temp_c:         h.temperature_2m[i].unwrap_or(0.0)    as f32,
            precip_mm:      h.precipitation[i].unwrap_or(0.0)     as f32,
            wmo_code:       h.weathercode[i].unwrap_or(0),
            wind_speed_kmh: h.windspeed_10m[i].unwrap_or(0.0)     as f32,
            wind_dir_deg:   h.winddirection_10m[i].unwrap_or(0.0) as f32,
            wind_gust_kmh:  h.windgusts_10m[i].unwrap_or(0.0)     as f32,
            pressure_hpa:   h.surface_pressure[i].unwrap_or(1013.0) as f32,
        });
    }
    Ok(map)
}

fn raw_to_entry(raw: &RawEntry, hour: u8) -> WeatherEntry {
    WeatherEntry {
        hour,
        temp_c:         raw.temp_c,
        wind_dir:       degrees_to_wind_dir(raw.wind_dir_deg),
        wind_speed_kmh: raw.wind_speed_kmh,
        wind_gust_kmh:  raw.wind_gust_kmh,
        pressure_hpa:   raw.pressure_hpa,
        precip_mm:      raw.precip_mm,
        condition:      wmo_to_condition(raw.wmo_code),
    }
}

fn degrees_to_wind_dir(deg: f32) -> WindDir {
    // Open-Meteo reports the direction the wind blows FROM (meteorological convention).
    // Adding 180° converts to the direction it blows TOWARD, which is what arrows show.
    let d = (deg + 180.0).rem_euclid(360.0) as u32;
    match d {
        0..=22    => WindDir::N,
        23..=67   => WindDir::NE,
        68..=112  => WindDir::E,
        113..=157 => WindDir::SE,
        158..=202 => WindDir::S,
        203..=247 => WindDir::SW,
        248..=292 => WindDir::W,
        293..=337 => WindDir::NW,
        _         => WindDir::N,
    }
}

fn wmo_to_condition(code: i64) -> Condition {
    match code {
        0       => Condition::Clear,
        1       => Condition::MostlyClear,
        2       => Condition::PartlyCloudy,
        3       => Condition::Overcast,
        45 | 48 => Condition::Fog,
        51..=57 => Condition::Drizzle,
        61 | 63 | 65 => Condition::Rain,
        66 | 67 => Condition::Sleet,      // freezing rain
        71..=77 => Condition::Snow,
        80..=82 => Condition::RainShowers,
        85 | 86 => Condition::SnowShowers,
        95..=99 => Condition::Thunderstorm,
        _       => Condition::Clear,
    }
}
