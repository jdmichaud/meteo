use chrono::{Duration, Local};

use crate::weather::{Condition, WeatherDay, WeatherEntry, WindDir};

/// Generates a deterministic fake forecast anchored to today's date.
/// Five days are produced so that all twelve Condition variants appear at
/// least once, making it easy to verify colours and icons.
///
/// Day patterns (by offset):
///   0 – clear/mostly-clear day  (Clear, MostlyClear, PartlyCloudy)
///   1 – rain event               (Drizzle → Rain → RainShowers → Thunderstorm)
///   2 – foggy morning, overcast  (Fog, Overcast, PartlyCloudy)
///   3 – winter / snow day        (Snow, SnowShowers, Sleet, Overcast)
///   4 – rain event (same as 1)
pub fn generate_fake_forecast(num_days: usize) -> Vec<WeatherDay> {
    let today = Local::now().date_naive();
    let hours: &[u8] = &[2, 5, 8, 11, 14, 17, 20, 23];
    let mut days = Vec::with_capacity(num_days);

    for day_offset in 0..num_days {
        let date = today + Duration::days(day_offset as i64);
        let d = day_offset as f32;

        let base_pressure = 1013.0 + 8.0 * (d * 0.7).cos();

        // Winter day gets colder temperatures
        let base_temp = if day_offset % 5 == 3 {
            -3.0 + 4.0 * (d * 1.3).sin()
        } else {
            18.0 + 6.0 * (d * 1.3).sin()
        };

        let is_rain_day  = day_offset % 3 == 1;
        let is_fog_day   = day_offset % 5 == 2;
        let is_snow_day  = day_offset % 5 == 3;

        // Rain-event Gaussian: peak shifts day to day, spans 0.0 → ~16 mm.
        let rain_peak = 12.0 + 2.5 * (d * 0.8).sin();

        let entries = hours
            .iter()
            .map(|&hour| {
                let h = hour as f32;

                let time_factor = (-(((h - 14.0) / 8.0).powi(2))).exp();
                let temp = base_temp - 5.0 + 10.0 * time_factor;

                let wind_speed = 5.0 + 8.0 * ((h * 0.4 + d * 1.1).sin().abs());
                let wind_gust  = wind_speed + 5.0 + 3.0 * ((h * 0.7).cos().abs());
                let wind_dir   = WindDir::from_index((hour as usize / 3 + day_offset * 2) % 8);
                let pressure   = base_pressure + 3.0 * ((h * 0.2).sin());

                let x      = (h - rain_peak) / 3.5;
                let precip = if is_rain_day { (16.0 * (-x * x).exp()).max(0.0) } else { 0.0 };

                let condition = if is_rain_day {
                    if precip > 14.0 {
                        Condition::Thunderstorm
                    } else if precip > 8.0 {
                        Condition::RainShowers
                    } else if precip > 3.0 {
                        Condition::Rain
                    } else if precip > 0.5 {
                        Condition::Drizzle
                    } else {
                        Condition::Overcast
                    }
                } else if is_fog_day {
                    if hour < 11 {
                        Condition::Fog
                    } else if wind_speed > 12.0 {
                        Condition::PartlyCloudy
                    } else {
                        Condition::Overcast
                    }
                } else if is_snow_day {
                    if wind_speed > 13.0 {
                        Condition::SnowShowers
                    } else if temp < -1.0 {
                        Condition::Snow
                    } else {
                        Condition::Sleet
                    }
                } else {
                    // Clear day — vary with wind
                    if wind_speed > 13.0 {
                        Condition::PartlyCloudy
                    } else if wind_speed > 8.0 {
                        Condition::MostlyClear
                    } else {
                        Condition::Clear
                    }
                };

                WeatherEntry {
                    hour,
                    temp_c: temp,
                    wind_dir,
                    wind_speed_kmh: wind_speed,
                    wind_gust_kmh: wind_gust,
                    pressure_hpa: pressure,
                    precip_mm: precip,
                    condition,
                }
            })
            .collect();

        days.push(WeatherDay { date, entries });
    }

    days
}
