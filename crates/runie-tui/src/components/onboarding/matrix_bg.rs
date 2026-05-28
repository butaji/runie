use rand::Rng;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::cell::{Cell as StdCell, RefCell};

// ── Visible gray palette — must contrast with dark terminal bg ────────────
const GRAYS: [Color; 5] = [
    Color::Rgb(70, 70, 85),   // far / dim
    Color::Rgb(100, 100, 118),// mid-dim
    Color::Rgb(135, 135, 155),// mid
    Color::Rgb(175, 175, 195),// bright
    Color::Rgb(210, 210, 230),// near / bold
];

// ── ASCII art glyph definitions ───────────────────────────────────────────
fn glyph_large_1() -> &'static [&'static str] {
    &[
        "  _____  ",
        " | _   | ",
        " |.|   | ",
        " `-|.  | ",
        "   |:  | ",
        "   |::.| ",
        "   `---' ",
    ]
}

fn glyph_large_0() -> &'static [&'static str] {
    &[
        " _______ ",
        "|   _   |",
        "|.  |   |",
        "|.  |   |",
        "|:  |   |",
        "|::.. . |",
        "`-------'",
    ]
}

fn glyph_med_1() -> &'static [&'static str] {
    &[
        "  ___  ",
        " | _ | ",
        " |.| | ",
        " `-|.| ",
        "   `-' ",
    ]
}

fn glyph_med_0() -> &'static [&'static str] {
    &[
        " _____ ",
        "|  _  |",
        "|.| | |",
        "|::.| |",
        "`-----'",
    ]
}

fn glyph_small_1() -> &'static [&'static str] {
    &[" _ ", "|.|", "`-|"]
}

fn glyph_small_0() -> &'static [&'static str] {
    &[" _ ", "| |", "|_|"]
}

#[derive(Clone, Copy, Debug)]
enum GlyphSize {
    Large,  // 9×7
    Medium, // 7×5
    Small,  // 3×3
    Tiny,   // 1×1
}

impl GlyphSize {
    fn dims(self) -> (u16, u16) {
        match self {
            GlyphSize::Large => (9, 7),
            GlyphSize::Medium => (7, 5),
            GlyphSize::Small => (3, 3),
            GlyphSize::Tiny => (1, 1),
        }
    }

    fn lines(self, is_one: bool) -> &'static [&'static str] {
        match (self, is_one) {
            (GlyphSize::Large, true) => glyph_large_1(),
            (GlyphSize::Large, false) => glyph_large_0(),
            (GlyphSize::Medium, true) => glyph_med_1(),
            (GlyphSize::Medium, false) => glyph_med_0(),
            (GlyphSize::Small, true) => glyph_small_1(),
            (GlyphSize::Small, false) => glyph_small_0(),
            (GlyphSize::Tiny, true) => &["1"],
            (GlyphSize::Tiny, false) => &["0"],
        }
    }
}

// ── Placed glyph ──────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
struct Glyph {
    x: u16,
    y: u16,
    size: GlyphSize,
    is_one: bool,
    bright: usize,
}

// ── Background widget ─────────────────────────────────────────────────────
#[derive(Debug)]
pub struct MatrixBg {
    glyphs: RefCell<Vec<Glyph>>,
    cw: StdCell<u16>,
    ch: StdCell<u16>,
}

impl Clone for MatrixBg {
    fn clone(&self) -> Self {
        Self {
            glyphs: RefCell::new(self.glyphs.borrow().clone()),
            cw: StdCell::new(self.cw.get()),
            ch: StdCell::new(self.ch.get()),
        }
    }
}

impl MatrixBg {
    pub fn new(w: u16, h: u16) -> Self {
        Self {
            glyphs: RefCell::new(Self::fill(w, h)),
            cw: StdCell::new(w),
            ch: StdCell::new(h),
        }
    }

    fn fill(w: u16, h: u16) -> Vec<Glyph> {
        if w < 3 || h < 3 {
            return Vec::new();
        }
        let mut rng = rand::thread_rng();
        let mut out = Vec::new();

        // ── Layer 1: dense tiny specks filling the whole screen ──────────
        // Every ~3rd cell gets a tiny 1 or 0
        for y in (0..h).step_by(2) {
            for x in (0..w).step_by(3) {
                if rng.gen_bool(0.4) {
                    out.push(Glyph {
                        x,
                        y,
                        size: GlyphSize::Tiny,
                        is_one: rng.gen_bool(0.5),
                        bright: rng.gen_range(0..2),
                    });
                }
            }
        }

        // ── Layer 2: small 3×3 glyphs scattered ──────────────────────────
        let small_count = ((w as usize * h as usize) / 40).max(4);
        for _ in 0..small_count {
            let x = rng.gen_range(0..w.saturating_sub(3));
            let y = rng.gen_range(0..h.saturating_sub(3));
            out.push(Glyph {
                x,
                y,
                size: GlyphSize::Small,
                is_one: rng.gen_bool(0.5),
                bright: rng.gen_range(1..3),
            });
        }

        // ── Layer 3: medium 7×5 glyphs ───────────────────────────────────
        // More toward bottom for perspective
        let med_count = ((w as usize * h as usize) / 120).max(2);
        for _ in 0..med_count {
            let x = rng.gen_range(0..w.saturating_sub(7));
            let y = rng.gen_range((h / 3)..h.saturating_sub(5));
            out.push(Glyph {
                x,
                y,
                size: GlyphSize::Medium,
                is_one: rng.gen_bool(0.5),
                bright: rng.gen_range(2..4),
            });
        }

        // ── Layer 4: large 9×7 glyphs ────────────────────────────────────
        // Only near bottom, few and proud
        let large_count = ((w as usize * h as usize) / 350).max(1);
        for _ in 0..large_count {
            let x = rng.gen_range(0..w.saturating_sub(9));
            let y = rng.gen_range((h * 2 / 3)..h.saturating_sub(7));
            out.push(Glyph {
                x,
                y,
                size: GlyphSize::Large,
                is_one: rng.gen_bool(0.5),
                bright: rng.gen_range(3..5),
            });
        }

        out
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        self.ensure_size(area);
        Self::fill_bg(area, buf);
        for g in self.glyphs.borrow().iter() {
            Self::draw_glyph(area, buf, g);
        }
    }

    fn ensure_size(&self, area: Rect) {
        if self.cw.get() == area.width && self.ch.get() == area.height {
            return;
        }
        let new = Self::new(area.width, area.height);
        *self.glyphs.borrow_mut() = new.glyphs.into_inner();
        self.cw.set(area.width);
        self.ch.set(area.height);
    }

    fn fill_bg(area: Rect, buf: &mut Buffer) {
        let bg = Color::Rgb(12, 12, 18);
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(" ");
                    cell.set_fg(bg);
                    cell.set_bg(bg);
                }
            }
        }
    }

    fn draw_glyph(area: Rect, buf: &mut Buffer, g: &Glyph) {
        let bg = Color::Rgb(12, 12, 18);
        let lines = g.size.lines(g.is_one);
        for (ri, line) in lines.iter().enumerate() {
            let y = area.y + g.y + ri as u16;
            if y >= area.bottom() {
                continue;
            }
            for (ci, ch) in line.chars().enumerate() {
                let x = area.x + g.x + ci as u16;
                if x >= area.right() || ch == ' ' {
                    continue;
                }
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_symbol(&ch.to_string());
                    cell.set_fg(GRAYS[g.bright.min(GRAYS.len() - 1)]);
                    cell.set_bg(bg);
                }
            }
        }
    }
}
