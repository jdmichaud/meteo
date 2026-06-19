pub fn check_256_colors() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if let Ok(ct) = std::env::var("COLORTERM") {
        let ct = ct.to_lowercase();
        if ct == "truecolor" || ct == "24bit" || ct == "256color" {
            return true;
        }
    }
    if let Ok(term) = std::env::var("TERM") {
        if term.contains("256color") {
            return true;
        }
        let known: &[&str] = &[
            "xterm", "screen", "tmux", "rxvt", "konsole",
            "alacritty", "kitty", "foot", "wezterm", "vte",
        ];
        if known.iter().any(|p| term.starts_with(p)) {
            return true;
        }
    }
    // Windows 10+ console supports 256 colours natively once VT processing is enabled
    cfg!(target_os = "windows")
}

#[cfg(target_os = "windows")]
pub fn enable_windows_ansi() {
    use windows_sys::Win32::System::Console::{
        GetConsoleMode, GetStdHandle, SetConsoleMode,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
    };
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

// Maps t ∈ [0,1] to a 256-colour index using the 6×6×6 cube (16–231).
// Gradient: deep blue (21) → green (46) → deep red (196).
fn gradient_index(t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    let (r, g, b): (u8, u8, u8) = if t <= 0.5 {
        let u = t * 2.0;
        let b_val = ((1.0 - u) * 5.0 + 0.5) as u8;
        let g_val = (u * 5.0 + 0.5) as u8;
        (0, g_val.min(5), b_val.min(5))
    } else {
        let u = (t - 0.5) * 2.0;
        let g_val = ((1.0 - u) * 5.0 + 0.5) as u8;
        let r_val = (u * 5.0 + 0.5) as u8;
        (r_val.min(5), g_val.min(5), 0)
    };
    16 + 36 * r + 6 * g + b
}

/// Wraps `text` in 256-colour background escape sequences.
///
/// `zero_no_color`: when true, a value of exactly 0.0 is rendered without colour.
pub fn colored_field(text: &str, value: f32, min: f32, max: f32, zero_no_color: bool) -> String {
    if zero_no_color && value == 0.0 {
        return text.to_string();
    }
    if max <= min {
        return text.to_string();
    }
    let t = (value - min) / (max - min);
    let bg = gradient_index(t);
    // Black foreground in the green zone (bright background), white elsewhere.
    let fg = if t > 0.3 && t < 0.7 { "\x1b[30m" } else { "\x1b[97m" };
    format!("\x1b[48;5;{}m{}{}\x1b[0m", bg, fg, text)
}
