//! Optional temperature side-graph rendered in the right margin.
//!
//! The forecast table is tall and narrow, so the graph is laid out as a
//! full-height side panel aligned row-for-row with the table:
//!   • vertical axis = time   (one anchor per forecast row, matching each line)
//!   • horizontal axis = temperature (cold → left, hot → right, auto-scaled)
//! The line is drawn with Unicode braille dots (8 sub-cells per character, as
//! in uplot) for a smooth curve, and coloured with the same gradient as the
//! temperature column so a given temperature shares its hue in both places.
//!
//! It is only drawn when the terminal is wide enough; otherwise the lines are
//! left untouched and the output is identical to the no-graph layout.

use crate::color::gradient_index_for;
use crate::config::Config;

/// Role of an output line, deciding what (if anything) the graph attaches to it.
pub enum Role {
    /// Printed verbatim, never decorated (e.g. the location line).
    Plain,
    /// First header line — receives the graph title.
    GraphTitle,
    /// Second header line — receives the temperature axis scale.
    GraphAxis,
    /// A data-region line (forecast row or day separator) — receives graph cells.
    Body,
}

/// One line of output plus the metadata the graph needs.
pub struct OutLine {
    pub text: String,
    /// Temperature for forecast rows; `None` for separators/headers.
    pub temp: Option<f32>,
    pub role: Role,
}

const GAP: usize = 2; // blank columns between the table and the graph
const MIN_W: usize = 14; // minimum graph width (chars) worth drawing
const MAX_W: usize = 40; // cap so the graph stays tidy on very wide terminals

// Braille dot bit for sub-cell (cx ∈ 0..2 columns, cy ∈ 0..4 rows).
// Unicode dot numbering and bit values:
//   1(0x01) 4(0x08)
//   2(0x02) 5(0x10)
//   3(0x04) 6(0x20)
//   7(0x40) 8(0x80)
const BRAILLE: [[u8; 2]; 4] = [
    [0x01, 0x08],
    [0x02, 0x10],
    [0x04, 0x20],
    [0x40, 0x80],
];

/// Attach the temperature graph to `lines` in place, when the terminal allows.
pub fn decorate(lines: &mut [OutLine], config: &Config) {
    let cols = match terminal_cols() {
        Some(c) => c,
        None => return, // not a terminal (piped) and no override → no graph
    };

    // The table width is the widest forecast row (all rows share one width).
    let body_width = match lines
        .iter()
        .filter(|l| matches!(l.role, Role::Body) && l.temp.is_some())
        .map(|l| display_width(&l.text))
        .max()
    {
        Some(w) => w,
        None => return,
    };

    let graph_col = body_width + GAP;
    if cols <= graph_col + MIN_W {
        return; // terminal too narrow to be worth it
    }
    let graph_w = (cols - graph_col).min(MAX_W);

    // Body lines, in printed order (forecast rows and the day separators).
    let body_idx: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| matches!(l.role, Role::Body))
        .map(|(i, _)| i)
        .collect();
    let n = body_idx.len();
    if n < 2 {
        return;
    }

    // Anchors: (row index within the body, temperature).
    let anchors: Vec<(usize, f32)> = body_idx
        .iter()
        .enumerate()
        .filter_map(|(k, &i)| lines[i].temp.map(|t| (k, t)))
        .collect();
    if anchors.len() < 2 {
        return;
    }

    // Auto-scale the horizontal (temperature) axis to the data.
    let tmin = anchors.iter().map(|&(_, t)| t).fold(f32::INFINITY, f32::min);
    let tmax = anchors
        .iter()
        .map(|&(_, t)| t)
        .fold(f32::NEG_INFINITY, f32::max);
    let (lo, hi) = if tmax - tmin < 0.5 {
        (tmin - 0.5, tmin + 0.5)
    } else {
        (tmin, tmax)
    };

    let dots_w = (graph_w * 2) as f32;
    let x_of = |t: f32| -> i32 {
        (((t - lo) / (hi - lo)) * (dots_w - 1.0))
            .round()
            .clamp(0.0, dots_w - 1.0) as i32
    };

    // Braille cell bitmasks: n rows × graph_w columns.
    let mut canvas = vec![0u8; n * graph_w];
    // Per-row temperature for colouring, interpolated between anchors.
    let mut temp_at = vec![lo; n];

    const C: i32 = 1; // dot-row offset of an anchor inside its 4-row cell
    for w in anchors.windows(2) {
        let (k0, t0) = w[0];
        let (k1, t1) = w[1];
        let y0 = k0 as i32 * 4 + C;
        let y1 = k1 as i32 * 4 + C;

        let mut prev_x = x_of(t0);
        for y in y0..=y1 {
            let f = (y - y0) as f32 / (y1 - y0) as f32;
            let x = x_of(t0 + (t1 - t0) * f);
            // Bridge horizontally so steep slopes stay an unbroken line.
            let (a, b) = if prev_x <= x { (prev_x, x) } else { (x, prev_x) };
            for xx in a..=b {
                set_dot(&mut canvas, graph_w, n, xx, y);
            }
            prev_x = x;
        }

        for k in k0..=k1 {
            let f = (k - k0) as f32 / (k1 - k0) as f32;
            temp_at[k] = t0 + (t1 - t0) * f;
        }
    }

    // Render each canvas row to a coloured braille string (trailing blanks trimmed).
    let strip: Vec<String> = (0..n)
        .map(|k| {
            let row = &canvas[k * graph_w..(k + 1) * graph_w];
            let last = row.iter().rposition(|&b| b != 0).map(|p| p + 1).unwrap_or(0);
            if last == 0 {
                return String::new();
            }
            let idx = gradient_index_for(temp_at[k], config.temperature.min, config.temperature.max);
            let mut s = format!("\x1b[38;5;{}m", idx);
            for &bits in &row[..last] {
                s.push(char::from_u32(0x2800 + bits as u32).unwrap());
            }
            s.push_str("\x1b[0m");
            s
        })
        .collect();

    // Header decorations: title above the graph, temperature scale below it.
    // Pick the longest title variant that fits the graph width.
    let titles: [&str; 3] = if config.language == "fr" {
        ["Température (°C)", "Temp. °C", "°C"]
    } else {
        ["Temperature (°C)", "Temp. °C", "°C"]
    };
    let title = titles
        .iter()
        .find(|t| t.chars().count() <= graph_w)
        .copied()
        .unwrap_or("");
    let lo_lbl = format!("{}°", tmin.round() as i32);
    let hi_lbl = format!("{}°", tmax.round() as i32);

    let mut k = 0;
    for line in lines.iter_mut() {
        match line.role {
            Role::Body => {
                if !strip[k].is_empty() {
                    pad_to(&mut line.text, graph_col);
                    line.text.push_str(&strip[k]);
                }
                k += 1;
            }
            Role::GraphTitle => {
                pad_to(&mut line.text, graph_col);
                line.text.push_str(&center(title, graph_w));
            }
            Role::GraphAxis => {
                pad_to(&mut line.text, graph_col);
                line.text.push_str(&axis(&lo_lbl, &hi_lbl, graph_w));
            }
            Role::Plain => {}
        }
    }
}

/// Set the braille dot at absolute dot-coordinates (`dx`, `dy`).
fn set_dot(canvas: &mut [u8], graph_w: usize, n: usize, dx: i32, dy: i32) {
    if dx < 0 || dy < 0 {
        return;
    }
    let (dx, dy) = (dx as usize, dy as usize);
    let (cell_row, cell_col) = (dy / 4, dx / 2);
    if cell_row >= n || cell_col >= graph_w {
        return;
    }
    canvas[cell_row * graph_w + cell_col] |= BRAILLE[dy % 4][dx % 2];
}

/// Terminal width: `METEO_COLS` override first (handy for tests/screenshots),
/// then the real terminal size, else `None` (e.g. piped output).
fn terminal_cols() -> Option<usize> {
    if let Ok(v) = std::env::var("METEO_COLS") {
        if let Ok(n) = v.trim().parse::<usize>() {
            return Some(n);
        }
    }
    terminal_size::terminal_size().map(|(w, _)| w.0 as usize)
}

/// Display width of `s`, skipping ANSI SGR escapes and counting wide glyphs as 2.
fn display_width(s: &str) -> usize {
    let mut w = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip a CSI sequence up to and including its final letter.
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        w += char_width(c);
    }
    w
}

/// 2 for the wide weather emoji/symbols used as condition icons, 1 otherwise.
/// The arrow glyphs (U+2190..21FF) and the current-row marker (U+1F81E) are 1.
fn char_width(c: char) -> usize {
    let u = c as u32;
    let wide = (0x1F300..=0x1F6FF).contains(&u)
        || (0x1F900..=0x1FAFF).contains(&u)
        || (0x2600..=0x27BF).contains(&u)
        || (0x2B00..=0x2BFF).contains(&u);
    if wide {
        2
    } else {
        1
    }
}

/// Pad `s` with spaces up to `target` display columns (no-op if already wider).
fn pad_to(s: &mut String, target: usize) {
    let w = display_width(s);
    if w < target {
        s.push_str(&" ".repeat(target - w));
    }
}

/// Centre `text` within `width` columns (truncating if it does not fit).
fn center(text: &str, width: usize) -> String {
    let tw = text.chars().count();
    if tw >= width {
        return text.chars().take(width).collect();
    }
    let left = (width - tw) / 2;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(width - tw - left))
}

/// `lo` flush-left and `hi` flush-right within `width` columns.
fn axis(lo: &str, hi: &str, width: usize) -> String {
    let used = lo.chars().count() + hi.chars().count();
    if used >= width {
        return format!("{}{}", lo, hi).chars().take(width).collect();
    }
    format!("{}{}{}", lo, " ".repeat(width - used), hi)
}
