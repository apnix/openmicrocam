//! The screen as a function of the state: the whole scene tree, and the pictures it shows. Sizes are the screen's units
//! (1/1080 of an output's height); text is measured by the engine microcamd draws it with.

use microcam_core::controls::Value;
use microcam_core::files::FileKind;
use microcam_core::protocol::{PlayerState, RecordState, ResCode, Source};
use microcam_core::scene::{Align, Anchor, Colour, Fit, Group, ImageNode, Kind, Node, Paint, PathNode, RectNode, Space, Stroke, Style, TextNode};
use microcam_draw::render::{measure, Fonts};

use crate::core::{frozen_half_id, frozen_id, Core, Dir, Freeze, Gallery, GalleryView, Origin, Screen, Toast};
use crate::menu::{Item, Kind as ItemKind};
use crate::text::{self, Words};

const PANEL: &str = "#111111bb";
const TEXT: &str = "#eeeeee";
const DIM: &str = "#99aaaa";
const OFF: &str = "#666677";
const HELP: &str = "#ccccdd";
const AMBER: &str = "#ffb300";
const RED: &str = "#ff4433";
const SEL: &str = "#ffffff22";
const RULE: &str = "#ffffff33";
const TAG: &str = "#111111cc";
const LINE: &str = "#44dd66";
const CROSS: &str = "#eeeeeeee";
/// The built-in font's line box in ems: a size given as the mockup's font size is this much taller in the scene.
const EM: f32 = 1.362;
/// The menu and its cards: at the full menu level; chips and toasts: from the plates' level; overlays from the lines'.
const MENU: u8 = 3;
const CHIPS: u8 = 2;
const LINES: u8 = 1;

/// What the presenter knows of a picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pic {
    Waiting,
    Ready(u32, u32),
    Failed(ResCode),
}

/// The scene and what it needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn {
    pub nodes: Vec<Node>,
    /// the pictures it names, by the core's id
    pub pictures: Vec<(String, Source)>,
    /// a step counts as shown only once all of it is drawn (the pictures of the gallery, the frozen picture)
    pub whole: bool,
}

/// How much less a screen asks after the outputs could not show it: 0 as designed; each step down drops the
/// heaviest part left.
pub type Down = u8;
/// A chip's text size, and the widest a toast may be: the screen less its margins.
const CHIP_FONT: f32 = 28.0;
const TOAST_W: f32 = 1920.0 - 64.0;
/// The freeze's steps down: its picture at half the size.
pub const FREEZE_DOWN: Down = 1;

fn colour(c: &str) -> Paint {
    Paint::Solid(Colour::try_from(c.to_string()).expect("a colour of the table"))
}

fn node(kind: Kind) -> Node {
    Node { id: None, level: None, visible: true, paused: false, kind }
}

fn rect(x: f32, y: f32, w: f32, h: f32, r: f32, fill: &str) -> Node {
    node(Kind::Rect(RectNode { x, y, w, h, r, style: Style { fill: Some(colour(fill)), ..Style::default() } }))
}

fn frame(x: f32, y: f32, w: f32, h: f32, r: f32, stroke: &str, width: f32) -> Node {
    node(Kind::Rect(RectNode { x, y, w, h, r, style: Style { stroke: Some(line_stroke(stroke, width)), ..Style::default() } }))
}

fn line_stroke(c: &str, width: f32) -> Stroke {
    Stroke { paint: colour(c), width, join: Default::default(), cap: Default::default(), miter_limit: 4.0, dash: Vec::new(), dash_offset: 0.0 }
}

fn path(d: &str, stroke: &str, width: f32) -> Node {
    node(Kind::Path(PathNode { d: d.to_string().try_into().expect("a path of ours"), style: Style { stroke: Some(line_stroke(stroke, width)), ..Style::default() } }))
}

fn text_node(x: f32, y: f32, font: f32, s: &str, fill: &str, align: Align) -> TextNode {
    TextNode { x, y, size: font * EM, text: s.to_string(), fill: colour(fill), align, width: None, lines: None, ellipsis: false, line_height: 1.0 }
}

/// One line of text, its line box `line` tall from `y` (the mockup's line height), the font `font` in it.
fn text(x: f32, y: f32, line: f32, font: f32, s: &str, fill: &str, align: Align) -> Node {
    node(Kind::Text(text_node(x, y + (line - font * EM) / 2.0, font, s, fill, align)))
}

fn image(res: &str, x: f32, y: f32, w: f32, h: f32, fit: Fit, full_colour: bool) -> Node {
    node(Kind::Image(ImageNode { res: res.into(), x, y, w, h, crop: None, fit, full_colour, prepare: false }))
}

fn group(id: &str, level: Option<u8>, anchor: Anchor, space: Space, x: f32, y: f32, children: Vec<Node>) -> Node {
    let g = Group { x, y, rotate: 0.0, scale: Default::default(), anchor, space, clip: None, opacity: 1.0, mask: None, keep: false, window: Default::default(), children };
    Node { id: Some(id.into()), level, ..node(Kind::Group(g)) }
}

/// A key in a hint: its sign, or the gallery key the font has no sign for.
enum K {
    S(&'static str),
    Gallery,
}

/// What draws: the state, the pictures as told and the fonts to measure with.
pub struct View<'a> {
    pub core: &'a Core,
    pub pic: &'a dyn Fn(&str) -> Pic,
    pub fonts: &'a Fonts,
    pub down: Down,
}

impl View<'_> {
    fn w(&self) -> &'static Words {
        self.core.words()
    }

    fn width(&self, font: f32, s: &str) -> f32 {
        measure(&text_node(0.0, 0.0, font, s, TEXT, Align::Left), self.fonts).w
    }

    /// A hint line from (`x`, `y`): each key bright, what it does dim, «·» between; its nodes and width.
    fn hint(&self, x: f32, y: f32, parts: &[(K, &str)]) -> (Vec<Node>, f32) {
        const FONT: f32 = 22.0;
        const LINE: f32 = 34.0;
        let mut nodes = Vec::new();
        let mut at = x;
        for (i, (k, what)) in parts.iter().enumerate() {
            if i > 0 {
                let sep = " · ";
                nodes.push(text(at, y, LINE, FONT, sep, DIM, Align::Left));
                at += self.width(FONT, sep);
            }
            match k {
                K::S(s) => {
                    nodes.push(text(at, y, LINE, FONT, s, TEXT, Align::Left));
                    at += self.width(FONT, s);
                }
                K::Gallery => {
                    // the gallery key as printed on the remote: a picture in a frame
                    let (gx, gy) = (at, y + 5.0);
                    nodes.push(frame(gx + 1.0, gy + 1.0, 18.0, 22.0, 2.0, TEXT, 2.0));
                    nodes.push(node(Kind::Path(PathNode {
                        d: format!("M{} {} l4.5 -8 l3 4.5 l2.5 -4 l3 7.5 Z", gx + 3.5, gy + 20.0).try_into().expect("a path of ours"),
                        style: Style { fill: Some(colour(TEXT)), ..Style::default() },
                    })));
                    at += 20.0;
                }
            }
            let what = format!(" {what}");
            nodes.push(text(at, y, LINE, FONT, &what, DIM, Align::Left));
            at += self.width(FONT, &what);
        }
        (nodes, at - x)
    }

    /// A chip: rounded, dark, one line of text; its nodes at (`x`, `y`) and its width. `parts`: the text in colours.
    fn chip(&self, x: f32, y: f32, parts: &[(&str, &str)]) -> (Vec<Node>, f32) {
        const FONT: f32 = CHIP_FONT;
        let mut nodes = vec![rect(x, y, 0.0, 56.0, 10.0, PANEL)];
        let mut at = x + 18.0;
        for (s, c) in parts {
            nodes.push(text(at, y + 8.0, 40.0, FONT, s, c, Align::Left));
            at += self.width(FONT, s);
        }
        let w = at + 18.0 - x;
        if let Kind::Rect(r) = &mut nodes[0].kind {
            r.w = w;
        }
        (nodes, w)
    }

    /// A message's lines, each broken after a comma where its chip would be wider than `max`.
    fn wrap(&self, s: &str, max: f32) -> Vec<String> {
        let fits = |l: &str| self.width(CHIP_FONT, l) + 36.0 <= max;
        let mut out = Vec::new();
        for line in s.lines() {
            let mut cur = String::new();
            for part in line.split_inclusive(", ") {
                if !cur.is_empty() && !fits((cur.clone() + part).trim_end()) {
                    out.push(cur.trim_end().to_string());
                    cur.clear();
                }
                cur += part;
            }
            out.push(cur.trim_end().to_string());
        }
        out
    }

    /// A chip holding a hint line.
    fn hint_chip(&self, x: f32, y: f32, parts: &[(K, &str)]) -> Vec<Node> {
        let (mut nodes, w) = self.hint(x + 18.0, y + 8.0, parts);
        nodes.insert(0, rect(x, y, w + 36.0, 50.0, 10.0, PANEL));
        nodes
    }

    pub fn draw(&self) -> Drawn {
        let st = &self.core.st;
        let mut nodes = Vec::new();
        let mut pictures = Vec::new();
        let mut whole = false;
        match &st.screen {
            Screen::Gallery(g) => {
                whole = true;
                self.gallery(g, &mut nodes, &mut pictures);
            }
            screen => {
                whole |= self.freeze(&mut nodes, &mut pictures);
                self.overlays(screen, &mut nodes);
                self.chips(&mut nodes);
                match screen {
                    Screen::Menu => nodes.push(self.panel()),
                    Screen::Slider { item, .. } => nodes.push(self.slider(*item)),
                    Screen::Place(_) => {
                        let k = &self.w().keys;
                        let parts = [(K::S("▲▼"), k.shift), (K::S("▶"), k.put), (K::S("◀"), k.cancel)];
                        nodes.push(group("place", Some(MENU), Anchor::BottomLeft, Space::Screen, 32.0, -82.0, self.hint_chip(0.0, 0.0, &parts)));
                    }
                    _ => {}
                }
            }
        }
        if let Some(Toast { msg: m, .. }) = &st.toast {
            // a line a chip, the chips one panel: the widest sets its width, the lines below raise it; a line wider than
            // the screen leaves is broken after a comma
            let s = m.say(self.core.st.own.lang);
            let lines = self.wrap(&s, TOAST_W);
            let extra = 40.0 * (lines.len() - 1) as f32;
            let mut c = Vec::new();
            let mut wide: f32 = 0.0;
            for (i, l) in lines.iter().enumerate() {
                let (mut one, w) = self.chip(0.0, 40.0 * i as f32, &[(l.as_str(), TEXT)]);
                one.remove(0);
                c.extend(one);
                wide = wide.max(w);
            }
            c.insert(0, rect(0.0, 0.0, wide, 56.0 + extra, 10.0, PANEL));
            for n in &mut c {
                shift(n, -wide / 2.0);
            }
            nodes.push(group("toast", Some(CHIPS), Anchor::Bottom, Space::Screen, 0.0, -96.0 - extra, c));
        }
        Drawn { nodes, pictures, whole }
    }

    /// The frozen picture over the live one, or its left half beside it; whether it is shown.
    fn freeze(&self, nodes: &mut Vec<Node>, pictures: &mut Vec<(String, Source)>) -> bool {
        let n = match self.core.st.freeze {
            Freeze::Off => return false,
            Freeze::Frozen(n) | Freeze::Compare(n) => n,
        };
        // the outputs' largest picture; a step down (one only) takes it again at half the size, under an id of its own
        let (id, w, h) = if self.down == 0 { (frozen_id(n), 3840, 2160) } else { (frozen_half_id(n), 1920, 1080) };
        pictures.push((id.clone(), Source::Camera { w, h }));
        let half = matches!(self.core.st.freeze, Freeze::Compare(_));
        let mut img = image(&id, -960.0, -540.0, if half { 960.0 } else { 1920.0 }, 1080.0, if half { Fit::Stretch } else { Fit::Cover }, true);
        if half {
            let Pic::Ready(pw, ph) = (self.pic)(&id) else { return true };
            if let Kind::Image(m) = &mut img.kind {
                m.crop = Some([0.0, 0.0, pw as f32 / 2.0, ph as f32]);
            }
        }
        let mut children = vec![img];
        if half {
            children.push(Node { level: Some(LINES), ..path("M0 -540 V540", TEXT, 2.0) });
        }
        nodes.push(group("frozen", None, Anchor::Centre, Space::Screen, 0.0, 0.0, children));
        true
    }

    /// The crosshair and the lines, tied to the live picture as it is shown.
    fn overlays(&self, screen: &Screen, nodes: &mut Vec<Node>) {
        let st = &self.core.st;
        let mut children = Vec::new();
        if st.own.cross {
            children.push(path("M0.4698 0.5 H0.5302", CROSS, 2.0));
            children.push(path("M0.5 0.4463 V0.5537", CROSS, 2.0));
        }
        let placing = if let Screen::Place(l) = screen { Some(*l) } else { None };
        for (l, c) in st.lines.iter().map(|l| (*l, LINE)).chain(placing.map(|l| (l, AMBER))) {
            let p = l.pos as f32 / 1000.0;
            let d = match l.dir {
                Dir::H => format!("M0 {p} H1"),
                Dir::V => format!("M{p} 0 V1"),
            };
            children.push(path(&d, c, 3.0));
        }
        if children.is_empty() {
            return;
        }
        nodes.push(group("overlays", Some(LINES), Anchor::TopLeft, Space::Live, 0.0, 0.0, children));
    }

    /// The states that are not the default, top right; the recording clock the rightmost.
    fn chips(&self, nodes: &mut Vec<Node>) {
        let core = self.core;
        let st = &core.st;
        let on = |c: &str| core.control(c).is_some_and(|c| c.value == Value::Bool(true));
        let mut list: Vec<Vec<(String, &str)>> = Vec::new();
        if let Some(f) = core.flash_text() {
            list.push(vec![(f, TEXT)]);
        }
        match st.freeze {
            Freeze::Frozen(_) => list.push(vec![(self.w().frozen.into(), AMBER)]),
            Freeze::Compare(_) => list.push(vec![(self.w().compare.into(), AMBER)]),
            Freeze::Off => {}
        }
        if on("exposure-lock") {
            list.push(vec![(self.w().ae_locked.into(), AMBER)]);
        }
        if on("white-balance-lock") {
            list.push(vec![(self.w().wb_locked.into(), AMBER)]);
        }
        if st.cam.running.record.as_ref().is_some_and(|r| r.state != RecordState::Stopping) {
            list.push(vec![("● ".into(), RED), (text::clock(core.record_now() / 1000), TEXT)]);
        }
        if list.is_empty() {
            return;
        }
        let mut children = Vec::new();
        let mut right = -32.0;
        for parts in list.iter().rev() {
            let parts: Vec<(&str, &str)> = parts.iter().map(|(s, c)| (s.as_str(), *c)).collect();
            let (_, w) = self.chip(0.0, 0.0, &parts);
            let (c, _) = self.chip(right - w, 32.0, &parts);
            children.extend(c);
            right -= w + 12.0;
        }
        nodes.push(group("chips", Some(CHIPS), Anchor::TopRight, Space::Screen, 0.0, 0.0, children));
    }

    /// The menu panel, top left: the title, the rows, the chosen row's description (the panel as tall as the
    /// longest of its level's), the keys.
    fn panel(&self) -> Node {
        const W: f32 = 560.0;
        const ROW: f32 = 64.0;
        let st = &self.core.st;
        let spot = *st.menu.last().expect("the menu has its root");
        let page = self.core.page(spot.level);
        let mut c = Vec::new();
        let mut y = 10.0;
        c.push(text(28.0, y, 52.0, 26.0, &page.title, DIM, Align::Left));
        y += 52.0;
        for (i, r) in page.rows.iter().enumerate() {
            let sel = i == spot.sel;
            if sel {
                c.push(rect(0.0, y, W, ROW, 0.0, SEL));
                c.push(rect(0.0, y + 12.0, 6.0, ROW - 24.0, 0.0, AMBER));
            }
            c.push(text(28.0, y, ROW, 28.0, &r.name, if r.dim { OFF } else { TEXT }, Align::Left));
            let value = if r.mark { "●" } else { r.value.as_str() };
            c.push(text(W - 24.0, y, ROW, 28.0, value, if r.dim { OFF } else if sel { AMBER } else { DIM }, Align::Right));
            y += ROW;
        }
        if page.rows.iter().any(|r| r.help.is_some()) {
            const FONT: f32 = 23.0;
            let help = |s: &str| TextNode { width: Some(W - 56.0), line_height: 32.0 / (FONT * EM), ..text_node(28.0, y + 24.0, FONT, s, HELP, Align::Left) };
            let tall = page.rows.iter().filter_map(|r| r.help.as_deref()).map(|h| measure(&help(h), self.fonts).h).fold(0.0, f32::max);
            c.push(rect(28.0, y + 10.0, W - 56.0, 2.0, 0.0, RULE));
            if let Some(h) = page.rows.get(spot.sel).and_then(|r| r.help.as_deref()) {
                c.push(node(Kind::Text(help(h))));
            }
            y += 24.0 + tall;
        }
        let k = &self.w().keys;
        let back = if st.menu.len() > 1 { k.back } else { k.close };
        let (h, _) = self.hint(28.0, y + 8.0, &[(K::S("▲▼"), k.choose), (K::S("▶"), k.enter), (K::S("◀"), back)]);
        c.extend(h);
        y += 8.0 + 34.0 + 12.0;
        c.insert(0, rect(0.0, 0.0, W, y, 16.0, PANEL));
        group("panel", Some(MENU), Anchor::TopLeft, Space::Screen, 32.0, 32.0, c)
    }

    /// The slider card, bottom left: the name, the value, the bar with the default marked, the keys.
    fn slider(&self, item: Item) -> Node {
        const W: f32 = 640.0;
        const H: f32 = 148.0;
        let ItemKind::Number(name) = item.kind() else { unreachable!("a slider shows a number") };
        let ctl = self.core.control(name);
        let int = |v: Option<&Value>| match v {
            Some(Value::Int(i)) => *i as f32,
            _ => 0.0,
        };
        let (lo, hi) = (ctl.and_then(|c| c.min).unwrap_or(0) as f32, ctl.and_then(|c| c.max).unwrap_or(1) as f32);
        let at = |v: f32| ((v - lo) / (hi - lo).max(1.0)).clamp(0.0, 1.0);
        let bar = W - 56.0;
        let mut c = vec![
            rect(0.0, 0.0, W, H, 16.0, PANEL),
            text(28.0, 16.0, 44.0, 28.0, (self.w().item)(item).name, TEXT, Align::Left),
            text(W - 28.0, 16.0, 44.0, 34.0, &self.core.number(item).unwrap_or_default(), AMBER, Align::Right),
            rect(28.0, 74.0, bar, 10.0, 0.0, RULE),
            rect(28.0, 74.0, bar * at(int(ctl.map(|c| &c.value))), 10.0, 0.0, AMBER),
            rect(28.0 + bar * at(int(ctl.map(|c| &c.default))) - 1.0, 66.0, 2.0, 26.0, 0.0, TEXT),
        ];
        let k = &self.w().keys;
        let mut parts = vec![(K::S("◀ ▶"), k.value)];
        if self.core.near_count(item) > 1 {
            parts.push((K::S("▲▼"), k.setting));
        }
        parts.push((K::S("MENU"), k.done));
        c.extend(self.hint(28.0, 100.0, &parts).0);
        group("slider", Some(MENU), Anchor::BottomLeft, Space::Screen, 32.0, -32.0 - H, c)
    }

    /// The gallery: the chosen file over the whole screen, the strip of its neighbours over it on the left until the
    /// file is opened; a video playing in the live window with its clock.
    fn gallery(&self, g: &Gallery, nodes: &mut Vec<Node>, pictures: &mut Vec<(String, Source)>) {
        let core = self.core;
        let total = core.st.cam.files.total;
        let file = |i: usize| g.list.as_ref().and_then(|w| i.checked_sub(w.from).and_then(|k| w.list.get(k)));
        // a video asked to play: no picture held, the player needs the memory
        let playing = matches!(g.view, GalleryView::Video { .. });
        let mut want = |path: &str| {
            if !pictures.iter().any(|p| p.0 == path) {
                pictures.push((path.to_string(), Source::File(path.to_string())));
            }
        };
        let mut full = Vec::new();
        if let Some(f) = file(g.at).filter(|_| !playing) {
            want(&f.path);
            match (self.pic)(&f.path) {
                Pic::Failed(_) => full.extend(self.sign(-960.0, -540.0, 1920.0, 1080.0, 120.0)),
                _ => full.push(image(&f.path, -960.0, -540.0, 1920.0, 1080.0, Fit::Cover, true)),
            }
        }
        // the next file made ready ahead, the way the choice went last
        if let Some(f) = file(if g.back { g.at.checked_sub(1).unwrap_or(total.saturating_sub(1)) } else { (g.at + 1) % total.max(1) }).filter(|_| self.down == 0 && !playing) {
            if !matches!((self.pic)(&f.path), Pic::Failed(_)) && Some(&f.path) != file(g.at).map(|c| &c.path) {
                want(&f.path);
                let mut n = image(&f.path, -960.0, -540.0, 1920.0, 1080.0, Fit::Cover, true);
                if let Kind::Image(m) = &mut n.kind {
                    m.prepare = true;
                }
                full.push(n);
            }
        }
        if !full.is_empty() {
            nodes.push(group("file", None, Anchor::Centre, Space::Screen, 0.0, 0.0, full));
        }
        let x0 = if g.view == GalleryView::Strip { 424.0 } else { 32.0 };
        if g.view == GalleryView::Strip {
            let mut c = vec![Node { level: Some(MENU), ..rect(32.0, 32.0, 368.0, 1016.0, 18.0, PANEL) }];
            for s in -2i64..=2 {
                let Some(f) = usize::try_from(g.at as i64 + s).ok().filter(|i| *i < total).and_then(file) else { continue };
                let y = 450.0 + s as f32 * 196.0;
                if self.down < 2 {
                    want(&f.path);
                    match (self.pic)(&f.path) {
                        Pic::Failed(_) => c.extend(self.sign(56.0, y, 320.0, 180.0, 60.0)),
                        // a step down: the thumbnails in the menu's layer, not the frame
                        _ => c.push(Node { level: (self.down > 0).then_some(MENU), ..image(&f.path, 56.0, y, 320.0, 180.0, Fit::Cover, self.down == 0) }),
                    }
                }
                if f.kind == FileKind::Video {
                    c.push(Node { level: Some(MENU), ..rect(64.0, y + 142.0, 36.0, 30.0, 0.0, TAG) });
                    c.push(Node { level: Some(MENU), ..text(72.0, y + 142.0, 30.0, 22.0, "▶", TEXT, Align::Left) });
                }
            }
            c.push(Node { level: Some(MENU), ..frame(50.0, 444.0, 332.0, 192.0, 8.0, AMBER, 4.0) });
            nodes.push(group("strip", None, Anchor::TopLeft, Space::Screen, 0.0, 0.0, c));
        }
        let name = file(g.at).map_or(String::new(), |f| f.name.clone());
        let label = format!("{name} · {}", (self.w().of)(&(g.at + 1).to_string(), &total.to_string()));
        let mut chip: Vec<(&str, &str)> = vec![(&label, TEXT)];
        let paused = playing && core.st.cam.player.as_ref().is_some_and(|p| matches!(p.state, PlayerState::Paused | PlayerState::Ended));
        let (mut c, w) = self.chip(x0, 32.0, &chip);
        if paused {
            chip = vec![(self.w().paused, AMBER)];
            c.extend(self.chip(x0 + w + 12.0, 32.0, &chip).0);
        }
        nodes.push(group("name", Some(CHIPS), Anchor::TopLeft, Space::Screen, 0.0, 0.0, c));
        let chosen_video = file(g.at).is_some_and(|f| f.kind == FileKind::Video);
        if playing {
            const W: f32 = 880.0;
            let p = core.st.cam.player.as_ref();
            // the clock's second: the card changes once a second, not at every commit the outputs could take
            let (ms, len) = (core.player_ms().unwrap_or(0) / 1000 * 1000, p.map_or(0, |p| p.duration_ms));
            let mut c = vec![
                rect(0.0, 0.0, W, 138.0, 16.0, PANEL),
                text(28.0, 16.0, 44.0, 28.0, &format!("{} / {}", text::clock(ms / 1000), text::clock(len / 1000)), TEXT, Align::Left),
                rect(28.0, 74.0, W - 56.0, 10.0, 0.0, RULE),
                rect(28.0, 74.0, (W - 56.0) * (ms as f32 / len.max(1) as f32).min(1.0), 10.0, 0.0, AMBER),
            ];
            let k = &self.w().keys;
            let play = if paused { k.go_on } else { k.pause };
            let parts = [(K::S("◀ ▶"), k.seek), (K::S("▶‖"), play), (K::S("▲▼"), k.file), (K::S("MENU"), k.to_strip), (K::Gallery, k.to_camera)];
            c.extend(self.hint(28.0, 90.0, &parts).0);
            nodes.push(group("player", Some(MENU), Anchor::BottomLeft, Space::Screen, 32.0, -170.0, c));
        } else {
            let k = &self.w().keys;
            let parts: Vec<(K, &str)> = match g.view {
                GalleryView::Strip => {
                    let mut p = vec![(K::S("▲▼"), k.file), (K::S("▶"), if chosen_video { k.watch } else { k.full }), (K::S("◀"), if g.from == Origin::Menu { k.to_menu } else { k.close })];
                    if g.from == Origin::Menu {
                        p.push((K::Gallery, k.to_camera));
                    }
                    p
                }
                _ => vec![(K::S("▲▼"), k.file), (K::S("◀"), k.to_strip), (K::Gallery, k.to_camera)],
            };
            nodes.push(group("keys", Some(MENU), Anchor::BottomLeft, Space::Screen, x0, -82.0, self.hint_chip(0.0, 0.0, &parts)));
        }
    }

    /// The menu's own sign for a file without a picture (a video with no cover): dark, a ▶ in the middle.
    fn sign(&self, x: f32, y: f32, w: f32, h: f32, font: f32) -> Vec<Node> {
        vec![rect(x, y, w, h, 0.0, "#202428ff"), text(x + w / 2.0, y + h / 2.0 - font * EM / 2.0, font * EM, font, "▶", DIM, Align::Centre)]
    }
}

/// A node moved right by `dx`.
fn shift(n: &mut Node, dx: f32) {
    match &mut n.kind {
        Kind::Rect(r) => r.x += dx,
        Kind::Text(t) => t.x += dx,
        _ => {}
    }
}
