//! Optional side-graphs rendered in the right margin.
//!
//! The forecast table is tall and narrow, so each metric is drawn as a
//! full-height panel aligned row-for-row with the table:
//!   • vertical axis = time   (one anchor per forecast row, matching each line)
//!   • horizontal axis = the metric value (low → left, high → right, auto-scaled)
//! Lines are drawn with Unicode braille dots (8 sub-cells per character, as in
//! uplot) for a smooth curve. Temperature and rain reuse the same colour
//! gradient as their table columns; pressure has no gradient so it uses a
//! single neutral hue.
//!
//! As many panels as fit are shown, side by side, in priority order
//! (temperature, pressure, rain). They are only drawn when the terminal is
//! wide enough; otherwise the lines are left untouched.

use crate::color::gradient_index_for;
use crate::config::Config;

/// Role of an output line, deciding what (if anything) the graphs attach to it.
pub enum Role {
    /// Printed verbatim, never decorated (e.g. the location line).
    Plain,
    /// First header/footer line — receives the graph titles.
    GraphTitle,
    /// Second header/footer line — receives the axis scales.
    GraphAxis,
    /// A data-region line (forecast row or day separator) — receives graph cells.
    Body,
}

/// The graphable values of a forecast row.
pub struct Metrics {
    pub temp: f32,
    pub pressure: f32,
    pub precip: f32,
}

/// One line of output plus the metadata the graphs need.
pub struct OutLine {
    pub text: String,
    /// Metric values for forecast rows; `None` for separators/headers.
    pub metrics: Option<Metrics>,
    pub role: Role,
}

const GAP: usize = 2; // blank columns between the table and the first graph
const GRAPH_GAP: usize = 2; // blank columns between adjacent graphs
const MIN_W: usize = 14; // minimum width (chars) for one graph to be worth drawing
const MAX_W: usize = 40; // cap so a graph stays tidy on very wide terminals

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

/// How a series' line is coloured per row.
enum SeriesColor {
    /// 256-colour gradient over [`min`, `max`] (matches the table column).
    Gradient { min: f32, max: f32 },
    /// A single fixed 256-colour index.
    Fixed(u8),
}

/// A metric to plot: its titles (longest-that-fits is used), values, colour
/// scheme and axis-label unit.
struct Series {
    titles: Vec<&'static str>,
    values: Vec<Option<f32>>,
    color: SeriesColor,
    unit: &'static str,
}

/// A rendered plot: braille cells, per-row colour and the axis extremes.
struct Plot {
    cells: Vec<u8>,   // n rows × w columns of braille bitmasks
    color: Vec<u8>,   // per-row 256-colour index
    lo: f32,          // axis minimum (data min)
    hi: f32,          // axis maximum (data max)
}

/// Attach the side-graphs to `lines` in place, when the terminal allows.
pub fn decorate(lines: &mut [OutLine], config: &Config) {
    let cols = match terminal_cols() {
        Some(c) => c,
        None => return, // not a terminal (piped) and no override → no graphs
    };

    // The table width is the widest forecast row (all rows share one width).
    let body_width = match lines
        .iter()
        .filter(|l| matches!(l.role, Role::Body) && l.metrics.is_some())
        .map(|l| display_width(&l.text))
        .max()
    {
        Some(w) => w,
        None => return,
    };

    let graph_col = body_width + GAP;
    if cols <= graph_col + MIN_W {
        return; // terminal too narrow for even one graph
    }
    let avail = cols - graph_col;

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

    // Per-body-line metric values (None for the day separators).
    let values = |pick: fn(&Metrics) -> f32| -> Vec<Option<f32>> {
        body_idx
            .iter()
            .map(|&i| lines[i].metrics.as_ref().map(pick))
            .collect()
    };
    let fr = config.language == "fr";
    let series = [
        Series {
            titles: if fr {
                vec!["Température (°C)", "Temp. °C", "°C"]
            } else {
                vec!["Temperature (°C)", "Temp. °C", "°C"]
            },
            values: values(|m| m.temp),
            color: SeriesColor::Gradient {
                min: config.temperature.min,
                max: config.temperature.max,
            },
            unit: "°",
        },
        Series {
            titles: if fr {
                vec!["Pression Athm.", "Pression", "hPa"]
            } else {
                vec!["Athm. Pressure", "Pressure", "hPa"]
            },
            values: values(|m| m.pressure),
            color: SeriesColor::Fixed(250),
            unit: "",
        },
        Series {
            titles: if fr {
                vec!["Pluie (mm)", "Pluie", "mm"]
            } else {
                vec!["Rain (mm)", "Rain", "mm"]
            },
            values: values(|m| m.precip),
            color: SeriesColor::Gradient {
                min: config.water.min,
                max: config.water.max,
            },
            unit: "",
        },
    ];

    // How many graphs fit side by side, and their shared width.
    let mut count = 0;
    for c in 1..=series.len() {
        if c * MIN_W + (c - 1) * GRAPH_GAP <= avail {
            count = c;
        } else {
            break;
        }
    }
    let w = ((avail - (count - 1) * GRAPH_GAP) / count).min(MAX_W);

    // Build the plots for the chosen series.
    let plots: Vec<(Plot, &Series)> = series
        .iter()
        .take(count)
        .filter_map(|s| build_plot(&s.values, n, w, &s.color).map(|p| (p, s)))
        .collect();
    if plots.is_empty() {
        return;
    }

    // Compose: append the graph strip / titles / axes to each line.
    let mut k = 0;
    for line in lines.iter_mut() {
        let right = match line.role {
            Role::Body => {
                let s = join(&plots, GRAPH_GAP, |(plot, _), is_last| {
                    row_string(plot, k, w, is_last)
                });
                k += 1;
                s
            }
            Role::GraphTitle => join(&plots, GRAPH_GAP, |(_, s), _| {
                center(pick_title(&s.titles, w), w)
            }),
            Role::GraphAxis => join(&plots, GRAPH_GAP, |(plot, s), _| {
                let lo = format!("{:.0}{}", plot.lo, s.unit);
                let hi = format!("{:.0}{}", plot.hi, s.unit);
                axis(&lo, &hi, w)
            }),
            Role::Plain => continue,
        };
        append_right(&mut line.text, graph_col, &right);
    }
}

/// Join each plot's segment with `gap` spaces, flagging the final one.
fn join<F>(plots: &[(Plot, &Series)], gap: usize, mut seg: F) -> String
where
    F: FnMut(&(Plot, &Series), bool) -> String,
{
    let last = plots.len() - 1;
    let mut s = String::new();
    for (i, p) in plots.iter().enumerate() {
        if i > 0 {
            s.push_str(&" ".repeat(gap));
        }
        s.push_str(&seg(p, i == last));
    }
    s
}

/// Pad `text` to `col` then append `right`, trimming trailing blank padding.
fn append_right(text: &mut String, col: usize, right: &str) {
    pad_to(text, col);
    text.push_str(right);
    // Colour resets / braille glyphs do not end in whitespace, so this only
    // strips the spacer/centering padding — never visible graph content.
    let end = text.trim_end().len();
    text.truncate(end);
}

/// Render row `k` of `plot` to a coloured braille string `w` columns wide.
/// Blank cells are plain spaces; the final graph in a row drops its trailing
/// blanks so lines do not carry invisible padding to the terminal edge.
fn row_string(plot: &Plot, k: usize, w: usize, is_last: bool) -> String {
    let row = &plot.cells[k * w..(k + 1) * w];
    let (first, last) = match (row.iter().position(|&b| b != 0), row.iter().rposition(|&b| b != 0)) {
        (Some(f), Some(l)) => (f, l),
        _ => return if is_last { String::new() } else { " ".repeat(w) },
    };

    let mut s = " ".repeat(first); // leading blank cells
    s.push_str(&format!("\x1b[38;5;{}m", plot.color[k]));
    for &bits in &row[first..=last] {
        s.push(char::from_u32(0x2800 + bits as u32).unwrap());
    }
    s.push_str("\x1b[0m");
    if !is_last {
        s.push_str(&" ".repeat(w - 1 - last)); // trailing blank cells
    }
    s
}

/// Plot a series into an `n`×`w` braille canvas, or `None` if it lacks data.
fn build_plot(values: &[Option<f32>], n: usize, w: usize, color: &SeriesColor) -> Option<Plot> {
    // Anchors: (row index within the body, value).
    let anchors: Vec<(usize, f32)> = values
        .iter()
        .enumerate()
        .filter_map(|(k, v)| v.map(|x| (k, x)))
        .collect();
    if anchors.len() < 2 {
        return None;
    }

    let dmin = anchors.iter().map(|&(_, v)| v).fold(f32::INFINITY, f32::min);
    let dmax = anchors.iter().map(|&(_, v)| v).fold(f32::NEG_INFINITY, f32::max);
    // Auto-scale the horizontal axis; a near-flat series sits against a
    // one-unit scale from its minimum (so a flat line hugs the left edge).
    let (lo, hi) = if dmax - dmin < 0.5 {
        (dmin, dmin + 1.0)
    } else {
        (dmin, dmax)
    };

    let dots_w = (w * 2) as f32;
    let x_of = |v: f32| -> i32 {
        (((v - lo) / (hi - lo)) * (dots_w - 1.0))
            .round()
            .clamp(0.0, dots_w - 1.0) as i32
    };

    let mut cells = vec![0u8; n * w];
    let mut val_at = vec![lo; n]; // per-row value for colouring, interpolated

    const C: i32 = 1; // dot-row offset of an anchor inside its 4-row cell
    for win in anchors.windows(2) {
        let (k0, t0) = win[0];
        let (k1, t1) = win[1];
        let y0 = k0 as i32 * 4 + C;
        let y1 = k1 as i32 * 4 + C;

        let mut prev_x = x_of(t0);
        for y in y0..=y1 {
            let f = (y - y0) as f32 / (y1 - y0) as f32;
            let x = x_of(t0 + (t1 - t0) * f);
            // Bridge horizontally so steep slopes stay an unbroken line.
            let (a, b) = if prev_x <= x { (prev_x, x) } else { (x, prev_x) };
            for xx in a..=b {
                set_dot(&mut cells, w, n, xx, y);
            }
            prev_x = x;
        }

        for k in k0..=k1 {
            let f = (k - k0) as f32 / (k1 - k0) as f32;
            val_at[k] = t0 + (t1 - t0) * f;
        }
    }

    let color = (0..n)
        .map(|k| match color {
            SeriesColor::Gradient { min, max } => gradient_index_for(val_at[k], *min, *max),
            SeriesColor::Fixed(c) => *c,
        })
        .collect();

    Some(Plot { cells, color, lo: dmin, hi: dmax })
}

/// Set the braille dot at absolute dot-coordinates (`dx`, `dy`).
fn set_dot(cells: &mut [u8], w: usize, n: usize, dx: i32, dy: i32) {
    if dx < 0 || dy < 0 {
        return;
    }
    let (dx, dy) = (dx as usize, dy as usize);
    let (cell_row, cell_col) = (dy / 4, dx / 2);
    if cell_row >= n || cell_col >= w {
        return;
    }
    cells[cell_row * w + cell_col] |= BRAILLE[dy % 4][dx % 2];
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

/// Longest title variant that fits within `width` columns (else empty).
fn pick_title(titles: &[&'static str], width: usize) -> &'static str {
    titles
        .iter()
        .copied()
        .find(|t| t.chars().count() <= width)
        .unwrap_or("")
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
