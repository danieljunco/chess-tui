//! Render every UI scene at the supported terminal sizes.
//!
//! cargo run --example snapshots [-- --out DIR] [--mono]
//!
//! Writes `<DIR>/<W>x<H>/<scene>.txt` (plain buffers for diffing) and
//! `<DIR>/index.html` (true-colour rendering for visual review).

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

use vesper_chess::app::fixtures;
use vesper_chess::ui::{self, theme::Theme};

const SIZES: [(u16, u16); 5] = [(120, 36), (100, 32), (90, 28), (80, 24), (70, 20)];

fn main() {
    let mut args = std::env::args().skip(1);
    let mut out = PathBuf::from("target/snapshots");
    let mut mono = false;
    let mut only: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--out" => out = PathBuf::from(args.next().expect("--out DIR")),
            "--mono" => mono = true,
            "--only" => only = args.next(),
            other => panic!("unknown argument {other}"),
        }
    }
    let theme = if mono { Theme::mono() } else { Theme::dark() };
    let now = Instant::now();

    let mut html = String::from(HEAD);
    for (w, h) in SIZES {
        let dir = out.join(format!("{w}x{h}"));
        fs::create_dir_all(&dir).expect("create snapshot dir");
        for (name, app) in fixtures::scenes(now) {
            if only.as_deref().is_some_and(|o| o != name) {
                continue;
            }
            let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("terminal");
            terminal.draw(|f| ui::draw(f, &app, &theme)).expect("draw");
            let buf = terminal.backend().buffer().clone();
            fs::write(dir.join(format!("{name}.txt")), to_text(&buf)).expect("write txt");
            let page = format!("{HEAD}{}</body></html>", to_html(&buf));
            fs::write(dir.join(format!("{name}.html")), page).expect("write html");
            let _ = write!(
                html,
                "<section id=\"{name}-{w}x{h}\"><h2>{name} · {w}×{h}</h2>{}</section>",
                to_html(&buf)
            );
        }
    }
    html.push_str("</body></html>");
    fs::write(out.join("index.html"), html).expect("write html");
    println!("snapshots written to {}", out.display());
}

fn to_text(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        s.push_str(line.trim_end());
        s.push('\n');
    }
    s
}

fn css(c: Color, default: &str) -> String {
    match c {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Reset => default.to_string(),
        other => format!("{other}").to_lowercase(),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn to_html(buf: &Buffer) -> String {
    let mut s = String::from("<pre class=\"term\">");
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            let mut fg = css(cell.fg, "#e8e9f1");
            let mut bg = css(cell.bg, "#0f1015");
            if cell.modifier.contains(Modifier::REVERSED) {
                std::mem::swap(&mut fg, &mut bg);
            }
            let bold = if cell.modifier.contains(Modifier::BOLD) {
                "font-weight:700;"
            } else {
                ""
            };
            let _ = write!(
                s,
                "<span style=\"color:{fg};background:{bg};{bold}\">{}</span>",
                esc(cell.symbol())
            );
        }
        s.push('\n');
    }
    s.push_str("</pre>");
    s
}

const HEAD: &str = r#"<!doctype html><html><head><meta charset="utf-8"><title>Vesper snapshots</title>
<style>
body{background:#0b0c10;color:#ccc;font-family:-apple-system,sans-serif;padding:16px}
h2{font-size:13px;font-weight:500;color:#888;margin:24px 0 6px}
pre.term{font-family:Menlo,"SF Mono","DejaVu Sans Mono",monospace;font-size:14px;line-height:1.18;margin:0;display:inline-block}
pre.term span{display:inline-block;width:1ch;text-align:center;overflow:visible}
</style></head><body>
"#;
