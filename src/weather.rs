use chrono::NaiveDate;

#[derive(Debug, Clone, Copy)]
pub enum WindDir {
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
    NW,
}

impl WindDir {
    pub fn arrow(self) -> &'static str {
        match self {
            WindDir::N  => "↑",
            WindDir::NE => "↗",
            WindDir::E  => "→",
            WindDir::SE => "↘",
            WindDir::S  => "↓",
            WindDir::SW => "↙",
            WindDir::W  => "←",
            WindDir::NW => "↖",
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx % 8 {
            0 => WindDir::N,
            1 => WindDir::NE,
            2 => WindDir::E,
            3 => WindDir::SE,
            4 => WindDir::S,
            5 => WindDir::SW,
            6 => WindDir::W,
            _ => WindDir::NW,
        }
    }
}

// WMO-aligned condition set.  Covers the states returned by Open-Meteo,
// OpenWeatherMap, Météo-France, and similar APIs once their codes are mapped.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Condition {
    Clear,          // 0   – clear sky
    MostlyClear,    // 1   – mainly clear, few clouds
    PartlyCloudy,   // 2   – scattered clouds
    Overcast,       // 3   – full cloud cover
    Fog,            // 45/48 – fog / freezing fog
    Drizzle,        // 51-57 – drizzle (light→dense, possibly freezing)
    Rain,           // 61-67 – rain / freezing rain (slight→heavy)
    RainShowers,    // 80-82 – rain showers (slight→violent)
    Snow,           // 71-77 – snowfall / snow grains
    SnowShowers,    // 85-86 – snow showers
    Sleet,          // mixed rain + snow
    Thunderstorm,   // 95-99 – thunderstorm (±hail)
}

impl Condition {
    /// Unicode weather glyph for this condition.
    pub fn icon(self) -> &'static str {
        match self {
            Condition::Clear        => "☀",   // U+2600  BMP
            Condition::MostlyClear  => "🌤",  // U+1F324
            Condition::PartlyCloudy => "⛅",  // U+26C5  BMP
            Condition::Overcast     => "☁",   // U+2601  BMP
            Condition::Fog          => "🌫",  // U+1F32B
            Condition::Drizzle      => "🌦",  // U+1F326
            Condition::Rain         => "🌧",  // U+1F327
            Condition::RainShowers  => "⛆",  // U+26C6  BMP
            Condition::Snow         => "❄",   // U+2744  BMP
            Condition::SnowShowers  => "🌨",  // U+1F328
            Condition::Sleet        => "☃",   // U+2603  BMP
            Condition::Thunderstorm => "⛈",  // U+26C8  BMP
        }
    }

    /// Fixed background colour for the icon cell: (256-colour index, dark_foreground).
    pub fn bg_color(self) -> (u8, bool) {
        match self {
            // warm / sunny
            Condition::Clear        => (226, true),  // bright yellow
            Condition::MostlyClear  => (220, true),  // gold
            // cloudy
            Condition::PartlyCloudy => (117, true),  // sky blue
            Condition::Overcast     => (244, true),  // medium grey
            // obscured
            Condition::Fog          => (252, true),  // light grey
            // wet
            Condition::Drizzle      => (111, true),  // pale periwinkle
            Condition::Rain         => (33,  false), // blue
            Condition::RainShowers  => (27,  false), // deep blue
            // frozen
            Condition::Snow         => (195, true),  // very pale cyan
            Condition::SnowShowers  => (189, true),  // pale lavender
            Condition::Sleet        => (153, true),  // ice blue
            // severe
            Condition::Thunderstorm => (91,  false), // dark purple
        }
    }
}

#[derive(Debug, Clone)]
pub struct WeatherEntry {
    pub hour: u8,
    pub temp_c: f32,
    pub wind_dir: WindDir,
    pub wind_speed_kmh: f32,
    pub wind_gust_kmh: f32,
    pub pressure_hpa: f32,
    pub precip_mm: f32,
    pub condition: Condition,
}

#[derive(Debug, Clone)]
pub struct WeatherDay {
    pub date: NaiveDate,
    pub entries: Vec<WeatherEntry>,
}
