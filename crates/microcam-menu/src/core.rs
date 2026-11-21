//! The menu without I/O: a key, an event or reply of microcamd, or the clock goes in ([`Input`]); the state moves at
//! once and what to ask the camera comes out ([`Effect`]). The camera's values are shown as microcamd confirms them
//! (its snapshot and events); the screens, the lines, the language and the crosshair are the menu's own.

use std::collections::BTreeMap;

use microcam_core::controls::{Code, Control, Held, Holder, Kind as ControlKind, Value};
use microcam_core::precedence::{Precedence, Who};
use microcam_core::files::{FileEntry, FileKind};
use microcam_core::keys::{Key, Phase};
use microcam_core::protocol::{Action, Card, CardInfo, Envelope, Event, Output, FilesInfo, FilesWindow, OutputInfo, PlayerInfo, PlayerState, RecordState, Request, ResCode, ResState, Response, Running, Snapshot};

use crate::menu::{Entry, Item, Kind, Section, ROOT};
use crate::text::{self, Asked, Lang, Msg, Wanted, Words};

/// A toast's time on the screen unless its caller gives one (the mockup's).
const TOAST_MS: u64 = 2200;
/// A toast that asks the user to act — a list of what to turn off, the card to eject on the computer: about the time
/// to read twenty words.
const TOAST_LONG_MS: u64 = 6000;
/// A chip that comes for a moment (the zoom's factor, the mirror, the start): its time.
const FLASH_MS: u64 = 2000;
/// The pause before asking for the scene again when another menu holds it.
const RETRY_MS: u64 = 1000;
/// A video's step back or forward.
const SEEK_MS: u64 = 5000;
/// A line's step while it is put, in thousandths of the screen's side, and the edges it stays inside.
const LINE_STEP: u16 = 5;
const LINE_EDGE: u16 = 10;
/// The gallery's window of the card's list: the strip shows two files on each side of the chosen one and the next is
/// prepared, so a window is read again when the choice comes this near its edge.
const WINDOW: usize = 32;
const MARGIN: usize = 4;
/// The menu's record's format.
const RECORD_VERSION: u64 = 1;

#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    /// The socket is open: the menu asks for the scene.
    Connected,
    /// The socket broke: nothing of the session is asked again.
    Disconnected,
    Reply(Response),
    Event(Event),
    /// Only the time moved.
    Clock,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Send(Envelope),
    /// Sent once the screen shown holds none of the pictures the state no longer names (they are gone from microcamd's
    /// memory): a request that needs that memory, the player's.
    SendClear(Envelope),
    /// The core wants a [`Input::Clock`] at this time (ms); the latest one replaces those before it.
    Wake(u64),
}

/// The menu's hold on microcamd's scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Offline,
    /// `hello` sent
    Asking,
    /// another menu holds the scene: asked again at this time
    Waiting(u64),
    Owner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    /// a line from left to right, moved up and down
    H,
    V,
}

/// A line over the picture: its direction and where it crosses the screen, in thousandths of its height (a horizontal
/// one) or width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub dir: Dir,
    pub pos: u16,
}

/// The ❄ key's three presses: the picture frozen, its left half beside the camera, the camera. The number names the
/// camera picture taken for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freeze {
    Off,
    Frozen(u32),
    Compare(u32),
}

/// The resource id of the camera picture of a freeze.
pub fn frozen_id(n: u32) -> String {
    format!("frozen-{n}")
}

/// The id of a freeze's picture taken at half the size, for a screen stepped down: a picture of its own, since the
/// camera picture is taken when it is asked for.
pub fn frozen_half_id(n: u32) -> String {
    format!("frozen-{n}-half")
}

/// The freeze a resource id names a picture of, at either size.
pub fn frozen_of(id: &str) -> Option<u32> {
    let n = id.strip_prefix("frozen-")?;
    n.strip_suffix("-half").unwrap_or(n).parse().ok()
}

/// Whether a resource id names the camera picture of a freeze.
pub fn is_frozen(id: &str) -> bool {
    frozen_of(id).is_some()
}

/// A level of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Root,
    Section(Section),
    /// a value item's list of values
    Values(Item),
    AddLine,
    Card,
    FormatAsk,
    ResetAsk,
    About,
}

/// A level opened and the row chosen on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spot {
    pub level: Level,
    pub sel: usize,
}

/// Where the gallery was opened from, where ◀ on its strip goes back to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Live,
    Menu,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GalleryView {
    /// the chosen file on the screen, the strip of its neighbours over it
    Strip,
    /// the chosen file alone
    Alone,
    /// the chosen video to play: `asked` once the player was free for it, `seen` once it shows it
    Video { file: String, seen: bool, asked: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gallery {
    pub from: Origin,
    /// the chosen file's place in the card's list, newest first
    pub at: usize,
    /// its path, once its place is in a window read
    pub path: Option<String>,
    pub view: GalleryView,
    pub list: Option<FilesWindow>,
    /// the last step went to a newer file (▲): the one made ready ahead is in that direction
    pub back: bool,
    asking: bool,
    /// a seek asked and not yet told by the player: the next step goes on from it
    seek: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    /// the camera alone
    Live,
    Menu,
    /// a number on the slider; `round`: where the ◐ key's round began
    Slider { item: Item, round: Item },
    /// a line being put
    Place(Line),
    Gallery(Gallery),
}

/// A chip that comes for a moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flash {
    Zoom,
    Mirror,
    AeAuto,
    WbAuto,
    /// the start: the HDMI mode, the zoom and the card
    Start,
}


/// A toast: its message, until when it is shown, and the time its caller gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast {
    pub msg: Msg,
    pub until: u64,
    pub ms: u64,
}

/// The camera as microcamd told it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cam {
    pub controls: BTreeMap<String, Control>,
    pub running: Running,
    /// when `running` was told (its recording's clock runs on from there)
    running_at: u64,
    pub files: FilesInfo,
    pub outputs: Vec<OutputInfo>,
    pub player: Option<PlayerInfo>,
    player_at: u64,
    pub card: CardInfo,
}

/// The menu's own record, kept by microcamd.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Own {
    pub lang: Lang,
    pub cross: bool,
}

impl Default for Own {
    fn default() -> Self {
        Own { lang: Lang::En, cross: true }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub now: u64,
    pub link: Link,
    pub cam: Cam,
    pub own: Own,
    pub screen: Screen,
    /// the menu's levels, kept while it is hidden: MENU opens it where it was left
    pub menu: Vec<Spot>,
    pub lines: Vec<Line>,
    pub freeze: Freeze,
    pub flash: Option<(Flash, u64)>,
    pub toast: Option<Toast>,
    /// the build microcamd runs, as `status` told it for «О камере»
    pub build: Option<String>,
}

/// A row as the menu shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub value: String,
    pub help: Option<String>,
    pub dim: bool,
    /// the current one of a list of values
    pub mark: bool,
    pub act: Option<Act>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub title: String,
    pub rows: Vec<Row>,
}

/// What ▶ on a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Open(Level),
    Gallery,
    Slider(Item),
    /// a value item to its value of this place
    Pick(Item, usize),
    Do(Item),
    Line(Dir),
    Format,
    ResetAll,
    /// the card reader mode on or off
    CardReader(bool),
    Back,
}

/// One of a value item's values: what choosing it asks.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Pick {
    Set(&'static str, Value),
    Mirror(bool, bool),
    Cross(bool),
    Lang(Lang),
}

/// What a request's reply is for.
#[derive(Clone, Debug, PartialEq)]
enum Pending {
    Hello,
    Set { name: String, group: Option<u64> },
    Adjust { name: String },
    Asked(Asked),
    /// the card reader mode switched off: the card taken from the computer for what follows
    Took(Then),
    /// asked with the card just taken from the computer: given back when it is refused
    Lent(Asked),
    Files,
    Status,
    /// a reply nobody acts on
    Quiet,
}

/// What a press does with the card once the camera has it.
#[derive(Clone, Debug, PartialEq)]
enum Then {
    Ask(Asked, Request),
    Gallery(Origin),
}

impl Then {
    fn asked(&self) -> Asked {
        match self {
            Then::Ask(what, _) => what.clone(),
            Then::Gallery(_) => Asked::Gallery,
        }
    }
}

/// Sets sent together whose toast comes once all are accepted.
struct Group {
    left: usize,
    failed: bool,
    msg: Msg,
}

pub struct Core {
    pub st: State,
    take: bool,
    seq: u64,
    pending: BTreeMap<u64, Pending>,
    /// a control's value asked and not yet answered, with the number of such asks: the next toggle goes on from it
    asked: BTreeMap<String, (Value, usize)>,
    groups: BTreeMap<u64, Group>,
    next_group: u64,
    /// a `reset` sent: the scene microcamd takes with it is asked for again at once
    resetting: bool,
    /// what the card was taken from the computer for, until its outcome: given back when it fails
    lent: Option<Asked>,
    /// the start's chip shown (once a run)
    started: bool,
    /// the recording's length as last told, for the toast at its end
    record_ms: u64,
    frozen: u32,
    /// outputs that could not show the menu for lack of resources, until they present again: said once
    missing: Vec<Output>,
    /// the revision the last of those was said for: the other outputs' refusals of it are not said again
    said_rev: Option<u64>,
    fx: Vec<Effect>,
}

fn basename(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

impl Core {
    /// `take`: the scene taken from another menu (a program started by hand); the one started at boot waits instead.
    pub fn new(take: bool) -> Self {
        let st = State {
            now: 0,
            link: Link::Offline,
            cam: Cam::default(),
            own: Own::default(),
            screen: Screen::Live,
            menu: vec![Spot { level: Level::Root, sel: 0 }],
            lines: Vec::new(),
            freeze: Freeze::Off,
            flash: None,
            toast: None,
            build: None,
        };
        Core { st, take, seq: 0, pending: BTreeMap::new(), asked: BTreeMap::new(), groups: BTreeMap::new(), next_group: 0, resetting: false, lent: None, started: false, record_ms: 0, frozen: 0, missing: Vec::new(), said_rev: None, fx: Vec::new() }
    }

    pub fn step(&mut self, now: u64, input: Input) -> Vec<Effect> {
        self.st.now = now;
        if self.st.toast.as_ref().is_some_and(|t| t.until <= now) {
            self.st.toast = None;
        }
        if self.st.flash.as_ref().is_some_and(|f| f.1 <= now) {
            self.st.flash = None;
        }
        match input {
            Input::Connected => self.hello(),
            Input::Disconnected => {
                self.st.link = Link::Offline;
                self.resetting = false;
                self.end_session();
            }
            Input::Reply(r) => self.reply(r),
            Input::Event(e) => self.event(e),
            Input::Clock => {}
        }
        if let Link::Waiting(at) = self.st.link {
            if at <= now {
                self.hello();
            }
        }
        if let Some(at) = self.wake() {
            self.fx.push(Effect::Wake(at));
        }
        std::mem::take(&mut self.fx)
    }

    /// The earliest time something on the screen changes by itself.
    fn wake(&self) -> Option<u64> {
        let now = self.st.now;
        let second = |ms: u64| now + 1000 - ms % 1000;
        let mut at: Vec<u64> = Vec::new();
        at.extend(self.st.toast.as_ref().map(|t| t.until));
        at.extend(self.st.flash.as_ref().map(|f| f.1));
        if let Link::Waiting(t) = self.st.link {
            at.push(t);
        }
        if self.st.cam.running.record.as_ref().is_some_and(|r| r.state == RecordState::Running) {
            at.push(second(self.record_now()));
        }
        if let (Screen::Gallery(Gallery { view: GalleryView::Video { .. }, .. }), Some(p)) = (&self.st.screen, &self.st.cam.player) {
            if p.state == PlayerState::Playing {
                at.push(second(self.player_ms().unwrap_or(0)));
            }
        }
        at.into_iter().min()
    }

    // ---- the link

    fn ask(&mut self, req: Request, p: Pending) -> u64 {
        self.seq += 1;
        self.pending.insert(self.seq, p);
        self.fx.push(Effect::Send(Envelope { seq: Some(self.seq), req }));
        self.seq
    }

    /// A request sent once the pictures left are gone ([`Effect::SendClear`]).
    fn ask_clear(&mut self, req: Request, p: Pending) {
        self.ask(req, p);
        if let Some(Effect::Send(e)) = self.fx.pop() {
            self.fx.push(Effect::SendClear(e));
        }
    }

    fn hello(&mut self) {
        self.st.link = Link::Asking;
        let take = self.take && !self.resetting;
        self.ask(Request::Hello { take }, Pending::Hello);
    }

    /// The session is over (the socket broke or the scene went to another menu): nothing asked in it is waited for,
    /// what lived in microcamd for it (the player, the camera picture) is gone.
    fn end_session(&mut self) {
        self.pending.clear();
        self.asked.clear();
        self.groups.clear();
        self.missing.clear();
        self.st.freeze = Freeze::Off;
        if matches!(self.st.screen, Screen::Gallery(_)) {
            self.st.screen = Screen::Live;
        }
    }

    fn load(&mut self, s: Snapshot) {
        let now = self.st.now;
        let cam = &mut self.st.cam;
        cam.controls = s.controls.into_iter().map(|c| (c.name.clone(), c)).collect();
        cam.running = s.running;
        cam.running_at = now;
        cam.files = s.files;
        cam.outputs = s.outputs;
        cam.player = s.player;
        cam.player_at = now;
        cam.card = s.card;
        // a record of a format not ours is left as it is: the defaults are shown and nothing is written over it
        self.st.own = Own::default();
        if let Some(r) = s.record.filter(|r| r.get("v").and_then(|v| v.as_u64()) == Some(RECORD_VERSION)) {
            if let Some(l) = r.get("lang").and_then(|l| l.as_str()).and_then(Lang::of) {
                self.st.own.lang = l;
            }
            if let Some(c) = r.get("cross").and_then(|c| c.as_bool()) {
                self.st.own.cross = c;
            }
        }
    }

    fn keep(&mut self) {
        let record = serde_json::json!({ "v": RECORD_VERSION, "lang": self.st.own.lang.code(), "cross": self.st.own.cross });
        self.ask(Request::Keep { record }, Pending::Quiet);
    }

    fn toast(&mut self, m: Msg) {
        self.toast_for(m, TOAST_MS);
    }

    /// A toast shown for `ms`.
    fn toast_for(&mut self, m: Msg, ms: u64) {
        self.st.toast = Some(Toast { msg: m, until: self.st.now + ms, ms });
    }

    /// A refusal for lack of resources, whatever path it came by. The recording outranks the freeze: a recording
    /// refused for the room of the frozen picture drops the freeze and is asked again once the picture is gone;
    /// anything else is said in the one toast.
    fn short(&mut self, what: Wanted, holders: Vec<Holder>) {
        let record = matches!(what, Wanted::Asked(Asked::Record) | Wanted::Failed(Action::Record));
        let frozen = self.freeze_n();
        if record && frozen.is_some() && holders.iter().any(|h| matches!(&h.what, Held::Picture { id } if frozen_of(id) == frozen)) {
            self.st.freeze = Freeze::Off;
            self.ask_clear(Request::Record, Pending::Asked(Asked::Record));
            return;
        }
        self.toast_for(Msg::Resources(what, holders), TOAST_LONG_MS);
    }

    /// The number of the freeze on the screen.
    fn freeze_n(&self) -> Option<u32> {
        match self.st.freeze {
            Freeze::Frozen(n) | Freeze::Compare(n) => Some(n),
            Freeze::Off => None,
        }
    }

    fn flash(&mut self, f: Flash) {
        self.st.flash = Some((f, self.st.now + FLASH_MS));
    }

    // ---- the camera's values

    pub fn control(&self, name: &str) -> Option<&Control> {
        self.st.cam.controls.get(name)
    }

    fn value(&self, name: &str) -> Option<&Value> {
        self.control(name).map(|c| &c.value)
    }

    /// A switch as microcamd holds it, or as last asked.
    fn wanted_on(&self, name: &str) -> bool {
        self.asked.get(name).map(|a| &a.0).or(self.value(name)) == Some(&Value::Bool(true))
    }

    fn set(&mut self, name: &'static str, value: Value, group: Option<u64>) {
        let a = self.asked.entry(name.to_string()).or_insert((value.clone(), 0));
        *a = (value.clone(), a.1 + 1);
        self.ask(Request::Set { name: name.into(), value }, Pending::Set { name: name.into(), group });
    }

    fn adjust(&mut self, name: &str, steps: i64) {
        self.ask(Request::Adjust { name: name.into(), steps }, Pending::Adjust { name: name.into() });
    }

    /// The recording's clock now, ms.
    pub fn record_now(&self) -> u64 {
        match &self.st.cam.running.record {
            Some(r) if r.state == RecordState::Running => r.ms + self.st.now.saturating_sub(self.st.cam.running_at),
            Some(r) => r.ms,
            None => 0,
        }
    }

    /// Where the player is now, ms.
    pub fn player_ms(&self) -> Option<u64> {
        let p = self.st.cam.player.as_ref()?;
        Some(match p.state {
            PlayerState::Playing => (p.ms + self.st.now.saturating_sub(self.st.cam.player_at)).min(p.duration_ms),
            _ => p.ms,
        })
    }

    /// The zoom's factor in tenths (10 = ×1.0).
    pub fn zoom(&self) -> i64 {
        10 + match self.value("zoom") {
            Some(Value::Int(z)) => *z,
            _ => 0,
        }
    }

    fn mirror_index(&self) -> Option<usize> {
        match (self.value("mirror"), self.value("flip")) {
            (Some(Value::Bool(m)), Some(Value::Bool(f))) => Some(*m as usize + 2 * *f as usize),
            _ => None,
        }
    }

    // ---- replies and events

    fn reply(&mut self, r: Response) {
        let Some(p) = r.seq.and_then(|s| self.pending.remove(&s)) else { return };
        match p {
            Pending::Hello if r.ok => {
                if let Some(s) = r.snapshot {
                    self.load(s);
                }
                self.st.link = Link::Owner;
                if std::mem::take(&mut self.resetting) {
                    self.toast(Msg::SettingsReset);
                }
                if !self.started {
                    self.started = true;
                    self.flash(Flash::Start);
                }
            }
            Pending::Hello => self.st.link = Link::Waiting(self.st.now + RETRY_MS),
            Pending::Set { name, group } => {
                if let Some(a) = self.asked.get_mut(&name) {
                    a.1 -= 1;
                    if a.1 == 0 {
                        self.asked.remove(&name);
                    }
                }
                if !r.ok {
                    self.refused(Asked::Control(name), r.code, r.holders);
                }
                if let Some(g) = group {
                    let done = self.groups.get_mut(&g).map(|grp| {
                        grp.left -= 1;
                        grp.failed |= !r.ok;
                        grp.left == 0
                    });
                    if done == Some(true) {
                        let grp = self.groups.remove(&g).expect("the group just counted");
                        if !grp.failed {
                            self.toast(grp.msg);
                        }
                    }
                }
            }
            Pending::Adjust { name } if !r.ok => self.refused(Asked::Control(name), r.code, r.holders),
            Pending::Asked(what) if !r.ok => {
                if what == Asked::Reset {
                    self.resetting = false;
                }
                if what == Asked::Play {
                    if let Screen::Gallery(g) = &mut self.st.screen {
                        g.view = GalleryView::Alone;
                    }
                }
                self.refused(what, r.code, r.holders);
            }
            Pending::Took(then) if !r.ok => self.refused(then.asked(), r.code, r.holders),
            // the card on the camera (its `card` and `files` events came before): what was pressed done now; a card the
            // camera cannot read is only formatted
            Pending::Took(then) => {
                if self.st.cam.card.at == Card::Unreadable && !matches!(then, Then::Ask(Asked::Format, _)) {
                    self.toast_for(Msg::CardUnreadable(then.asked()), TOAST_LONG_MS);
                    self.give_back();
                } else {
                    self.then(then, true);
                }
            }
            Pending::Lent(what) if !r.ok => {
                self.refused(what, r.code, r.holders);
                self.give_back();
            }
            Pending::Lent(what) => self.lent = Some(what),
            Pending::Files => {
                if let Some(w) = r.files {
                    self.window(w);
                }
            }
            Pending::Status => {
                if let Some(s) = r.status {
                    self.st.build = Some(s.build.split_whitespace().next().unwrap_or("").to_string());
                }
            }
            Pending::Adjust { .. } | Pending::Asked(_) | Pending::Quiet => {}
        }
    }

    /// A request refused: the card's absence, the card reader mode and the computer's hold on the card said as such,
    /// whatever was asked of the card; the resources' refusal one way.
    fn refused(&mut self, what: Asked, code: Option<Code>, holders: Vec<Holder>) {
        match code {
            Some(Code::NoCard) => self.toast(Msg::Card(Card::None)),
            Some(Code::CardShared) => self.toast(Msg::Card(Card::Host)),
            Some(Code::CardOpen) => self.toast_for(Msg::CardOpen(what), TOAST_LONG_MS),
            Some(Code::CardUnreadable) => self.toast_for(Msg::CardUnreadable(what), TOAST_LONG_MS),
            Some(Code::Resources) => self.short(Wanted::Asked(what), holders),
            code => self.toast(Msg::Refused(what, code)),
        }
    }

    fn event(&mut self, e: Event) {
        match e {
            Event::Key { key, phase } => self.key(key, phase),
            Event::Control { control } => {
                self.st.cam.controls.insert(control.name.clone(), control);
            }
            Event::Running { running } => {
                // a recording that runs keeps the card it was taken for
                if running.record.as_ref().is_some_and(|r| r.state == RecordState::Running) {
                    self.kept(Asked::Record);
                }
                self.record_ms = match &running.record {
                    Some(r) => r.ms,
                    None => self.record_now(),
                };
                self.st.cam.running = running;
                self.st.cam.running_at = self.st.now;
            }
            Event::Files { version, total } => {
                self.st.cam.files = FilesInfo { version, total };
                self.want_window();
            }
            Event::PhotoSaved { file } => {
                self.kept(Asked::Photo);
                self.toast(Msg::PhotoSaved(basename(&file)));
            }
            Event::Recorded { file } => self.toast(Msg::Recorded(basename(&file), self.record_ms / 1000)),
            Event::Formatted => {
                self.kept(Asked::Format);
                self.toast(Msg::Formatted);
            }
            Event::Failed { what, code, holders, .. } => {
                let failed = match what {
                    Action::Photo => Some(Asked::Photo),
                    Action::Record => Some(Asked::Record),
                    Action::Format => Some(Asked::Format),
                    _ => None,
                };
                if failed.is_some() && self.lent == failed {
                    self.give_back();
                }
                // a video that did not start: its file alone again
                if what == Action::Play {
                    if let Screen::Gallery(g) = &mut self.st.screen {
                        if matches!(g.view, GalleryView::Video { .. }) {
                            g.view = GalleryView::Alone;
                        }
                    }
                }
                if code == Code::Resources {
                    self.short(Wanted::Failed(what), holders);
                } else {
                    self.toast(Msg::Failed(what, code));
                }
            }
            Event::Player { player } => self.player(player),
            Event::Card { card } => {
                self.st.cam.card = card;
                if card.at != Card::Camera && matches!(self.st.screen, Screen::Gallery(_)) {
                    self.leave_gallery(Origin::Live);
                    self.toast(Msg::Card(card.at));
                }
            }
            Event::Outputs { outputs } => self.st.cam.outputs = outputs,
            Event::Lost => {
                self.end_session();
                if self.resetting {
                    self.hello();
                } else {
                    self.st.link = Link::Waiting(self.st.now + RETRY_MS);
                }
            }
            Event::Resource { id, state: ResState::Failed { code, holders, .. } } => {
                let freeze = self.freeze_n().is_some_and(|n| frozen_of(&id) == Some(n));
                if freeze {
                    self.st.freeze = Freeze::Off;
                }
                match (code, freeze) {
                    (ResCode::Resources, true) => self.short(Wanted::Freeze, holders),
                    // a freeze the key has already moved past is not the gallery's
                    (ResCode::Resources, false) if !is_frozen(&id) => self.short(Wanted::Gallery, holders),
                    (_, true) => self.toast(Msg::NoFreeze),
                    _ => {}
                }
            }
            // the menu missing on an output for lack of resources is said once a revision, until the output presents
            // again; the compare that does not fit (beside a recording: the recording outranks the freeze) is refused,
            // back to the freeze that fitted. An output that cannot carry the menu at all (`refused`) leaves the user
            // nothing to do
            Event::NotShown { output, revision, code: Code::Resources, holders, .. } => {
                if !self.missing.contains(&output) {
                    self.missing.push(output);
                    if self.said_rev != Some(revision) {
                        self.said_rev = Some(revision);
                        match self.st.freeze {
                            Freeze::Compare(n) => {
                                self.st.freeze = Freeze::Frozen(n);
                                self.short(Wanted::Freeze, holders);
                            }
                            _ => self.short(Wanted::Menu(output), holders),
                        }
                    }
                }
            }
            // the toast could not be seen there until now: its own time starts again
            Event::Presented { output, .. } if self.missing.contains(&output) => {
                self.missing.retain(|o| *o != output);
                if let Some(t) = self.st.toast.take() {
                    self.toast_for(t.msg, t.ms);
                }
            }
            Event::Presented { .. } => {}
            Event::NotShown { .. } | Event::Resource { .. } | Event::Account { .. } => {}
        }
    }

    fn player(&mut self, player: Option<PlayerInfo>) {
        self.st.cam.player = player;
        self.st.cam.player_at = self.st.now;
        let Screen::Gallery(g) = &mut self.st.screen else { return };
        g.seek = None;
        let GalleryView::Video { file, seen, asked } = &mut g.view else { return };
        let (mut stop, mut play) = (false, None);
        match &self.st.cam.player {
            Some(p) if p.file == *file && *asked => match p.state {
                PlayerState::Starting | PlayerState::Stopping => {}
                // a clip at its end goes back to the strip, as the vendor's player did
                PlayerState::Ended => {
                    g.view = GalleryView::Strip;
                    stop = true;
                }
                PlayerState::Playing | PlayerState::Paused => *seen = true,
            },
            None if *seen => g.view = GalleryView::Strip,
            // the file before it stopped: this one asked for now
            None if !*asked => {
                *asked = true;
                play = Some(file.clone());
            }
            _ => {}
        }
        if stop {
            self.ask(Request::PlayStop, Pending::Quiet);
        }
        if let Some(file) = play {
            self.ask_clear(Request::Play { file }, Pending::Asked(Asked::Play));
        }
    }

    // ---- the keys

    /// Whether a held key repeats on this screen: the arrows that move, the zoom, and ◀ ▶ that change a number.
    fn repeats(&self, k: Key) -> bool {
        match k {
            Key::Up | Key::Down | Key::ZoomIn | Key::ZoomOut => true,
            Key::Left | Key::Right => matches!(&self.st.screen, Screen::Slider { .. } | Screen::Gallery(Gallery { view: GalleryView::Video { .. }, .. })),
            _ => false,
        }
    }

    fn key(&mut self, k: Key, phase: Phase) {
        match phase {
            Phase::Release => return,
            Phase::Repeat if !self.repeats(k) => return,
            _ => {}
        }
        let media = matches!(self.st.screen, Screen::Gallery(_));
        match k {
            // MENU shows the menu and hides whatever is open
            Key::Menu => match &self.st.screen {
                Screen::Live => self.st.screen = Screen::Menu,
                Screen::Gallery(g) if g.view != GalleryView::Strip => self.to_strip(),
                Screen::Gallery(_) => self.leave_gallery(Origin::Live),
                _ => self.st.screen = Screen::Live,
            },
            Key::Gallery if media => self.leave_gallery(Origin::Live),
            Key::Gallery => self.open_gallery(Origin::Live),
            _ if media => self.gallery_key(k),
            // the keys of their own, the same on every screen that shows the camera
            Key::ZoomIn | Key::ZoomOut => {
                self.adjust("zoom", if k == Key::ZoomIn { 1 } else { -1 });
                self.flash(Flash::Zoom);
            }
            Key::Mirror | Key::Flip => {
                let name = if k == Key::Mirror { "mirror" } else { "flip" };
                self.set(name, Value::Bool(!self.wanted_on(name)), None);
                self.flash(Flash::Mirror);
            }
            Key::Ae | Key::Awb => {
                let name = if k == Key::Ae { "exposure-lock" } else { "white-balance-lock" };
                let lock = !self.wanted_on(name);
                self.set(name, Value::Bool(lock), None);
                if !lock {
                    self.flash(if k == Key::Ae { Flash::AeAuto } else { Flash::WbAuto });
                }
            }
            Key::Cross => self.own(Pick::Cross(!self.st.own.cross)),
            Key::Picture => self.picture_round(),
            Key::Freeze => {
                self.st.freeze = match self.st.freeze {
                    Freeze::Off => {
                        self.frozen += 1;
                        Freeze::Frozen(self.frozen)
                    }
                    Freeze::Frozen(n) => Freeze::Compare(n),
                    Freeze::Compare(_) => Freeze::Off,
                }
            }
            Key::Play => {}
            Key::Photo => self.with_card(Then::Ask(Asked::Photo, Request::Photo)),
            Key::Record => match self.st.cam.running.record.as_ref().map(|r| r.state) {
                None => self.with_card(Then::Ask(Asked::Record, Request::Record)),
                Some(RecordState::Starting | RecordState::Running) => {
                    self.ask(Request::RecordStop, Pending::Asked(Asked::RecordStop));
                }
                Some(RecordState::Stopping) => {}
            },
            Key::Up | Key::Down | Key::Left | Key::Right => match self.st.screen.clone() {
                Screen::Menu => self.menu_key(k),
                Screen::Slider { item, round } => self.slider_key(k, item, round),
                Screen::Place(line) => self.place_key(k, line),
                Screen::Live | Screen::Gallery(_) => {}
            },
        }
    }

    /// ◐: the contrast slider, then saturation, sharpness, brightness, then closed.
    fn picture_round(&mut self) {
        let round = Item::PICTURE;
        match self.st.screen {
            Screen::Slider { item, round: start } if round.contains(&item) => {
                let next = round[(round.iter().position(|i| *i == item).expect("in the round") + 1) % round.len()];
                self.st.screen = if next == start { Screen::Live } else { Screen::Slider { item: next, round: start } };
            }
            _ => self.st.screen = Screen::Slider { item: Item::Contrast, round: Item::Contrast },
        }
    }

    fn menu_key(&mut self, k: Key) {
        let top = *self.st.menu.last().expect("the menu has its root");
        let page = self.page(top.level);
        let n = page.rows.len();
        match k {
            Key::Up | Key::Down if page.rows.iter().any(|r| r.act.is_some()) => {
                let mut sel = top.sel;
                loop {
                    sel = if k == Key::Down { (sel + 1) % n } else { (sel + n - 1) % n };
                    if page.rows[sel].act.is_some() {
                        break;
                    }
                }
                self.st.menu.last_mut().expect("the menu has its root").sel = sel;
            }
            Key::Left => self.pop(),
            Key::Right => {
                if let Some(Row { dim: false, act: Some(a), .. }) = page.rows.get(top.sel) {
                    self.act(*a);
                }
            }
            _ => {}
        }
    }

    /// The numbers of the slider's section that can be changed now, for ▲▼.
    fn near(&self, item: Item) -> Vec<Item> {
        let open = |i: &Item| match i.kind() {
            Kind::Number(c) => self.control(c).is_some_and(|c| c.available),
            _ => false,
        };
        Section::of(item).items().iter().copied().filter(open).collect()
    }

    /// How many numbers ▲▼ go between on the slider of `item`.
    pub fn near_count(&self, item: Item) -> usize {
        self.near(item).len()
    }

    fn slider_key(&mut self, k: Key, item: Item, round: Item) {
        let Kind::Number(control) = item.kind() else { unreachable!("a slider shows a number") };
        match k {
            Key::Left | Key::Right => self.adjust(control, if k == Key::Right { 1 } else { -1 }),
            _ => {
                let near = self.near(item);
                if let Some(at) = near.iter().position(|i| *i == item) {
                    let n = near.len();
                    let to = near[if k == Key::Down { (at + 1) % n } else { (at + n - 1) % n }];
                    self.st.screen = Screen::Slider { item: to, round };
                }
            }
        }
    }

    fn place_key(&mut self, k: Key, mut line: Line) {
        match k {
            Key::Right => {
                self.st.lines.push(line);
                self.st.screen = Screen::Live;
            }
            Key::Left => self.st.screen = Screen::Menu,
            _ => {
                // a horizontal line goes up with ▲; a vertical one goes right with ▲
                let back = (k == Key::Up) == (line.dir == Dir::H);
                line.pos = if back { line.pos.saturating_sub(LINE_STEP).max(LINE_EDGE) } else { (line.pos + LINE_STEP).min(1000 - LINE_EDGE) };
                self.st.screen = Screen::Place(line);
            }
        }
    }

    // ---- the menu's levels

    fn push(&mut self, level: Level) {
        let page = self.page(level);
        let mut sel = match level {
            Level::Values(item) => self.current(item).unwrap_or(0),
            _ => 0,
        };
        while page.rows.get(sel).is_some_and(|r| r.dim && r.act.is_none()) {
            sel += 1;
        }
        let sel = sel.min(page.rows.len().saturating_sub(1));
        self.st.menu.push(Spot { level, sel });
    }

    fn pop(&mut self) {
        if self.st.menu.len() > 1 {
            self.st.menu.pop();
        } else {
            self.st.screen = Screen::Live;
        }
    }

    fn act(&mut self, a: Act) {
        match a {
            Act::Open(level) => {
                if level == Level::About {
                    self.ask(Request::Status, Pending::Status);
                }
                self.push(level);
            }
            Act::Gallery => self.open_gallery(Origin::Menu),
            Act::Slider(item) => self.st.screen = Screen::Slider { item, round: item },
            Act::Pick(item, i) => {
                let in_list = matches!(self.st.menu.last(), Some(Spot { level: Level::Values(_), .. }));
                if let Some(p) = self.values(item).into_iter().nth(i).map(|v| v.0) {
                    self.pick(p);
                }
                if in_list {
                    self.pop();
                }
            }
            Act::Do(Item::ResetPicture) => {
                let back: Vec<(&'static str, Value)> = Item::PICTURE
                    .iter()
                    .filter_map(|i| i.control())
                    .filter_map(|c| self.control(c).filter(|c| c.value != c.default).map(|ctl| (c, ctl.default.clone())))
                    .collect();
                if back.is_empty() {
                    self.toast(Msg::PictureReset);
                } else {
                    self.next_group += 1;
                    let g = self.next_group;
                    self.groups.insert(g, Group { left: back.len(), failed: false, msg: Msg::PictureReset });
                    for (c, v) in back {
                        self.set(c, v, Some(g));
                    }
                }
            }
            Act::Do(Item::ClearLines) => {
                self.st.lines.clear();
                self.toast(Msg::LinesCleared);
            }
            Act::Do(_) => {}
            Act::Line(dir) => self.st.screen = Screen::Place(Line { dir, pos: 500 }),
            Act::Format => {
                self.with_card(Then::Ask(Asked::Format, Request::Format));
                self.pop();
            }
            Act::CardReader(on) => self.set("card-reader", Value::Bool(on), None),
            Act::ResetAll => {
                self.resetting = true;
                self.ask(Request::Reset, Pending::Asked(Asked::Reset));
                self.pop();
            }
            Act::Back => self.pop(),
        }
    }

    fn pick(&mut self, p: Pick) {
        match p {
            Pick::Set(name, v) => self.set(name, v, None),
            Pick::Mirror(m, f) => {
                if !self.value("mirror").is_some_and(|v| *v == Value::Bool(m)) {
                    self.set("mirror", Value::Bool(m), None);
                }
                if !self.value("flip").is_some_and(|v| *v == Value::Bool(f)) {
                    self.set("flip", Value::Bool(f), None);
                }
            }
            p @ (Pick::Cross(_) | Pick::Lang(_)) => self.own(p),
        }
    }

    fn own(&mut self, p: Pick) {
        match p {
            Pick::Cross(c) => self.st.own.cross = c,
            Pick::Lang(l) => self.st.own.lang = l,
            _ => unreachable!("the menu's own values"),
        }
        self.keep();
    }

    /// A value item's values: what each asks, its words, its own description.
    fn values(&self, item: Item) -> Vec<(Pick, String, Option<&'static str>)> {
        let w = self.words();
        match item {
            Item::Cross => vec![(Pick::Cross(false), w.off.into(), None), (Pick::Cross(true), w.on.into(), None)],
            Item::Language => Lang::ALL.iter().map(|l| (Pick::Lang(*l), l.words().name.into(), None)).collect(),
            Item::Mirror => (0..4).map(|i| (Pick::Mirror(i & 1 != 0, i & 2 != 0), w.mirrors[i].into(), None)).collect(),
            // which output gives way to the other: the precedence with USB or HDMI the higher of the two
            Item::Priority => {
                let Some(Value::Text(q)) = self.value("precedence") else { return Vec::new() };
                let Ok(q) = Precedence::parse(q) else { return Vec::new() };
                [(Who::Usb, Who::Hdmi, "USB"), (Who::Hdmi, Who::Usb, "HDMI")].into_iter().map(|(a, b, word)| (Pick::Set("precedence", Value::Text(q.with_above(a, b).to_string())), word.to_string(), None)).collect()
            }
            _ => {
                let Some(name) = item.control() else { return Vec::new() };
                let Some(c) = self.control(name) else { return Vec::new() };
                match c.kind {
                    ControlKind::Bool => {
                        let (off, on) = if item == Item::WhiteBalanceLock { (w.auto, w.locked) } else { (w.off, w.on) };
                        vec![(Pick::Set(name, Value::Bool(false)), off.into(), None), (Pick::Set(name, Value::Bool(true)), on.into(), None)]
                    }
                    ControlKind::Int => {
                        let (lo, hi) = (c.min.unwrap_or(0), c.max.unwrap_or(0));
                        (lo..=hi).map(|v| (Pick::Set(name, Value::Int(v)), w.levels.get(v as usize).map_or(v.to_string(), |l| l.name.into()), w.levels.get(v as usize).map(|l| l.help))).collect()
                    }
                    // the whole order is not shown in this menu: only its HDMI/USB part, as `Priority`
                    ControlKind::Order => Vec::new(),
                    ControlKind::Choice => c
                        .values
                        .iter()
                        // the HDMI off is not offered: it would take away the screen the menu is on
                        .filter(|v| !(item == Item::Hdmi && *v == "off" && c.value != Value::Text("off".into())))
                        .map(|v| (Pick::Set(name, Value::Text(v.clone())), choice_word(item, v, w), None))
                        .collect(),
                }
            }
        }
    }

    /// The place of a value item's current value among its values.
    fn current(&self, item: Item) -> Option<usize> {
        let now = match item {
            Item::Cross => Pick::Cross(self.st.own.cross),
            Item::Language => Pick::Lang(self.st.own.lang),
            Item::Mirror => {
                let i = self.mirror_index()?;
                Pick::Mirror(i & 1 != 0, i & 2 != 0)
            }
            _ => Pick::Set(item.control()?, self.value(item.control()?)?.clone()),
        };
        self.values(item).iter().position(|v| v.0 == now)
    }

    /// A number as its row and the slider show it.
    pub fn number(&self, item: Item) -> Option<String> {
        let Kind::Number(c) = item.kind() else { return None };
        let Some(Value::Int(v)) = self.value(c) else { return None };
        Some(if item == Item::ExposureBias {
            let sign = if *v > 0 { "+" } else if *v < 0 { "−" } else { "" };
            format!("{sign}{}", self.words().tenths(v.unsigned_abs()))
        } else {
            v.to_string()
        })
    }

    pub fn page(&self, level: Level) -> Page {
        let w = self.words();
        let row = |name: &str, value: String, help: Option<&str>, act: Option<Act>| Row { name: name.into(), value, help: help.map(String::from), dim: false, mark: false, act };
        let info = |name: &str, value: String| Row { dim: true, ..row(name, value, None, None) };
        match level {
            Level::Root => Page {
                title: w.menu.into(),
                rows: ROOT
                    .iter()
                    .map(|e| match e {
                        Entry::Section(s) => {
                            let n = (w.section)(*s);
                            row(n.name, "›".into(), Some(n.help), Some(Act::Open(Level::Section(*s))))
                        }
                        Entry::Gallery => row(w.gallery.name, "›".into(), Some(w.gallery.help), Some(Act::Gallery)),
                    })
                    .collect(),
            },
            Level::Section(s) => Page { title: (w.section)(s).name.into(), rows: s.items().iter().map(|i| self.item_row(*i)).collect() },
            Level::Values(item) => {
                let cur = self.current(item);
                let rows = self.values(item).into_iter().enumerate().map(|(i, (_, word, help))| Row { mark: cur == Some(i), ..row(&word, String::new(), Some(help.unwrap_or((w.item)(item).help)), Some(Act::Pick(item, i))) }).collect();
                Page { title: (w.item)(item).name.into(), rows }
            }
            Level::AddLine => Page {
                title: (w.item)(Item::AddLine).name.into(),
                rows: [(w.line_h, Dir::H), (w.line_v, Dir::V)].into_iter().map(|(l, d)| row(l.name, String::new(), Some(l.help), Some(Act::Line(d)))).collect(),
            },
            Level::Card => {
                let cam = &self.st.cam;
                let on = self.wanted_on("card-reader");
                let reader = || row(w.card_reader.name, (if on { w.on } else { w.off }).into(), Some(w.card_reader.help), Some(Act::CardReader(!on)));
                let mut rows = match (cam.card.at, cam.card.free.zip(cam.card.total)) {
                    // the card is unmounted while it is formatted
                    _ if cam.running.format => vec![info(w.card_state, w.card_formatting.into())],
                    (Card::None, _) => vec![info(w.card_none, String::new()), reader()],
                    (Card::Host, _) => vec![info(w.card_state, w.card_host.into()), reader()],
                    (Card::Unreadable, _) => vec![info(w.card_state, w.card_unreadable.into()), reader()],
                    (Card::Camera, Some((free, total))) => vec![info(w.card_free, (w.of)(&gb(free, w), &gb_unit(total, w)))],
                    (Card::Camera, None) => vec![info(w.card_state, w.card_in.into())],
                };
                if !cam.running.format {
                    if cam.card.at == Card::Camera {
                        rows.push(reader());
                    }
                    // formatting takes the card from the computer as a photo does
                    if cam.card.at != Card::None {
                        rows.push(row(w.format.name, "›".into(), Some(w.format.help), Some(Act::Open(Level::FormatAsk))));
                    }
                }
                Page { title: (w.item)(Item::Card).name.into(), rows }
            }
            Level::FormatAsk => Page {
                title: w.format_ask.into(),
                rows: vec![row(w.cancel, String::new(), Some(w.format_cancel_help), Some(Act::Back)), row(w.format.name, String::new(), Some(w.format_yes_help), Some(Act::Format))],
            },
            Level::ResetAsk => Page {
                title: w.reset_ask.into(),
                rows: vec![row(w.cancel, String::new(), Some(w.reset_cancel_help), Some(Act::Back)), row(w.reset_yes, String::new(), Some((w.item)(Item::Reset).help), Some(Act::ResetAll))],
            },
            Level::About => {
                let ours = env!("CARGO_PKG_VERSION").rsplit_once('.').map_or(env!("CARGO_PKG_VERSION"), |v| v.0);
                let version = format!("{ours} · {}", self.st.build.clone().unwrap_or_else(|| text::WAITING.into()));
                let hdmi = self.hdmi_timing().unwrap_or_else(|| w.off.into());
                let usb = match self.output(Output::Usb) {
                    Some(o) if o.fps.is_some() => format!("{}×{} · {} {}", o.w, o.h, o.fps.unwrap_or(0), w.fps),
                    _ => w.no_stream.into(),
                };
                Page { title: (w.item)(Item::About).name.into(), rows: vec![info(text::ABOUT_NAME, version), info("HDMI", hdmi), info("USB", usb)] }
            }
        }
    }

    fn item_row(&self, item: Item) -> Row {
        let w = self.words();
        let named = (w.item)(item);
        let (name, help) = (named.name.to_string(), Some(named.help.to_string()));
        let available = |c: &str| self.control(c).is_some_and(|c| c.available);
        match item.kind() {
            Kind::Number(c) => Row { name, value: self.number(item).unwrap_or_else(|| text::NO_VALUE.into()), help, dim: !available(c), mark: false, act: Some(Act::Slider(item)) },
            Kind::Value => {
                let values = self.values(item);
                let cur = self.current(item);
                let mut value = cur.and_then(|i| values.get(i)).map_or_else(|| text::NO_VALUE.into(), |v| v.1.clone());
                if values.len() > 2 {
                    value.push_str(" ›");
                }
                let act = match values.len() {
                    0 | 1 => None,
                    2 => Some(Act::Pick(item, 1 - cur.unwrap_or(1))),
                    _ => Some(Act::Open(Level::Values(item))),
                };
                let dim = match item {
                    Item::Mirror => !(available("mirror") && available("flip")),
                    _ => item.control().is_some_and(|c| !available(c)),
                } || act.is_none();
                Row { name, value, help, dim, mark: false, act }
            }
            Kind::Action => {
                let (value, act) = match item {
                    Item::AddLine => ("›", Act::Open(Level::AddLine)),
                    Item::Card => ("›", Act::Open(Level::Card)),
                    Item::Reset => ("›", Act::Open(Level::ResetAsk)),
                    Item::About => ("›", Act::Open(Level::About)),
                    _ => ("", Act::Do(item)),
                };
                Row { name, value: value.into(), help, dim: false, mark: false, act: Some(act) }
            }
        }
    }

    /// The words of the menu's language.
    pub fn words(&self) -> &'static Words {
        self.st.own.lang.words()
    }

    pub fn output(&self, name: Output) -> Option<&OutputInfo> {
        self.st.cam.outputs.iter().find(|o| o.name == name)
    }

    /// The HDMI mode the monitor gets now.
    fn hdmi_timing(&self) -> Option<String> {
        self.output(Output::Hdmi).and_then(|o| o.timing).map(|t| t.name().to_string())
    }

    /// A chip that came for a moment, in words.
    pub fn flash_text(&self) -> Option<String> {
        let w = self.words();
        let zoom = || format!("×{}", w.tenths(self.zoom().unsigned_abs()));
        Some(match &self.st.flash.as_ref()?.0 {
            Flash::Zoom => zoom(),
            Flash::Mirror => (w.mirror)(self.mirror_index().map_or(text::NO_VALUE, |i| w.mirrors[i])),
            Flash::AeAuto => w.ae_auto.into(),
            Flash::WbAuto => w.wb_auto.into(),
            Flash::Start => {
                let card = &self.st.cam.card;
                let sd = match (card.at, card.free) {
                    (Card::Camera, Some(free)) => format!("{} {}", w.sd, gb_unit(free, w)),
                    (Card::Camera, None) => w.sd.into(),
                    (Card::None, _) => w.no_sd.into(),
                    (Card::Host, _) => w.sd_host.into(),
                    (Card::Unreadable, _) => w.sd_unreadable.into(),
                };
                let mut parts: Vec<String> = self.hdmi_timing().into_iter().collect();
                parts.push(zoom());
                parts.push(sd);
                parts.join(" · ")
            }
        })
    }

    // ---- the gallery

    /// The gallery opened, the card taken from the computer first as for a photo.
    fn open_gallery(&mut self, from: Origin) {
        self.with_card(Then::Gallery(from));
    }

    /// The gallery opened on the camera's card, or why not (false).
    fn show_gallery(&mut self, from: Origin) -> bool {
        let cam = &self.st.cam;
        let refusal = if cam.card.at == Card::Unreadable {
            Some(Msg::CardUnreadable(Asked::Gallery))
        } else if cam.card.at != Card::Camera {
            Some(Msg::Card(cam.card.at))
        } else if cam.running.record.is_some() {
            Some(Msg::Recording)
        } else if cam.files.total == 0 {
            Some(Msg::NoFiles)
        } else {
            None
        };
        if let Some(m) = refusal {
            let ms = if matches!(m, Msg::CardUnreadable(_)) { TOAST_LONG_MS } else { TOAST_MS };
            self.toast_for(m, ms);
            return false;
        }
        self.st.screen = Screen::Gallery(Gallery { from, at: 0, path: None, view: GalleryView::Strip, list: None, back: false, asking: false, seek: None });
        self.want_window();
        true
    }

    // ---- the card for what is pressed

    /// What is pressed done on the card: the card taken from the computer first when it has it (the card reader mode
    /// off), then what was pressed, the card given back when that does not get there.
    fn with_card(&mut self, then: Then) {
        if self.st.cam.card.at == Card::Host {
            self.ask(Request::Set { name: "card-reader".into(), value: Value::Bool(false) }, Pending::Took(then));
        } else {
            self.then(then, false);
        }
    }

    /// What was pressed, the card on the camera; `took`: taken from the computer for it.
    fn then(&mut self, then: Then, took: bool) {
        match then {
            Then::Ask(what, req) => {
                self.ask(req, if took { Pending::Lent(what) } else { Pending::Asked(what) });
            }
            Then::Gallery(from) => {
                if !self.show_gallery(from) && took {
                    self.give_back();
                }
            }
        }
    }

    /// What the card was taken for got there: the card stays.
    fn kept(&mut self, what: Asked) {
        if self.lent.as_ref() == Some(&what) {
            self.lent = None;
        }
    }

    /// The card back to the computer, as it was before the press (the card reader mode on).
    fn give_back(&mut self) {
        self.lent = None;
        self.ask(Request::Set { name: "card-reader".into(), value: Value::Bool(true) }, Pending::Quiet);
    }

    fn leave_gallery(&mut self, to: Origin) {
        if matches!(&self.st.screen, Screen::Gallery(g) if matches!(g.view, GalleryView::Video { .. })) {
            self.ask(Request::PlayStop, Pending::Quiet);
        }
        self.st.screen = if to == Origin::Menu { Screen::Menu } else { Screen::Live };
    }

    /// From a file alone or a video back to the strip.
    fn to_strip(&mut self) {
        let Screen::Gallery(g) = &mut self.st.screen else { return };
        let video = matches!(g.view, GalleryView::Video { .. });
        g.view = GalleryView::Strip;
        if video {
            self.ask(Request::PlayStop, Pending::Quiet);
        }
    }

    /// The chosen file, once its window is read.
    pub fn chosen(&self) -> Option<&FileEntry> {
        let Screen::Gallery(g) = &self.st.screen else { return None };
        let w = g.list.as_ref()?;
        g.at.checked_sub(w.from).and_then(|i| w.list.get(i))
    }

    /// The card's list read around the chosen file when it is not, or no longer, around it.
    fn want_window(&mut self) {
        let (total, version) = (self.st.cam.files.total, self.st.cam.files.version);
        let Screen::Gallery(g) = &mut self.st.screen else { return };
        if g.asking {
            return;
        }
        let fresh = g.list.as_ref().is_some_and(|w| {
            let end = w.from + w.list.len();
            w.version == version && g.at >= w.from && g.at < end && (g.at >= w.from + MARGIN || w.from == 0) && (g.at + MARGIN < end || end >= total)
        });
        if fresh {
            return;
        }
        let from = g.at.saturating_sub(WINDOW / 2).min(total.saturating_sub(WINDOW));
        g.asking = true;
        self.ask(Request::Files { from, count: WINDOW }, Pending::Files);
    }

    fn window(&mut self, w: FilesWindow) {
        let files = &mut self.st.cam.files;
        if w.version > files.version {
            *files = FilesInfo { version: w.version, total: w.total };
        }
        let current = w.version == files.version;
        let Screen::Gallery(g) = &mut self.st.screen else { return };
        g.asking = false;
        if current && w.total == 0 {
            self.leave_gallery(Origin::Live);
            return self.toast(Msg::NoFiles);
        }
        if current {
            // the choice kept by the file, not by its place, when the list changed under it
            if let Some(i) = g.path.as_ref().and_then(|p| w.list.iter().position(|e| e.path == *p)) {
                g.at = w.from + i;
            }
            g.at = g.at.min(w.total - 1);
            g.path = g.at.checked_sub(w.from).and_then(|i| w.list.get(i)).map(|e| e.path.clone());
            g.list = Some(w);
        }
        self.want_window();
    }

    fn gallery_key(&mut self, k: Key) {
        let total = self.st.cam.files.total;
        let chosen = self.chosen().cloned();
        let player_ms = self.player_ms();
        let player = self.st.cam.player.clone();
        let Screen::Gallery(g) = &mut self.st.screen else { return };
        let mut send: Option<(Request, Pending)> = None;
        let mut leave = None;
        match k {
            Key::Up | Key::Down if total > 0 => {
                g.at = if k == Key::Down { (g.at + 1) % total } else { (g.at + total - 1) % total };
                g.back = k == Key::Up;
                g.path = g.list.as_ref().and_then(|w| g.at.checked_sub(w.from).and_then(|i| w.list.get(i))).map(|e| e.path.clone());
                g.seek = None;
                if matches!(g.view, GalleryView::Video { .. }) {
                    g.view = GalleryView::Alone;
                    send = Some((Request::PlayStop, Pending::Quiet));
                }
            }
            Key::Play | Key::Left | Key::Right if matches!(g.view, GalleryView::Video { .. }) => {
                let seen = matches!(g.view, GalleryView::Video { seen: true, .. });
                if let Some(p) = player.filter(|_| seen) {
                    let req = match (k, p.state) {
                        (_, PlayerState::Starting | PlayerState::Stopping) => return,
                        (Key::Play, PlayerState::Playing) => Request::PlayPause,
                        (Key::Play, PlayerState::Paused) => Request::PlayResume,
                        (Key::Play, PlayerState::Ended) => Request::PlaySeek { ms: 0 },
                        _ => {
                            let base = g.seek.or(player_ms).unwrap_or(0);
                            let ms = if k == Key::Right { (base + SEEK_MS).min(p.duration_ms) } else { base.saturating_sub(SEEK_MS) };
                            g.seek = Some(ms);
                            Request::PlaySeek { ms }
                        }
                    };
                    send = Some((req, Pending::Asked(Asked::Player)));
                }
            }
            Key::Right | Key::Play => match chosen {
                // asked once the player is free: the file before may be stopping still
                Some(f) if f.kind == FileKind::Video => {
                    let free = player.is_none();
                    g.view = GalleryView::Video { file: f.path.clone(), seen: false, asked: free };
                    if free {
                        send = Some((Request::Play { file: f.path }, Pending::Asked(Asked::Play)));
                    }
                }
                Some(_) if k == Key::Right => g.view = GalleryView::Alone,
                _ => {}
            },
            Key::Left if g.view == GalleryView::Alone => g.view = GalleryView::Strip,
            Key::Left => leave = Some(g.from),
            _ => {}
        }
        if let Some((req, p)) = send {
            if matches!(req, Request::Play { .. }) {
                self.ask_clear(req, p);
            } else {
                self.ask(req, p);
            }
        }
        match leave {
            Some(to) => self.leave_gallery(to),
            None => self.want_window(),
        }
    }
}

/// A choice control's value in words.
fn choice_word(item: Item, v: &str, w: &Words) -> String {
    match (item, v) {
        (Item::PowerLine, "off") | (Item::Hdmi, "off") => w.off.into(),
        (Item::PowerLine, hz) => format!("{hz} {}", w.hz),
        (Item::Hdmi, "auto") => w.auto.into(),
        (Item::RecordSize, "4k") => "4K".into(),
        (Item::RecordSize, p) => format!("{p}p"),
        (_, v) => v.into(),
    }
}

#[cfg(test)]
mod tests;

/// Bytes as gigabytes (10⁹ bytes, as a card's label counts them) with one decimal.
fn gb(bytes: u64, w: &Words) -> String {
    w.tenths((bytes as f64 / 1e8).round() as u64)
}

fn gb_unit(bytes: u64, w: &Words) -> String {
    format!("{} {}", gb(bytes, w), w.gb)
}
