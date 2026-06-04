//! `bin/runie-dspec` — declarative spec renderer for TUI screens.
//!
//! `dspec` is a JSON DSL that describes a TUI screen using the same
//! `paint` primitives (Text, Row, Col, Border, Pad, Fill, If, Blank).
//! Loading + rendering is sub-10ms, so the iteration loop is:
//!
//!   1. Edit `foo.dspec.json`
//!   2. Save
//!   3. Within 10ms, see the new render in your terminal
//!
//! This is the fast loop for TUI development. No Rust compile, no
//! scenario file to maintain, no test to run. The dspec IS the spec
//! and the rendered output IS the verification.
//!
//! Usage:
//!   `runie-dspec path/to/spec.dspec.json [--diff grok.txt] [--width W] [--height H]`
//!
//! Example spec:
//! ```json
//! {
//!   "width": 78, "height": 24,
//!   "root": { "row": {
//!     "children": [
//!       { "text": " feat/grok-redesign", "style": "accent" },
//!       { "fill": true },
//!       { "text": "│ 20K / 512K │", "style": "dim" }
//!     ]
//!   }}
//! }
//! ```

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use runie_tui::paint::{paint, text, Node, StyleRef};
use runie_tui::ThemeWrapper;
use runie_tui::theme::ThemeColors;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[derive(serde::Deserialize)]
struct Dspec {
    width: u16,
    height: u16,
    root: Dnode,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Dnode {
    Text { content: String, #[serde(default)] style: Option<String>, #[serde(default)] right_inset: u16 },
    Row {
        #[serde(default)] gap: u16,
        #[serde(default)] fill_trailing: bool,
        children: Vec<Dnode>,
    },
    Col {
        #[serde(default)] gap: u16,
        children: Vec<Dnode>,
    },
    Border { style: Option<String>, child: Box<Dnode> },
    Pad { top: u16, right: u16, bottom: u16, left: u16, child: Box<Dnode> },
    Fill,
    If { cond: bool, then: Box<Dnode> },
    Blank { style: Option<String> },
    RightGap { n: u16 },
}

impl Dnode {
    fn resolve(&self) -> Node {
        match self {
            Dnode::Text { content, style, right_inset } => {
                let mut t = text(content.clone());
                if let Some(s) = style { t = apply_style(t, s); }
                if *right_inset > 0 { t = t.right_inset(*right_inset); }
                Node::T(t)
            }
            Dnode::Row { gap, fill_trailing, children } => {
                let ns: Vec<Node> = children.iter().map(|c| c.resolve()).collect();
                let mut r = runie_tui::paint::row(ns).gap(*gap);
                if *fill_trailing { r = r.fill_trailing(); }
                r.build()
            }
            Dnode::Col { gap, children } => {
                let ns: Vec<Node> = children.iter().map(|c| c.resolve()).collect();
                runie_tui::paint::col(ns).gap(*gap).build()
            }
            Dnode::Border { style, child } => {
                let mut b = runie_tui::paint::border(child.resolve());
                if let Some(s) = style { b = b.style(parse_style(s)); }
                b.build()
            }
            Dnode::Pad { top, right, bottom, left, child } => {
                let mut p = runie_tui::paint::pad(child.resolve());
                p.top = *top; p.right = *right; p.bottom = *bottom; p.left = *left;
                p.build()
            }
            Dnode::Fill => Node::Fill,
            Dnode::If { cond, then } => Node::If { cond: *cond, then: Box::new(then.resolve()) },
            Dnode::Blank { style } => {
                let s = style.as_deref().map(parse_style).unwrap_or(StyleRef::BgBase);
                Node::Blank { bg: s }
            }
            Dnode::RightGap { n } => Node::RightGap { n: *n },
        }
    }
}

fn parse_style(s: &str) -> StyleRef {
    match s {
        "primary" => StyleRef::Primary,
        "secondary" => StyleRef::Secondary,
        "dim" => StyleRef::Dim,
        "muted" => StyleRef::Muted,
        "accent" => StyleRef::Accent,
        "bg_panel" => StyleRef::BgPanel,
        "bg_base" => StyleRef::BgBase,
        _ => StyleRef::Dim,
    }
}

fn apply_style(t: runie_tui::paint::Text, s: &str) -> runie_tui::paint::Text {
    match s {
        "primary" => t.primary(),
        "secondary" => t.secondary(),
        "dim" => t.dim(),
        "muted" => t.muted(),
        "accent" => t.accent(),
        _ => t,
    }
}

fn render(spec: &Dspec) -> String {
    let backend = TestBackend::new(spec.width, spec.height);
    let mut terminal = Terminal::new(backend).unwrap();
    let wrapper = ThemeWrapper::crush_grok();
    let theme = ThemeColors::from(&wrapper);
    let node = spec.root.resolve();
    terminal
        .draw(|frame| {
            paint(&node, frame.area(), frame.buffer_mut(), &theme);
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut s = String::with_capacity((spec.width as usize + 1) * spec.height as usize);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            s.push_str(buf.cell((x, y)).unwrap().symbol());
        }
        s.push('\n');
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: runie-dspec <spec.dspec.json> [--diff <grok.txt>]");
        std::process::exit(2);
    }
    let spec_path = PathBuf::from(&args[1]);
    let spec_src = fs::read_to_string(&spec_path).expect("read spec");
    let spec: Dspec = serde_json::from_str(&spec_src).expect("parse spec");
    let rendered = render(&spec);

    let mut diff_target: Option<PathBuf> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--diff" => { diff_target = Some(PathBuf::from(&args[i + 1])); i += 2; }
            _ => i += 1,
        }
    }
    match diff_target {
        Some(path) => {
            let ref_src = fs::read_to_string(&path).expect("read ref");
            // line-by-line diff with trim_end
            let a: Vec<&str> = rendered.lines().collect();
            let b: Vec<&str> = ref_src.lines().collect();
            let mut diffs = 0;
            for (idx, (lhs, rhs)) in a.iter().zip(b.iter()).enumerate() {
                let l = lhs.trim_end();
                let r = rhs.trim_end();
                if l != r {
                    diffs += 1;
                    println!("line {idx}:\n  - ref: |{r}|\n  + got: |{l}|");
                }
            }
            if a.len() != b.len() {
                println!("length differs: got {} lines, ref {} lines", a.len(), b.len());
            }
            if diffs == 0 { println!("✓ dspec matches reference"); }
            else { println!("{diffs} line(s) differ"); std::process::exit(1); }
        }
        None => {
            print!("{rendered}");
            std::io::stdout().flush().ok();
        }
    }
}
