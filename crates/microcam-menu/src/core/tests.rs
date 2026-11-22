//! The mockup's behaviour, key by key, against a minimal microcamd: its controls are `microcam_core::controls`' own
//! list (the same rules of steps, clamping and availability), the rest a few lines each. The clock is the test's.

use microcam_core::controls::{apply, Change, Code, Drive, Held, Holder, List, Refusal, Value};
use microcam_core::files::{FileEntry, FileKind};
use microcam_core::hdmi::{Monitor, MonitorCaps, Timing};
use microcam_core::{Codec, Size};
use microcam_core::keys::{Key, Phase};
use microcam_core::policy::Prefs;
use microcam_core::protocol::{Card, CardInfo, DaemonState, Output as Out, OutputInfo as OutInfo, Envelope, Event, FilesInfo, FilesWindow, Hello, Limits, PlayerInfo, PlayerState, RecordRun, RecordState, Request, ResCode, ResState, Response, Running, Snapshot, Status};
use microcam_core::reconcile::{Current, HdmiState};

use super::*;

/// Which requests a rule of the fake takes.
type Matches = fn(&Request) -> bool;

/// microcamd as far as the menu sees it.
struct Fake {
    list: List,
    prefs: Prefs,
    monitor: Monitor,
    card: Card,
    /// the computer keeps the card open (it has it mounted)
    open: bool,
    /// the camera cannot read the card's file system
    unreadable: bool,
    /// the card's free bytes
    free: u64,
    /// a `play-stop` leaves the player stopping until the test lets it go
    slow_stop: bool,
    /// newest first
    files: Vec<FileEntry>,
    version: u64,
    running: Running,
    record_start: u64,
    player: Option<PlayerInfo>,
    record: Option<serde_json::Value>,
    /// another menu holds the scene
    other: bool,
    /// the next request this matches is refused for lack of resources, these holding them
    short: Option<(Matches, Vec<Holder>)>,
    /// every request as it came
    log: Vec<Request>,
}

fn file(n: u32) -> FileEntry {
    if n % 5 == 3 {
        FileEntry { name: format!("V{n:05}.mp4"), path: format!("/mnt/VIDEO/V{n:05}.mp4"), kind: FileKind::Video, number: n, bytes: 1 }
    } else {
        FileEntry { name: format!("P{n:05}.jpg"), path: format!("/mnt/PHOTO/P{n:05}.jpg"), kind: FileKind::Photo, number: n, bytes: 1 }
    }
}

/// A video's length: 20 s and 7 more for each number.
fn duration(path: &str) -> u64 {
    let n: u64 = path.trim_end_matches(".mp4").rsplit('V').next().unwrap().parse().unwrap();
    (20 + 7 * n) * 1000
}

impl Fake {
    fn new() -> Self {
        let prefs = Prefs::default();
        let monitor = Monitor::Present(MonitorCaps { modes: vec![(Size::new(3840, 2160), 30), (Size::new(1920, 1080), 60)] });
        let policy = microcam_core::policy::Policy::parse(include_str!("../../../../camera/policy.toml")).unwrap();
        let mut list = List::new(&prefs, &policy);
        list.follow(&prefs, &monitor);
        // the card on the camera: the card reader mode, on by default, switched off
        list.store("card-reader", Value::Bool(false), microcam_core::controls::By::User, &microcam_core::controls::Now::default());
        Fake {
            list,
            prefs,
            monitor,
            card: Card::Camera,
            open: false,
            unreadable: false,
            free: 12_400 << 20,
            slow_stop: false,
            files: (1..=40).rev().map(file).collect(),
            version: 1,
            running: Running::default(),
            record_start: 0,
            player: None,
            // the tests read the menu's words in Russian, the language its record keeps
            record: Some(serde_json::json!({"v": 1, "lang": "ru", "cross": true})),
            other: false,
            short: None,
            log: Vec::new(),
        }
    }

    fn info(&self) -> FilesInfo {
        FilesInfo { version: self.version, total: self.files.len() }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { controls: self.list.all(), running: self.running.clone(), files: self.info(), outputs: self.outputs(), player: self.player.clone(), card: self.card_info(), record: self.record.clone() }
    }

    fn card_info(&self) -> CardInfo {
        let here = self.card == Card::Camera;
        CardInfo { at: self.card, free: here.then_some(self.free), total: here.then_some(29_700 << 20) }
    }

    fn outputs(&self) -> Vec<OutInfo> {
        let o = |name| OutInfo { name, w: 3840, h: 2160, level: 3, paused: None, timing: None, codec: None, fps: None };
        vec![OutInfo { timing: Some(Timing::P2160_30), ..o(Out::Hdmi) }, OutInfo { codec: Some(Codec::Mjpeg), fps: Some(45), ..o(Out::Usb) }]
    }

    /// The player gone after a slow stop.
    fn stopped(&mut self) -> Input {
        self.player = None;
        self.player_event()
    }

    fn control_events(&self, names: Vec<String>) -> Vec<Input> {
        names.into_iter().map(|n| Input::Event(Event::Control { control: self.list.get(&n).unwrap().clone() })).collect()
    }

    fn change(&mut self, name: &str, ch: Change) -> (Response, Vec<Input>) {
        let v = match self.list.target(name, ch) {
            Ok(v) => v,
            Err(r) => return (Response::refused(r), Vec::new()),
        };
        let changed = if self.list.drive(name) == Some(Drive::Built) {
            apply(name, &v, &mut self.prefs).unwrap();
            self.list.follow(&self.prefs, &self.monitor)
        } else {
            self.list.store(name, v.clone(), microcam_core::controls::By::User, &microcam_core::controls::Now::default())
        };
        (Response::of(Ok(v)), self.control_events(changed))
    }

    fn card_free(&self) -> Result<(), Refusal> {
        match self.card {
            Card::None => Err(Refusal::new(Code::NoCard, "no card")),
            Card::Host => Err(Refusal::new(Code::CardShared, "with the host")),
            Card::Unreadable => Err(Refusal::new(Code::CardUnreadable, "cannot read it")),
            Card::Camera => Ok(()),
        }
    }

    /// The card to the computer, as microcamd's card reader mode switched on: its `card` event.
    fn share(&mut self) -> Result<Vec<Input>, Refusal> {
        match self.card {
            Card::None => Err(Refusal::new(Code::NoCard, "no card")),
            Card::Host => Ok(vec![]),
            Card::Camera | Card::Unreadable => {
                self.card = Card::Host;
                Ok(vec![Input::Event(Event::Card { card: self.card_info() })])
            }
        }
    }

    /// The card back from the computer, as microcamd takes it: its `card` event and the mode off, or refused while the
    /// computer keeps it open.
    fn take(&mut self) -> Result<Vec<Input>, Refusal> {
        match self.card {
            Card::Host if self.open => Err(Refusal::new(Code::CardOpen, "the USB host keeps the card open")),
            Card::Host => {
                self.card = if self.unreadable { Card::Unreadable } else { Card::Camera };
                let mut ev = vec![Input::Event(Event::Card { card: self.card_info() })];
                ev.extend(self.change("card-reader", Change::Set(Value::Bool(false))).1);
                Ok(ev)
            }
            _ => Ok(vec![]),
        }
    }

    fn running(&self) -> Input {
        Input::Event(Event::Running { running: self.running.clone() })
    }

    fn new_file(&mut self, kind: FileKind) -> FileEntry {
        let n = self.files.first().map_or(1, |f| f.number + 1);
        let mut f = file(n);
        if (f.kind == FileKind::Video) != (kind == FileKind::Video) {
            f = if kind == FileKind::Video {
                FileEntry { name: format!("V{n:05}.mp4"), path: format!("/mnt/VIDEO/V{n:05}.mp4"), kind, number: n, bytes: 1 }
            } else {
                FileEntry { name: format!("P{n:05}.jpg"), path: format!("/mnt/PHOTO/P{n:05}.jpg"), kind, number: n, bytes: 1 }
            };
        }
        f
    }

    fn added(&mut self, f: FileEntry) -> Input {
        self.files.insert(0, f);
        self.version += 1;
        Input::Event(Event::Files { version: self.version, total: self.files.len() })
    }

    fn player_event(&self) -> Input {
        Input::Event(Event::Player { player: self.player.clone() })
    }

    /// The request's reply and the events it brings, in the order microcamd sends them.
    fn handle(&mut self, now: u64, e: Envelope) -> Vec<Input> {
        self.log.push(e.req.clone());
        if self.short.as_ref().is_some_and(|(m, _)| m(&e.req)) {
            let (_, holders) = self.short.take().unwrap();
            return vec![Input::Reply(Response { seq: e.seq, ..Response::refused(Refusal::resources("short", holders)) })];
        }
        let ok = Response::ok;
        let refused = |r| (vec![], Response::refused(r), vec![]);
        // events before the reply, the reply, events after it
        let (before, reply, after): (Vec<Input>, Response, Vec<Input>) = match e.req {
            Request::Hello { take } if self.other && !take => (vec![], Response::err("another menu holds the scene"), vec![]),
            Request::Hello { .. } => {
                self.other = false;
                let limits = Limits { nodes: 4096, depth: 32, segments: 65536, text: 1024, text_total: 16384, levels: 3, menu_bytes: 1 << 26, picture_side: 4096 };
                (vec![], Response { snapshot: Some(self.snapshot()), hello: Some(Hello { protocol: 5, scene: 2, limits }), ..ok() }, vec![])
            }
            // the card moved first, the mode stored then, both told before the answer
            Request::Set { name, value } if name == "card-reader" => match if value == Value::Bool(true) { self.share() } else { self.take() } {
                Err(r) => refused(r),
                Ok(mut ev) => {
                    let (r, told) = self.change(&name, Change::Set(value));
                    ev.extend(told);
                    (ev, r, vec![])
                }
            },
            Request::Set { name, value } => {
                let (r, ev) = self.change(&name, Change::Set(value));
                (vec![], r, ev)
            }
            Request::Adjust { name, steps } => {
                let (r, ev) = self.change(&name, Change::Adjust(steps));
                (vec![], r, ev)
            }
            Request::Keep { record } => {
                self.record = Some(record);
                (vec![], ok(), vec![])
            }
            Request::Photo => match self.card_free() {
                Err(r) => refused(r),
                Ok(()) if self.player.is_some() => refused(Refusal::new(Code::Playing, "a file plays")),
                Ok(()) => {
                    let f = self.new_file(FileKind::Photo);
                    self.running.photo = Some(f.path.clone());
                    let busy = self.running();
                    self.running.photo = None;
                    let path = f.path.clone();
                    let files = self.added(f);
                    (vec![busy], ok(), vec![Input::Event(Event::PhotoSaved { file: path }), self.running(), files])
                }
            },
            Request::Record => match self.card_free() {
                Err(r) => refused(r),
                Ok(()) => {
                    let f = self.new_file(FileKind::Video);
                    self.running.record = Some(RecordRun { file: f.path, state: RecordState::Starting, ms: 0 });
                    let starting = self.running();
                    self.running.record.as_mut().unwrap().state = RecordState::Running;
                    self.record_start = now;
                    (vec![starting], ok(), vec![self.running()])
                }
            },
            Request::RecordStop => match self.running.record.clone() {
                None => refused(Refusal::new(Code::Refused, "no recording")),
                Some(r) => {
                    self.running.record = Some(RecordRun { state: RecordState::Stopping, ms: now - self.record_start, ..r.clone() });
                    let stopping = self.running();
                    self.running.record = None;
                    let mut f = self.new_file(FileKind::Video);
                    f.path = r.file.clone();
                    let files = self.added(f);
                    (vec![stopping], ok(), vec![Input::Event(Event::Recorded { file: r.file }), self.running(), files])
                }
            },
            Request::Format => match if self.card == Card::Unreadable { Ok(()) } else { self.card_free() } {
                Err(r) => refused(r),
                Ok(()) => {
                    self.card = Card::Camera;
                    self.running.format = true;
                    let busy = self.running();
                    self.running.format = false;
                    self.files.clear();
                    self.version += 1;
                    self.free = 29_600 << 20;
                    let card = Input::Event(Event::Card { card: self.card_info() });
                    (vec![busy], ok(), vec![Input::Event(Event::Formatted), self.running(), Input::Event(Event::Files { version: self.version, total: 0 }), card])
                }
            },
            Request::Reset => {
                let (told, build) = self.list.reset();
                for (name, v) in &build {
                    apply(name, v, &mut self.prefs).unwrap();
                }
                let mut names = told;
                names.extend(self.list.follow(&self.prefs, &self.monitor));
                self.record = None;
                let mut before = self.control_events(names);
                before.push(Input::Event(Event::Lost));
                (before, ok(), vec![])
            }
            Request::Files { from, count } => {
                let list = self.files.iter().skip(from).take(count).cloned().collect();
                (vec![], Response { files: Some(FilesWindow { from, total: self.files.len(), version: self.version, list }), ..ok() }, vec![])
            }
            Request::Play { .. } if self.running.record.is_some() => refused(Refusal::new(Code::Recording, "recording")),
            Request::Play { .. } if self.player.is_some() => refused(Refusal::new(Code::Busy, "the player starts or stops")),
            // answered at once, the player starting before the answer, playing after it
            Request::Play { file } => {
                self.player = Some(PlayerInfo { duration_ms: 0, file: file.clone(), state: PlayerState::Starting, ms: 0 });
                let starting = self.player_event();
                self.player = Some(PlayerInfo { duration_ms: duration(&file), file, state: PlayerState::Playing, ms: 0 });
                (vec![starting], ok(), vec![self.player_event()])
            }
            Request::PlayPause | Request::PlayResume | Request::PlaySeek { .. } => {
                let Some(p) = self.player.as_mut() else { return vec![Input::Reply(Response { seq: e.seq, ..Response::err("nothing plays") })] };
                match e.req {
                    Request::PlayPause => p.state = PlayerState::Paused,
                    Request::PlayResume => p.state = PlayerState::Playing,
                    Request::PlaySeek { ms } => p.ms = ms.min(p.duration_ms),
                    _ => unreachable!(),
                }
                (vec![], ok(), vec![self.player_event()])
            }
            Request::PlayStop => match self.player.as_mut() {
                None => refused(Refusal::new(Code::Unavailable, "nothing plays")),
                Some(p) => {
                    p.state = PlayerState::Stopping;
                    let stopping = self.player_event();
                    let after = if self.slow_stop { vec![] } else { vec![self.stopped()] };
                    (vec![stopping], ok(), after)
                }
            },
            Request::Status => {
                let current = Current { hdmi: Some(HdmiState { timing: Timing::P2160_30 }), ..Current::default() };
                let s = Status {
                    state: DaemonState::Running,
                    uptime_s: 1,
                    monitor: String::new(),
                    host_stream: None,
                    desired: None,
                    current,
                    last_error: None,
                    last_transition: None,
                    transitions: 0,
                    pump: None,
                    usb_delay: None,
                    usb_ui: None,
                    media_ui: None,
                    ui_levels: [3, 3, 0],
                    build: "524f825 2026-11-13T21:03Z".into(),
                    scene: true,
                    player: None,
                    card: self.card_info(),
                    lists: Vec::new(),
                };
                (vec![], Response { status: Some(s), ..ok() }, vec![])
            }
            r => panic!("the menu asked what it should not: {r:?}"),
        };
        let mut out = before;
        out.push(Input::Reply(Response { seq: e.seq, ..reply }));
        out.extend(after);
        out
    }
}

/// The core and the fake, joined: every request answered at once, the clock moved by the test.
struct Rig {
    core: Core,
    cam: Fake,
    now: u64,
    wake: Option<u64>,
    /// what the core asked since the last look
    asked: Vec<Request>,
    /// requests held back, not handed to the fake (a broken link)
    hold: bool,
}

impl Rig {
    fn with(cam: Fake, take: bool) -> Rig {
        let mut r = Rig { core: Core::new(take), cam, now: 1000, wake: None, asked: Vec::new(), hold: false };
        r.feed(Input::Connected);
        r.asked.clear();
        r
    }

    fn new() -> Rig {
        Rig::with(Fake::new(), false)
    }

    fn feed(&mut self, input: Input) {
        let mut queue = std::collections::VecDeque::from([input]);
        while let Some(i) = queue.pop_front() {
            for fx in self.core.step(self.now, i) {
                match fx {
                    Effect::Wake(at) => self.wake = Some(at),
                    Effect::Send(e) | Effect::SendClear(e) => {
                        self.asked.push(e.req.clone());
                        if !self.hold {
                            queue.extend(self.cam.handle(self.now, e));
                        }
                    }
                }
            }
        }
    }

    fn event(&mut self, e: Event) {
        self.feed(Input::Event(e));
    }

    /// The time moved on by `ms`, the core woken whenever it asked.
    fn wait(&mut self, ms: u64) {
        let end = self.now + ms;
        while let Some(at) = self.wake.filter(|at| *at <= end) {
            self.wake = None;
            self.now = self.now.max(at);
            self.feed(Input::Clock);
        }
        self.now = end;
        self.feed(Input::Clock);
    }

    fn phase(&mut self, key: Key, phase: Phase) {
        self.event(Event::Key { key, phase });
    }

    /// A short press: press, then release.
    fn press(&mut self, key: Key) {
        self.phase(key, Phase::Press);
        self.phase(key, Phase::Release);
    }

    fn keys(&mut self, keys: &[Key]) {
        for k in keys {
            self.press(*k);
        }
    }

    /// A key held: the press, the kernel's repeats from 505 ms every 140 ms, the release.
    fn hold(&mut self, key: Key, repeats: usize) {
        self.phase(key, Phase::Press);
        self.wait(505);
        for i in 0..repeats {
            if i > 0 {
                self.wait(140);
            }
            self.phase(key, Phase::Repeat);
        }
        self.phase(key, Phase::Release);
    }

    fn asked(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.asked)
    }

    fn page(&self) -> Page {
        self.core.page(self.core.st.menu.last().unwrap().level)
    }

    fn title(&self) -> String {
        self.page().title
    }

    fn names(&self) -> Vec<String> {
        self.page().rows.into_iter().map(|r| r.name).collect()
    }

    fn chosen(&self) -> String {
        let spot = self.core.st.menu.last().unwrap();
        self.page().rows[spot.sel].name.clone()
    }

    fn row(&self, name: &str) -> Row {
        self.page().rows.into_iter().find(|r| r.name == name).unwrap_or_else(|| panic!("no row {name} in {:?}", self.names()))
    }

    fn value(&self, name: &str) -> String {
        self.row(name).value
    }

    fn help(&self) -> String {
        let spot = self.core.st.menu.last().unwrap();
        self.page().rows[spot.sel].help.clone().unwrap_or_default()
    }

    fn toast(&self) -> Option<String> {
        self.core.st.toast.as_ref().map(|t| t.msg.say(self.core.st.own.lang))
    }

    fn int(&self, name: &str) -> i64 {
        match self.cam.list.get(name).unwrap().value {
            Value::Int(i) => i,
            ref v => panic!("{name} is {v:?}"),
        }
    }

    fn on(&self, name: &str) -> bool {
        self.cam.list.get(name).unwrap().value == Value::Bool(true)
    }

    fn screen(&self) -> &Screen {
        &self.core.st.screen
    }

    fn gallery(&self) -> &Gallery {
        match self.screen() {
            Screen::Gallery(g) => g,
            s => panic!("not the gallery: {s:?}"),
        }
    }

    fn chosen_file(&self) -> String {
        self.core.chosen().unwrap().name.clone()
    }

    /// Down the menu to the row named, from where the cursor is.
    fn to(&mut self, name: &str) {
        for _ in 0..20 {
            if self.chosen() == name {
                return;
            }
            self.press(Key::Down);
        }
        panic!("no row {name} in {:?}", self.names());
    }

    /// The menu opened at its root, then down the path named, entering each.
    fn open(&mut self, path: &[&str]) {
        self.core.st.menu.truncate(1);
        self.core.st.menu[0].sel = 0;
        if *self.screen() != Screen::Menu {
            self.press(Key::Menu);
        }
        for name in path {
            self.to(name);
            self.press(Key::Right);
        }
    }
}

use Key::{Ae, Awb, Cross, Down, Flip, Left, Menu, Mirror, Photo, Picture, Play, Record, Right, Up, ZoomIn, ZoomOut};

#[test]
fn the_start_asks_for_the_scene_and_shows_the_mode_for_two_seconds() {
    let mut r = Rig::new();
    assert_eq!(r.cam.log, [Request::Hello { take: false }], "the outputs' facts come in the snapshot");
    assert_eq!(r.core.st.link, Link::Owner);
    assert_eq!(*r.screen(), Screen::Live);
    assert_eq!(r.core.flash_text().as_deref(), Some("2160p30 · ×1,0 · SD 13,0 ГБ"));
    assert_eq!(r.core.st.own, Own { lang: Lang::Ru, cross: true }, "the menu's own from its record");
    r.wait(1999);
    assert!(r.core.flash_text().is_some());
    r.wait(1);
    assert_eq!(r.core.flash_text(), None);
    assert!(r.asked().is_empty(), "nothing written at the start: an unknown record stays as it is");
}

#[test]
fn without_a_record_the_menu_is_in_english_with_the_crosshair_on() {
    let mut cam = Fake::new();
    cam.record = None;
    let r = Rig::with(cam, false);
    assert_eq!(r.core.st.own, Own { lang: Lang::En, cross: true });
}

#[test]
fn a_menu_held_elsewhere_is_waited_for_unless_taken() {
    let mut cam = Fake::new();
    cam.other = true;
    let mut r = Rig::with(cam, false);
    assert_eq!(r.core.st.link, Link::Waiting(2000));
    r.wait(999);
    assert_eq!(r.cam.log.len(), 1);
    r.wait(1);
    assert_eq!(r.cam.log.len(), 2, "asked again after a second");
    assert!(matches!(r.core.st.link, Link::Waiting(_)));
    r.cam.other = false;
    r.wait(1000);
    assert_eq!(r.core.st.link, Link::Owner);
    let mut cam = Fake::new();
    cam.other = true;
    let r = Rig::with(cam, true);
    assert_eq!((r.cam.log[0].clone(), r.core.st.link), (Request::Hello { take: true }, Link::Owner), "started by hand it takes the scene");
}

#[test]
fn menu_shows_the_menu_where_it_was_left_and_hides_it() {
    let mut r = Rig::new();
    r.press(Menu);
    assert_eq!((r.screen(), r.title(), r.chosen()), (&Screen::Menu, "Меню".into(), "Картинка".into()));
    assert_eq!(r.names(), ["Картинка", "Экспозиция", "Наложения", "Запись", "Галерея", "Установка", "Система"]);
    assert_eq!(r.help(), "Яркость, контраст, насыщенность и резкость изображения.");
    r.keys(&[Down, Down]);
    assert_eq!(r.chosen(), "Наложения");
    r.press(Menu);
    assert_eq!(*r.screen(), Screen::Live);
    r.press(Menu);
    assert_eq!((r.screen(), r.chosen()), (&Screen::Menu, "Наложения".into()));
    r.keys(&[Right, Down]);
    assert_eq!((r.title(), r.chosen()), ("Наложения".into(), "Добавить линию".into()));
    r.keys(&[Menu, Menu]);
    assert_eq!((r.title(), r.chosen()), ("Наложения".into(), "Добавить линию".into()), "a section too is kept");
    r.press(Left);
    assert_eq!(r.title(), "Меню");
    r.press(Left);
    assert_eq!(*r.screen(), Screen::Live, "◀ at the root closes");
}

#[test]
fn the_arrows_wrap_and_a_held_one_repeats() {
    let mut r = Rig::new();
    r.press(Menu);
    r.press(Up);
    assert_eq!(r.chosen(), "Система", "▲ from the first row goes to the last");
    r.press(Down);
    assert_eq!(r.chosen(), "Картинка");
    r.hold(Down, 3);
    assert_eq!(r.chosen(), "Галерея", "a press and three repeats: four rows");
    r.hold(Right, 2);
    assert_eq!(r.screen(), &Screen::Gallery(r.gallery().clone()), "▶ held enters once: the menu's ▶ does not repeat");
}

#[test]
fn a_number_is_edited_on_the_slider_and_the_value_is_microcamds() {
    let mut r = Rig::new();
    r.open(&["Картинка"]);
    assert_eq!(r.names(), ["Яркость", "Контраст", "Насыщенность", "Резкость", "Сбросить раздел"]);
    assert_eq!(r.value("Яркость"), "50");
    r.press(Right);
    assert_eq!(*r.screen(), Screen::Slider { item: Item::Brightness, round: Item::Brightness });
    r.keys(&[Right, Right, Right, Left]);
    assert_eq!(r.int("brightness"), 52);
    assert_eq!(r.core.number(Item::Brightness).as_deref(), Some("52"), "the slider shows what microcamd confirmed");
    assert_eq!(r.asked().iter().filter(|q| matches!(q, Request::Adjust { name, .. } if name == "brightness")).count(), 4, "steps, not values: a USB host's change is never written over");
    r.hold(Right, 4);
    assert_eq!(r.int("brightness"), 57, "a held ▶ is a press and the kernel's repeats");
    r.hold(Left, 70);
    assert_eq!(r.int("brightness"), 0, "clamped by microcamd");
    r.press(Down);
    assert_eq!(*r.screen(), Screen::Slider { item: Item::Contrast, round: Item::Brightness }, "▼ the section's next number");
    r.keys(&[Up, Up]);
    assert_eq!(*r.screen(), Screen::Slider { item: Item::Sharpness, round: Item::Brightness }, "▲ wraps");
    r.press(Menu);
    assert_eq!(*r.screen(), Screen::Live, "MENU: done");
    r.press(Menu);
    assert_eq!((r.title(), r.chosen()), ("Картинка".into(), "Яркость".into()));
    assert_eq!(r.value("Яркость"), "0");
}

#[test]
fn resetting_the_picture_puts_the_four_back_and_says_so_once_accepted() {
    let mut r = Rig::new();
    r.open(&["Картинка", "Контраст"]);
    r.keys(&[Right, Right, Menu, Menu]);
    r.open(&["Картинка", "Сбросить раздел"]);
    assert_eq!(r.asked().last(), Some(&Request::Set { name: "contrast".into(), value: Value::Int(50) }), "only what differs");
    assert_eq!(r.int("contrast"), 50);
    assert_eq!(r.toast().as_deref(), Some("Картинка: заводские значения"));
    assert_eq!(*r.screen(), Screen::Menu, "an action leaves the menu where it is");
}

#[test]
fn the_picture_key_goes_round_the_four_numbers_from_contrast() {
    let mut r = Rig::new();
    let slider = |item| Screen::Slider { item, round: Item::Contrast };
    r.press(Picture);
    assert_eq!(*r.screen(), slider(Item::Contrast));
    r.press(Picture);
    assert_eq!(*r.screen(), slider(Item::Saturation));
    r.press(Picture);
    assert_eq!(*r.screen(), slider(Item::Sharpness));
    r.press(Picture);
    assert_eq!(*r.screen(), slider(Item::Brightness));
    r.press(Picture);
    assert_eq!(*r.screen(), Screen::Live, "one more closes it");
    r.open(&["Картинка", "Яркость"]);
    r.keys(&[Picture, Picture, Picture]);
    assert_eq!(*r.screen(), Screen::Slider { item: Item::Sharpness, round: Item::Brightness }, "a round begun from the menu's brightness");
    r.press(Picture);
    assert_eq!(*r.screen(), Screen::Live);
    r.open(&["Наложения", "Добавить линию", "Горизонтальная"]);
    r.press(Picture);
    assert_eq!(*r.screen(), slider(Item::Contrast), "a line being put is dropped");
    assert!(r.core.st.lines.is_empty());
}

#[test]
fn exposure_rows_follow_what_microcamd_allows() {
    let mut r = Rig::new();
    r.open(&["Экспозиция"]);
    assert_eq!(r.names(), ["Автоэкспозиция", "Коррекция экспозиции", "Фиксация экспозиции", "Баланс белого"]);
    assert_eq!([r.value("Автоэкспозиция"), r.value("Коррекция экспозиции"), r.value("Фиксация экспозиции"), r.value("Баланс белого")], ["вкл", "0,0", "выкл", "авто"]);
    r.keys(&[Down, Right, Right, Right, Menu, Menu]);
    assert_eq!(r.value("Коррекция экспозиции"), "+0,6", "two steps of 0.3");
    r.press(Up);
    r.press(Right);
    assert!(!r.on("auto-exposure"), "a row of two values toggles in place");
    assert_eq!(r.value("Автоэкспозиция"), "выкл");
    assert!(r.row("Коррекция экспозиции").dim && r.row("Фиксация экспозиции").dim, "unavailable with the auto off");
    r.press(Down);
    assert_eq!(r.chosen(), "Коррекция экспозиции", "a dimmed row is passed over by nothing: it can be chosen…");
    r.press(Right);
    assert_eq!(*r.screen(), Screen::Menu, "…but not entered");
    r.press(Down);
    r.press(Right);
    assert!(!r.on("exposure-lock"));
    r.keys(&[Up, Up, Right]);
    assert!(r.on("auto-exposure"));
    r.keys(&[Down, Down, Down, Right]);
    assert_eq!(r.value("Баланс белого"), "фиксация");
    assert!(r.on("white-balance-lock"));
}

#[test]
fn the_exposure_slider_alone_in_its_section_has_no_neighbours() {
    let mut r = Rig::new();
    r.open(&["Экспозиция", "Коррекция экспозиции"]);
    r.keys(&[Left, Down, Up]);
    assert_eq!(*r.screen(), Screen::Slider { item: Item::ExposureBias, round: Item::ExposureBias });
    assert_eq!(r.core.number(Item::ExposureBias).as_deref(), Some("−0,3"));
    r.press(Picture);
    assert_eq!(*r.screen(), Screen::Slider { item: Item::Contrast, round: Item::Contrast }, "◐ from another number starts its round");
}

#[test]
fn ae_and_awb_lock_and_unlock_with_a_chip_while_locked() {
    let mut r = Rig::new();
    r.press(Ae);
    assert!(r.on("exposure-lock"));
    r.press(Ae);
    assert!(!r.on("exposure-lock"));
    assert_eq!(r.core.flash_text().as_deref(), Some("Экспозиция: авто"));
    r.press(Awb);
    assert!(r.on("white-balance-lock"));
    r.press(Awb);
    assert_eq!(r.core.flash_text().as_deref(), Some("Баланс белого: авто"));
    r.open(&["Экспозиция"]);
    r.press(Right);
    r.press(Menu);
    r.press(Ae);
    assert_eq!(r.toast().as_deref(), Some("Фиксация экспозиции: автоэкспозиция выключена"), "refused by microcamd, worded by the menu");
}

#[test]
fn the_zoom_keys_step_by_a_tenth_and_hold_repeats() {
    let mut r = Rig::new();
    r.press(ZoomIn);
    assert_eq!(r.core.flash_text().as_deref(), Some("×1,1"));
    r.hold(ZoomIn, 4);
    assert_eq!(r.core.flash_text().as_deref(), Some("×1,6"));
    r.hold(ZoomIn, 60);
    assert_eq!(r.int("zoom"), 40);
    assert_eq!(r.core.flash_text().as_deref(), Some("×5,0"));
    r.wait(2000);
    assert_eq!(r.core.flash_text(), None, "the factor for 2 s after the last press");
    r.open(&["Картинка"]);
    r.press(ZoomOut);
    assert_eq!(r.int("zoom"), 39, "the keys of their own work over the menu");
    assert_eq!(*r.screen(), Screen::Menu);
}

#[test]
fn the_mirror_keys_and_the_mirror_list_are_the_same_two_controls() {
    let mut r = Rig::new();
    r.press(Mirror);
    assert_eq!(r.core.flash_text().as_deref(), Some("Зеркало: по горизонтали"));
    r.press(Flip);
    assert_eq!(r.core.flash_text().as_deref(), Some("Зеркало: по обеим осям"));
    r.open(&["Установка"]);
    assert_eq!(r.names(), ["Зеркало", "Частота сети", "Формат экрана", "Режим HDMI", "Приоритет"]);
    assert_eq!(r.value("Зеркало"), "по обеим осям ›");
    r.press(Right);
    assert_eq!((r.title(), r.chosen()), ("Зеркало".into(), "по обеим осям".into()), "the list opens at the current value");
    assert!(r.row("по обеим осям").mark);
    r.keys(&[Down, Right]);
    assert_eq!((r.on("mirror"), r.on("flip")), (false, false), "«нет»: both back");
    assert_eq!((r.title(), r.value("Зеркало")), ("Установка".into(), "нет ›".into()), "a choice goes back to the section");
    r.keys(&[Right, Down, Down, Left]);
    assert_eq!(r.title(), "Установка", "◀ leaves the list unchanged");
    assert!(!r.on("flip"));
}

#[test]
fn the_crosshair_is_the_menus_own_and_kept_in_its_record() {
    let mut r = Rig::new();
    r.press(Cross);
    assert!(!r.core.st.own.cross);
    assert_eq!(r.cam.record, Some(serde_json::json!({"v": 1, "lang": "ru", "cross": false})));
    r.open(&["Наложения"]);
    assert_eq!(r.value("Перекрестие"), "выкл");
    r.press(Right);
    assert_eq!(r.value("Перекрестие"), "вкл");
    r.feed(Input::Disconnected);
    r.feed(Input::Connected);
    assert!(r.core.st.own.cross, "back from the record after a break");
}

#[test]
fn a_line_is_put_moved_and_cancelled() {
    let mut r = Rig::new();
    r.open(&["Наложения", "Добавить линию"]);
    assert_eq!(r.names(), ["Горизонтальная", "Вертикальная"]);
    r.press(Right);
    assert_eq!(*r.screen(), Screen::Place(Line { dir: Dir::H, pos: 500 }));
    r.press(Up);
    r.hold(Down, 3);
    assert_eq!(*r.screen(), Screen::Place(Line { dir: Dir::H, pos: 515 }), "▲ up; ▼ held moves on");
    r.press(Right);
    assert_eq!((r.screen(), r.core.st.lines.clone()), (&Screen::Live, vec![Line { dir: Dir::H, pos: 515 }]));
    r.press(Menu);
    assert_eq!((r.title(), r.chosen()), ("Добавить линию".into(), "Горизонтальная".into()), "the next line is two presses");
    r.keys(&[Down, Right, Up, Up]);
    assert_eq!(*r.screen(), Screen::Place(Line { dir: Dir::V, pos: 510 }), "a vertical one: ▲ right");
    r.press(Left);
    assert_eq!((r.screen(), r.core.st.lines.len()), (&Screen::Menu, 1), "◀ cancels, back to the menu");
    r.keys(&[Right, Menu]);
    assert_eq!((r.screen(), r.core.st.lines.len()), (&Screen::Live, 1), "MENU drops it, to the camera");
    r.hold(Right, 1);
    r.open(&["Наложения", "Добавить линию", "Горизонтальная"]);
    r.hold(Up, 120);
    assert_eq!(*r.screen(), Screen::Place(Line { dir: Dir::H, pos: 10 }), "stops at the edge");
    r.press(Right);
    r.open(&["Наложения", "Убрать все линии"]);
    assert!(r.core.st.lines.is_empty());
    assert_eq!(r.toast().as_deref(), Some("Линии убраны"));
}

#[test]
fn display_levels_are_a_list_of_four_with_words_of_their_own() {
    let mut r = Rig::new();
    r.open(&["Наложения"]);
    assert_eq!(r.names(), ["Перекрестие", "Добавить линию", "Убрать все линии", "Показ на HDMI", "Показ на USB", "В файлах"]);
    let lvl = r.cam.list.get("level-media").unwrap().value.clone();
    assert_eq!(r.value("В файлах"), format!("{} ›", Lang::Ru.words().levels[match lvl { Value::Int(i) => i as usize, _ => 0 }].name));
    r.to("Показ на HDMI");
    r.press(Right);
    assert_eq!(r.names(), ["только картинка", "с линиями", "с индикаторами", "всё"]);
    r.to("с линиями");
    assert_eq!(r.help(), "Перекрестие и линии.", "a value with its own description");
    r.press(Right);
    assert_eq!(r.int("level-hdmi"), 1);
    assert_eq!(r.value("Показ на HDMI"), "с линиями ›");
}

#[test]
fn the_setup_lists_are_microcamds_values() {
    let mut r = Rig::new();
    r.open(&["Запись", "Разрешение видео"]);
    assert_eq!(r.names(), ["4K", "1440p", "1080p"], "microcamd offers three sizes, the mockup two");
    r.to("1080p");
    r.press(Right);
    assert_eq!(r.value("Разрешение видео"), "1080p ›");
    r.open(&["Установка", "Частота сети"]);
    assert_eq!(r.names(), ["выкл", "50 Гц", "60 Гц"]);
    r.press(Left);
    r.open(&["Установка", "Формат экрана"]);
    assert_eq!(r.names(), ["16:9", "16:10", "4:3"]);
    r.keys(&[Down, Right]);
    assert_eq!(r.value("Формат экрана"), "16:10 ›");
    r.to("Режим HDMI");
    r.press(Right);
    assert_eq!(r.names(), ["авто", "2160p30", "1080p60"], "the monitor's timings beside «авто»; «выкл» not offered");
    r.to("1080p60");
    r.press(Right);
    assert_eq!(r.value("Режим HDMI"), "1080p60 ›");
    r.to("Приоритет");
    assert_eq!(r.value("Приоритет"), "USB", "USB above HDMI in the default precedence");
    r.press(Right);
    assert_eq!(r.value("Приоритет"), "HDMI", "two values toggle");
    // the swap of the two in the precedence, the rest and the floors as they were
    let q = r.cam.list.get("precedence").unwrap().value.clone();
    assert_eq!(q, Value::Text("recording > player > hdmi > usb > photo > drawing > picture".into()));
}

#[test]
fn the_language_switches_every_word_and_is_kept() {
    let mut r = Rig::new();
    r.open(&["Система"]);
    assert_eq!(r.names(), ["Язык", "SD-карта", "Сброс настроек", "О камере"]);
    assert_eq!(r.value("Язык"), "Русский ›");
    r.press(Right);
    assert_eq!(r.names(), ["Русский", "English", "Polski", "Deutsch", "Français", "Español", "Nederlands"], "each language by its own name");
    r.keys(&[Down, Right]);
    assert_eq!((r.title(), r.value("Language")), ("System".into(), "English ›".into()));
    assert_eq!(r.names(), ["Language", "SD card", "Reset settings", "About the camera"]);
    assert_eq!(r.help(), "The menu's language.");
    r.press(Left);
    assert_eq!(r.names(), ["Picture", "Exposure", "Overlays", "Recording", "Gallery", "Setup", "System"]);
    assert_eq!(r.cam.record.as_ref().unwrap()["lang"], "en");
    r.press(Photo);
    assert_eq!(r.toast().as_deref(), Some("Photo saved · P00041.jpg"));
}

#[test]
fn every_row_has_its_words_in_both_languages() {
    let mut r = Rig::new();
    for lang in Lang::ALL {
        r.core.st.own.lang = lang;
        let levels = [Level::Root, Level::AddLine, Level::Card, Level::FormatAsk, Level::ResetAsk, Level::About]
            .into_iter()
            .chain([Section::Picture, Section::Exposure, Section::Overlays, Section::Recording, Section::Setup, Section::System].map(Level::Section));
        for l in levels {
            let p = r.core.page(l);
            assert!(!p.title.is_empty());
            for row in p.rows {
                assert!(!row.name.is_empty(), "{l:?}");
                assert!(row.act.is_none() || row.help.as_deref().is_some_and(|h| !h.is_empty()), "{l:?} {} has no description", row.name);
            }
        }
    }
}

#[test]
fn the_card_is_formatted_after_a_question() {
    let mut r = Rig::new();
    r.open(&["Система", "SD-карта"]);
    assert_eq!(r.names(), ["Свободно", "Режим картридера", "Форматировать"]);
    assert_eq!(r.value("Свободно"), "13,0 из 31,1 ГБ");
    assert_eq!(r.chosen(), "Режим картридера", "the state row is not chosen");
    r.press(Down);
    r.press(Right);
    assert_eq!((r.title(), r.chosen()), ("Стереть всё с SD-карты?".into(), "Отмена".into()));
    r.press(Right);
    assert_eq!(r.title(), "SD-карта", "Отмена: nothing");
    r.asked();
    r.keys(&[Right, Down, Right]);
    assert_eq!(r.asked(), [Request::Format]);
    assert_eq!(r.toast().as_deref(), Some("SD-карта отформатирована"));
    assert_eq!(r.core.st.cam.files.total, 0);
    assert_eq!(r.value("Свободно"), "31,0 из 31,1 ГБ", "the space told after the format");
    r.cam.card = Card::None;
    r.event(Event::Card { card: CardInfo { at: Card::None, ..CardInfo::default() } });
    assert_eq!(r.names(), ["Карты нет", "Режим картридера"], "the mode is kept without a card too");
}

#[test]
fn the_card_reader_mode_is_switched_in_the_card_page() {
    let mut r = Rig::new();
    r.open(&["Система", "SD-карта"]);
    assert_eq!(r.value("Режим картридера"), "выкл");
    r.asked();
    r.press(Right);
    let set = |on| Request::Set { name: "card-reader".into(), value: Value::Bool(on) };
    assert_eq!(r.asked(), [set(true)]);
    assert_eq!(r.names(), ["Карта", "Режим картридера", "Форматировать"], "the space is the computer's now; formatting takes the card as a photo does");
    assert_eq!((r.value("Карта"), r.chosen(), r.value("Режим картридера")), ("у компьютера".into(), "Режим картридера".into(), "вкл".into()));
    r.cam.open = true;
    r.press(Right);
    assert_eq!(r.asked(), [set(false)]);
    assert_eq!((r.toast().as_deref(), r.value("Режим картридера")), (Some("Режим картридера не выключен: карта открыта на компьютере, извлеките её там"), "вкл".into()));
    r.cam.open = false;
    r.press(Right);
    assert_eq!(r.value("Режим картридера"), "выкл");
    assert_eq!(r.cam.card, Card::Camera);
}

#[test]
fn a_photo_takes_the_card_back_when_the_computer_has_it_closed() {
    let mut r = Rig::new();
    r.open(&["Система", "SD-карта", "Режим картридера"]);
    assert_eq!((r.core.st.cam.card.at, r.value("Режим картридера")), (Card::Host, "вкл".into()));
    r.press(Menu);
    r.asked();
    r.press(Photo);
    assert_eq!(r.asked(), [Request::Set { name: "card-reader".into(), value: Value::Bool(false) }, Request::Photo]);
    assert_eq!((r.core.st.cam.card.at, r.toast().as_deref()), (Card::Camera, Some("Снимок сохранён · P00041.jpg")));
    r.open(&["Система", "SD-карта"]);
    assert_eq!(r.value("Режим картридера"), "выкл", "the mode off with the card back");
}

#[test]
fn the_gallery_takes_the_card_back_unless_the_computer_has_it_open() {
    let mut r = Rig::new();
    r.open(&["Система", "SD-карта", "Режим картридера"]);
    r.press(Menu);
    r.cam.open = true;
    r.asked();
    r.press(Key::Gallery);
    assert_eq!(r.asked(), [Request::Set { name: "card-reader".into(), value: Value::Bool(false) }]);
    assert_eq!((r.screen(), r.toast().as_deref(), r.cam.card), (&Screen::Live, Some("Галерея недоступна: карта открыта на компьютере, извлеките её там"), Card::Host));
    r.wait(5999);
    assert!(r.toast().is_some(), "a toast to act on stays long");
    r.wait(1);
    assert_eq!(r.toast(), None);
    r.cam.open = false;
    r.press(Key::Gallery);
    assert_eq!((r.gallery().from, r.core.st.cam.card.at, r.chosen_file()), (Origin::Live, Card::Camera, "P00040.jpg".into()));
    r.press(Key::Gallery);
    r.open(&["Система", "SD-карта"]);
    assert_eq!(r.value("Режим картридера"), "выкл", "the mode off with the card back");
}

#[test]
fn a_reset_is_a_new_session_from_the_defaults() {
    let mut r = Rig::new();
    r.press(Cross);
    r.press(ZoomIn);
    r.press(Right);
    r.open(&["Наложения", "Добавить линию", "Горизонтальная"]);
    r.press(Right);
    r.open(&["Система", "Сброс настроек"]);
    assert_eq!((r.title(), r.chosen()), ("Вернуть заводские настройки?".into(), "Отмена".into()));
    r.asked();
    r.keys(&[Down, Right]);
    assert_eq!(r.asked(), [Request::Reset, Request::Hello { take: false }], "microcamd takes the scene with a reset: asked for again at once");
    assert_eq!(r.core.st.link, Link::Owner);
    assert_eq!(r.toast().as_deref(), Some("Settings reset"), "the record is gone with the settings: the language is the default");
    assert_eq!((r.int("zoom"), r.core.st.own), (0, Own::default()));
    assert_eq!(r.core.st.lines.len(), 1, "the lines are not settings (the mockup keeps them)");
    assert_eq!(r.title(), "System");
}

#[test]
fn about_asks_microcamd_what_runs() {
    let mut r = Rig::new();
    r.open(&["Система", "О камере"]);
    assert_eq!(r.asked(), [Request::Status]);
    let p = r.page();
    let rows: Vec<(String, String)> = p.rows.iter().map(|r| (r.name.clone(), r.value.clone())).collect();
    assert_eq!(rows, [("OpenMicroCam".into(), "0.92 · 524f825".into()), ("HDMI".into(), "2160p30".into()), ("USB".into(), "3840×2160 · 45 к/с".into())]);
    assert!(p.rows.iter().all(|r| r.dim && r.act.is_none()));
}

#[test]
fn a_photo_says_its_file_and_no_card_says_so() {
    let mut r = Rig::new();
    r.press(Photo);
    assert_eq!(r.toast().as_deref(), Some("Снимок сохранён · P00041.jpg"));
    r.wait(2199);
    assert!(r.toast().is_some());
    r.wait(1);
    assert_eq!(r.toast(), None);
    r.cam.card = Card::None;
    r.press(Photo);
    assert_eq!(r.toast().as_deref(), Some("Нет SD-карты"));
    (r.cam.card, r.cam.open) = (Card::Host, true);
    let card = r.cam.card_info();
    r.event(Event::Card { card });
    r.press(Record);
    assert_eq!(r.toast().as_deref(), Some("Запись невозможна: карта открыта на компьютере, извлеките её там"));
    r.press(Photo);
    assert_eq!(r.toast().as_deref(), Some("Снимок невозможен: карта открыта на компьютере, извлеките её там"));
    assert_eq!(r.cam.card, Card::Host, "not taken");
    (r.cam.card, r.cam.open) = (Card::Camera, false);
    r.asked();
    r.hold(Photo, 3);
    assert_eq!(r.asked().iter().filter(|q| **q == Request::Photo).count(), 1, "the photo key does not repeat");
}

#[test]
fn a_recording_runs_its_clock_between_events() {
    let mut r = Rig::new();
    r.press(Record);
    assert_eq!(r.core.st.cam.running.record.as_ref().map(|x| x.state), Some(RecordState::Running));
    r.wait(12_300);
    assert_eq!(r.core.record_now() / 1000, 12);
    assert_eq!(r.wake, Some(r.now + 700), "woken at the clock's next second");
    r.press(Key::Gallery);
    assert_eq!(r.toast().as_deref(), Some("Галерея недоступна: идёт запись"), "no gallery while recording");
    r.press(Record);
    assert_eq!(r.toast().as_deref(), Some("Запись сохранена · V00041.mp4 · 00:12"));
    assert_eq!(r.core.st.cam.running.record, None);
}

#[test]
fn the_gallery_steps_through_the_cards_list() {
    let mut r = Rig::new();
    r.press(Key::Gallery);
    assert_eq!(r.asked(), [Request::Files { from: 0, count: WINDOW }]);
    assert_eq!((r.gallery().view.clone(), r.gallery().from, r.chosen_file()), (GalleryView::Strip, Origin::Live, "P00040.jpg".into()));
    r.press(Down);
    assert_eq!(r.chosen_file(), "P00039.jpg");
    r.keys(&[Up, Up]);
    assert_eq!(r.gallery().at, 39, "▲ from the newest wraps to the oldest");
    assert_eq!(r.chosen_file(), "P00001.jpg", "its window read");
    r.press(Down);
    r.press(Right);
    assert_eq!(r.gallery().view, GalleryView::Alone, "a photo alone on the screen");
    r.press(Left);
    assert_eq!(r.gallery().view, GalleryView::Strip);
    r.keys(&[Right, Menu]);
    assert_eq!(r.gallery().view, GalleryView::Strip, "MENU from a file: to the strip");
    r.keys(&[Photo, ZoomIn, Key::Freeze, Cross, Picture]);
    assert!(r.asked().iter().all(|q| matches!(q, Request::Files { .. })), "the other keys do nothing in the gallery");
    assert_eq!(r.core.st.freeze, Freeze::Off);
    r.press(Left);
    assert_eq!(*r.screen(), Screen::Live, "opened from the camera: ◀ closes");
    r.open(&["Галерея"]);
    assert_eq!(r.gallery().from, Origin::Menu);
    r.press(Left);
    assert_eq!((r.screen(), r.chosen()), (&Screen::Menu, "Галерея".into()), "opened from the menu: ◀ back to it");
    r.keys(&[Right, Right]);
    r.press(Key::Gallery);
    assert_eq!(*r.screen(), Screen::Live, "🖼 leaves for the camera from any depth");
    r.press(Key::Gallery);
    r.press(Menu);
    assert_eq!(*r.screen(), Screen::Live, "MENU on the strip closes it");
}

#[test]
fn a_video_plays_winds_by_five_seconds_and_ends_on_the_strip() {
    let mut r = Rig::new();
    r.press(Key::Gallery);
    r.keys(&[Down, Down]);
    assert_eq!(r.chosen_file(), "V00038.mp4");
    r.asked();
    r.press(Right);
    assert_eq!(r.asked(), [Request::Play { file: "/mnt/VIDEO/V00038.mp4".into() }]);
    assert_eq!(r.gallery().view, GalleryView::Video { file: "/mnt/VIDEO/V00038.mp4".into(), seen: true, asked: true });
    r.wait(3000);
    r.press(Right);
    assert_eq!(r.asked(), [Request::PlaySeek { ms: 8000 }], "from where it plays now");
    r.hold(Right, 3);
    assert!(r.asked().iter().all(|q| matches!(q, Request::PlaySeek { .. })));
    let at = r.core.player_ms().unwrap();
    r.press(Left);
    assert_eq!(r.asked(), [Request::PlaySeek { ms: at - 5000 }]);
    r.press(Play);
    assert_eq!(r.cam.player.as_ref().unwrap().state, PlayerState::Paused);
    r.press(Play);
    assert_eq!(r.cam.player.as_ref().unwrap().state, PlayerState::Playing);
    r.asked();
    let p = r.cam.player.as_mut().unwrap();
    (p.state, p.ms) = (PlayerState::Ended, p.duration_ms);
    let ended = r.cam.player.clone();
    r.event(Event::Player { player: ended });
    assert_eq!((r.gallery().view.clone(), r.asked()), (GalleryView::Strip, vec![Request::PlayStop]), "a clip at its end returns to the strip");
    r.keys(&[Play, Down]);
    assert_eq!(r.asked(), [Request::Play { file: "/mnt/VIDEO/V00038.mp4".into() }, Request::PlayStop], "▶‖ plays from the strip; ▼ stops it…");
    assert_eq!((r.gallery().view.clone(), r.chosen_file()), (GalleryView::Alone, "P00037.jpg".into()), "…and the next file is alone");
    r.keys(&[Play, Up, Right, Menu]);
    assert_eq!((r.gallery().view.clone(), r.asked().last().cloned()), (GalleryView::Strip, Some(Request::PlayStop)), "MENU: to the strip");
    r.keys(&[Right, Key::Gallery]);
    let last = r.asked().last().cloned();
    assert_eq!((r.screen(), last), (&Screen::Live, Some(Request::PlayStop)));
}

#[test]
fn the_gallery_refuses_without_a_card_or_files() {
    let mut r = Rig::new();
    r.cam.files.clear();
    r.cam.version += 1;
    r.event(Event::Files { version: r.cam.version, total: 0 });
    r.press(Key::Gallery);
    assert_eq!((r.screen(), r.toast().as_deref()), (&Screen::Live, Some("На карте нет снимков и видео")));
    r.event(Event::Card { card: CardInfo { at: Card::None, ..CardInfo::default() } });
    r.press(Key::Gallery);
    assert_eq!(r.toast().as_deref(), Some("Нет SD-карты"));
}

#[test]
fn the_choice_is_kept_by_file_when_the_list_changes() {
    let mut r = Rig::new();
    r.press(Key::Gallery);
    r.keys(&[Down, Down, Down]);
    assert_eq!(r.chosen_file(), "P00037.jpg");
    let f = r.cam.new_file(FileKind::Photo);
    let ev = r.cam.added(f);
    r.feed(ev);
    assert_eq!((r.gallery().at, r.chosen_file()), (4, "P00037.jpg".into()));
    r.event(Event::Card { card: CardInfo { at: Card::Host, ..CardInfo::default() } });
    assert_eq!((r.screen(), r.toast().as_deref()), (&Screen::Live, Some("Включён режим картридера")));
}

#[test]
fn an_old_window_is_read_again_and_a_late_one_dropped() {
    let mut r = Rig::new();
    r.hold = true;
    r.press(Key::Gallery);
    r.hold = false;
    let list = r.cam.files[..WINDOW].to_vec();
    let w = move |seq, version| Response { seq: Some(seq), files: Some(FilesWindow { from: 0, total: 40, version, list: list.clone() }), ..Response::ok() };
    let stale = w(r.core.seq, 0);
    r.feed(Input::Reply(stale));
    assert_eq!(r.asked().last(), Some(&Request::Files { from: 0, count: WINDOW }), "a window older than the list is read again");
    assert_eq!(r.chosen_file(), "P00040.jpg");
    r.hold = true;
    r.press(Up);
    r.press(Key::Gallery);
    r.hold = false;
    let late = w(r.core.seq, 1);
    r.feed(Input::Reply(late));
    assert_eq!(*r.screen(), Screen::Live, "an answer after the gallery is left is dropped");
}

#[test]
fn freeze_goes_round_three_presses() {
    let mut r = Rig::new();
    r.press(Key::Freeze);
    assert_eq!(r.core.st.freeze, Freeze::Frozen(1));
    r.press(Key::Freeze);
    assert_eq!(r.core.st.freeze, Freeze::Compare(1));
    r.press(Key::Freeze);
    assert_eq!(r.core.st.freeze, Freeze::Off);
    r.press(Key::Freeze);
    assert_eq!(r.core.st.freeze, Freeze::Frozen(2), "a new picture each time");
    r.event(Event::Resource { id: frozen_id(2), state: ResState::Failed { code: ResCode::Busy, error: "media port".into(), holders: vec![] } });
    assert_eq!((r.core.st.freeze, r.toast().as_deref()), (Freeze::Off, Some("Стоп-кадр не получился")));
}

#[test]
fn after_a_break_nothing_is_asked_again() {
    let mut r = Rig::new();
    r.hold = true;
    r.press(Photo);
    r.press(Key::Gallery);
    r.feed(Input::Disconnected);
    r.hold = false;
    assert_eq!(r.core.st.link, Link::Offline);
    assert_eq!(*r.screen(), Screen::Live);
    r.asked();
    r.feed(Input::Connected);
    assert_eq!(r.asked(), [Request::Hello { take: false }], "the photo is not sent again, nor the status");
    assert_eq!(r.core.st.link, Link::Owner);
}

#[test]
fn a_scene_taken_by_another_menu_is_waited_for() {
    let mut r = Rig::new();
    r.press(Key::Gallery);
    r.cam.other = true;
    r.event(Event::Lost);
    assert_eq!((r.core.st.link, r.screen()), (Link::Waiting(r.now + 1000), &Screen::Live));
    r.asked();
    r.wait(1000);
    assert_eq!(r.asked(), [Request::Hello { take: false }]);
    r.cam.other = false;
    r.wait(1000);
    assert_eq!(r.core.st.link, Link::Owner);
}

#[test]
fn repeats_go_only_where_holding_means_more() {
    let mut r = Rig::new();
    r.hold(Play, 3);
    r.hold(Cross, 3);
    assert!(!r.core.st.own.cross, "pressed once, not four times");
    r.press(Menu);
    r.hold(Left, 3);
    assert_eq!(*r.screen(), Screen::Live, "◀ held at the root closes once");
}

#[test]
fn a_refused_set_says_what_and_why() {
    let mut r = Rig::new();
    r.event(Event::Running { running: Running { record: Some(RecordRun { file: "/mnt/VIDEO/V00041.mp4".into(), state: RecordState::Running, ms: 0 }), ..Running::default() } });
    r.cam.list.record_runs(true);
    r.open(&["Запись", "Разрешение видео"]);
    r.keys(&[Down, Right]);
    assert_eq!(r.toast().as_deref(), Some("Разрешение видео: идёт запись"));
}

mod drawn {
    use super::*;
    use crate::presenter::{Presenter, SEQ_BASE};
    use crate::view::{Pic, View};
    use microcam_core::edit::Op;
    use microcam_core::protocol::{Output, OutputInfo, Source};
    use microcam_core::scene::Scene;
    use microcam_draw::render::Fonts;

    fn check(r: &Rig, fonts: &Fonts, what: &str) -> usize {
        let pic = |_: &str| Pic::Ready(3840, 2160);
        let d = View { core: &r.core, pic: &pic, fonts, down: 0 }.draw();
        let s = Scene { nodes: d.nodes, ..Scene::empty() };
        s.validate().unwrap_or_else(|e| panic!("{what}: {e:?}"));
        for res in s.resources() {
            assert!(d.pictures.iter().any(|p| p.0 == res), "{what}: {res} shown but not asked for");
        }
        s.nodes.len()
    }

    #[test]
    fn every_screen_draws_a_scene_microcamd_takes() {
        let fonts = Fonts::new();
        let mut r = Rig::new();
        r.press(Ae);
        r.press(Record);
        check(&r, &fonts, "the camera with chips");
        r.press(Record);
        check(&r, &fonts, "a toast");
        for lang in Lang::ALL {
            r.core.st.own.lang = lang;
            let levels = [Level::Root, Level::AddLine, Level::Card, Level::FormatAsk, Level::ResetAsk, Level::About]
                .into_iter()
                .chain([Section::Picture, Section::Exposure, Section::Overlays, Section::Recording, Section::Setup, Section::System].map(Level::Section));
            for l in levels {
                r.core.st.menu = vec![Spot { level: l, sel: 0 }];
                r.core.st.screen = Screen::Menu;
                check(&r, &fonts, &format!("{l:?}"));
            }
        }
        r.core.st.own.lang = Lang::Ru;
        r.core.st.screen = Screen::Live;
        r.core.st.menu = vec![Spot { level: Level::Root, sel: 0 }];
        r.open(&["Картинка", "Яркость"]);
        check(&r, &fonts, "the slider");
        r.press(Menu);
        r.open(&["Наложения", "Добавить линию", "Вертикальная"]);
        check(&r, &fonts, "a line");
        r.press(Right);
        r.press(Key::Freeze);
        check(&r, &fonts, "frozen");
        r.press(Key::Freeze);
        check(&r, &fonts, "compare");
        r.press(Key::Freeze);
        r.press(Key::Gallery);
        check(&r, &fonts, "the strip");
        r.keys(&[Down, Down, Right]);
        r.wait(3000);
        check(&r, &fonts, "a video");
        r.keys(&[Up, Right]);
        check(&r, &fonts, "a photo alone");
    }

    /// The presenter between the core and a microcamd that answers commits and says when each output shows them.
    struct Screen2 {
        r: Rig,
        p: Presenter,
        sent: Vec<Request>,
    }

    impl Screen2 {
        fn new() -> Self {
            let mut r = Rig::new();
            r.core.st.cam.outputs = [Output::Hdmi, Output::Usb].map(|name| OutputInfo { name, w: 3840, h: 2160, level: 3, paused: None, timing: None, codec: None, fps: None }).to_vec();
            let mut s = Screen2 { r, p: Presenter::new(), sent: Vec::new() };
            s.update();
            s
        }

        fn update(&mut self) {
            let (reqs, _) = self.p.update(self.r.now, &self.r.core);
            for e in reqs {
                assert!(e.seq.unwrap() > SEQ_BASE);
                if let Request::Commit { base, .. } = &e.req {
                    self.p.reply(&Response { seq: e.seq, revision: Some(base + 1), ..Response::ok() });
                }
                self.sent.push(e.req);
            }
        }

        fn shown(&mut self, rev: u64) {
            for output in [Output::Hdmi, Output::Usb] {
                self.p.event(self.r.now, &Event::Presented { output, revision: rev, complete: true });
            }
            self.update();
        }

        fn press(&mut self, k: Key) {
            self.r.press(k);
            self.update();
        }

        fn commits(&mut self) -> Vec<u64> {
            let c = self.sent.iter().filter_map(|q| if let Request::Commit { base, ops } = q { assert!(matches!(ops[..], [Op::Replace { .. }])); Some(*base) } else { None }).collect();
            c
        }
    }

    #[test]
    fn the_next_commit_waits_until_the_last_is_shown_and_takes_the_newest_state() {
        let mut s = Screen2::new();
        assert_eq!(s.commits(), [0], "the start's chip");
        s.shown(1);
        s.press(Menu);
        s.press(Down);
        s.press(Down);
        assert_eq!(s.commits(), [0, 1], "the menu opened; the two steps wait");
        s.shown(2);
        assert_eq!(s.commits(), [0, 1, 2], "both steps in one commit");
        s.shown(3);
        assert_eq!(s.commits().len(), 3, "nothing changed: nothing sent");
        s.press(Down);
        s.r.wait(2000);
        s.update();
        s.press(Down);
        assert_eq!(s.commits().len(), 6, "an output stuck past its limit is not waited for longer (the start's chip gone, then the step)");
    }

    #[test]
    fn an_output_that_stopped_presenting_is_not_waited_for_until_it_presents_again() {
        let mut s = Screen2::new();
        s.shown(1);
        let usb = |s: &mut Screen2, rev| {
            s.p.event(s.r.now, &Event::Presented { output: Output::Usb, revision: rev, complete: true });
            s.update();
        };
        s.press(Menu);
        usb(&mut s, 2);
        s.press(Down);
        assert_eq!(s.commits().len(), 2, "HDMI is waited for…");
        s.r.wait(2000);
        s.update();
        assert_eq!(s.commits().len(), 3, "…up to its limit");
        usb(&mut s, 3);
        s.press(Down);
        assert_eq!(s.commits().len(), 4, "then no longer: USB alone paces");
        s.shown(4);
        s.press(Down);
        s.p.event(s.r.now, &Event::Presented { output: Output::Usb, revision: 5, complete: true });
        s.update();
        s.press(Down);
        assert_eq!(s.commits().len(), 5, "HDMI presented again: waited for again");
    }

    #[test]
    fn a_resources_toast_is_one_panel_inside_the_screen() {
        use microcam_core::scene::Kind as NodeKind;
        let fonts = Fonts::new();
        let mut r = Rig::new();
        r.press(Key::Freeze);
        r.event(Event::Running { running: Running::default() });
        // the widest the camera can hold at once: a recording forbids the player and the gallery
        let mut hs = holders(&frozen_id(9));
        hs.insert(0, held(Held::Recording { w: 3840, h: 2160 }, 80));
        r.event(Event::NotShown { output: Output::Usb, revision: 9, code: Code::Resources, reason: "e".into(), holders: hs });
        for lang in Lang::ALL {
            r.core.st.own.lang = lang;
            check(&r, &fonts, "a resources toast");
            let pic = |_: &str| Pic::Ready(3840, 2160);
            let d = View { core: &r.core, pic: &pic, fonts: &fonts, down: 0 }.draw();
            let toast = d.nodes.iter().find(|n| n.id.as_deref() == Some("toast")).unwrap();
            let NodeKind::Group(g) = &toast.kind else { panic!("a group") };
            let rects: Vec<f32> = g.children.iter().filter_map(|n| if let NodeKind::Rect(r) = &n.kind { Some(r.w) } else { None }).collect();
            let texts = g.children.iter().filter(|n| matches!(n.kind, NodeKind::Text(_))).count();
            assert!(rects.len() == 1 && texts >= 2, "{lang:?}: one panel, a line each: {texts}");
            assert!(rects[0] <= 1920.0 - 64.0, "{lang:?}: {} wide", rects[0]);
        }
    }

    #[test]
    fn a_not_shown_met_by_a_step_down_is_the_presenters_and_holders_get_the_cores_ids() {
        let mut s = Screen2::new();
        s.shown(1);
        s.press(Key::Gallery);
        let not_shown = |revision| Event::NotShown { output: Output::Hdmi, revision, code: Code::Resources, reason: "e".into(), holders: vec![] };
        assert!(s.p.event(s.r.now, &not_shown(2)), "the strip's thumbnails into the menu's layer");
        s.update();
        assert!(s.p.event(s.r.now, &not_shown(3)), "a step further");
        s.update();
        assert!(!s.p.event(s.r.now, &not_shown(4)), "no step left: the core says it");
        let mut hs = vec![held(Held::Picture { id: "1:frozen-1".into() }, 11), held(Held::Picture { id: "0:frozen-7".into() }, 1)];
        s.p.core_holders(&mut hs);
        assert_eq!(hs.iter().map(|h| h.what.clone()).collect::<Vec<_>>(), [Held::Picture { id: "frozen-1".into() }, Held::Picture { id: "0:frozen-7".into() }]);
    }

    fn resources(s: &Screen2) -> Vec<(String, Source)> {
        s.sent.iter().filter_map(|q| if let Request::Resource { id, source } = q { Some((id.clone(), source.clone())) } else { None }).collect()
    }

    fn released(s: &Screen2) -> Vec<String> {
        s.sent.iter().filter_map(|q| if let Request::Release { id } = q { Some(id.clone()) } else { None }).collect()
    }

    #[test]
    fn a_freeze_not_yet_shown_steps_down_once_to_a_half_picture_of_its_own() {
        let mut s = Screen2::new();
        s.shown(1);
        s.press(Key::Freeze);
        assert_eq!(resources(&s), [("1:frozen-1".to_string(), Source::Camera { w: 3840, h: 2160 })]);
        let not_shown = |revision| Event::NotShown { output: Output::Hdmi, revision, code: Code::Resources, reason: "e".into(), holders: vec![] };
        s.p.event(s.r.now, &Event::Presented { output: Output::Media, revision: 2, complete: true });
        assert!(s.p.event(s.r.now, &not_shown(2)), "shown nowhere yet (the recording's output, unseen, presents it whole): a step down");
        s.update();
        assert_eq!(resources(&s)[1], ("1:frozen-1-half".to_string(), Source::Camera { w: 1920, h: 1080 }), "asked again under a new id");
        assert_eq!(s.commits(), [0, 1, 2], "the tree names the new picture");
        assert!(!s.p.event(s.r.now, &not_shown(3)), "one step only: the next is the core's to say");
        s.shown(3);
        s.press(Cross);
        assert_eq!(released(&s), ["1:frozen-1"], "the full-size picture goes");
        assert_eq!(s.r.core.st.freeze, Freeze::Frozen(1), "the same freeze");
    }

    #[test]
    fn an_output_that_cannot_show_a_step_is_not_waited_for() {
        let mut s = Screen2::new();
        s.shown(1);
        s.press(Menu);
        for output in [Output::Hdmi, Output::Usb] {
            assert!(!s.p.event(s.r.now, &Event::NotShown { output, revision: 2, code: Code::Resources, reason: "e".into(), holders: vec![] }));
        }
        s.press(Down);
        assert_eq!(s.commits(), [0, 1, 2], "both answered: the next step at once, not after the wait");
    }

    #[test]
    fn a_freeze_shown_is_not_taken_again_smaller() {
        let mut s = Screen2::new();
        s.shown(1);
        s.press(Key::Freeze);
        s.p.event(s.r.now, &Event::Presented { output: Output::Usb, revision: 2, complete: false });
        s.p.event(s.r.now, &Event::Presented { output: Output::Usb, revision: 2, complete: true });
        assert!(!s.p.event(s.r.now, &Event::NotShown { output: Output::Hdmi, revision: 2, code: Code::Resources, reason: "e".into(), holders: vec![] }), "shown whole on USB: kept, said by the core");
        s.update();
        assert_eq!(resources(&s).len(), 1, "no new picture asked");
    }

    #[test]
    fn a_screen_left_releases_its_pictures_and_a_new_session_starts_from_nothing() {
        let mut s = Screen2::new();
        s.shown(1);
        s.press(Key::Gallery);
        let asked: Vec<String> = s.sent.iter().filter_map(|q| if let Request::Resource { id, .. } = q { Some(id.clone()) } else { None }).collect();
        assert!(asked.contains(&"1:/mnt/PHOTO/P00040.jpg".to_string()), "{asked:?}");
        assert!(asked.contains(&"1:/mnt/PHOTO/P00039.jpg".to_string()), "the next made ready ahead");
        s.shown(2);
        s.press(Key::Gallery);
        s.shown(3);
        s.r.wait(2000);
        s.update();
        let released = s.sent.iter().filter(|q| matches!(q, Request::Release { .. })).count();
        assert_eq!(released, asked.len(), "every picture released once the camera is back");
        s.r.feed(Input::Disconnected);
        s.update();
        s.r.feed(Input::Connected);
        s.sent.clear();
        s.update();
        assert_eq!(s.commits(), [0], "a new session: revision 0 again");
    }
}

#[test]
fn a_video_waits_for_the_one_before_to_stop() {
    let mut r = Rig::new();
    r.cam.slow_stop = true;
    r.press(Key::Gallery);
    r.keys(&[Down, Down, Right]);
    assert!(matches!(r.gallery().view, GalleryView::Video { seen: true, .. }));
    r.asked();
    r.keys(&[Down, Up]);
    assert_eq!(r.cam.player.as_ref().map(|p| p.state), Some(PlayerState::Stopping));
    r.press(Right);
    assert_eq!(r.asked(), [Request::PlayStop], "not asked while the player stops: it would be refused busy");
    assert!(matches!(r.gallery().view, GalleryView::Video { asked: false, .. }));
    let ev = r.cam.stopped();
    r.feed(ev);
    assert_eq!(r.asked(), [Request::Play { file: "/mnt/VIDEO/V00038.mp4".into() }]);
    assert!(matches!(r.gallery().view, GalleryView::Video { seen: true, .. }));
    r.event(Event::Failed { what: Action::Play, code: Code::Refused, error: "e".into(), holders: vec![] });
    assert_eq!((r.gallery().view.clone(), r.toast().as_deref()), (GalleryView::Alone, Some("Видео не запустилось: камера не смогла")));
}

// ---- refusals for lack of resources

fn held(what: Held, mib: u64) -> Holder {
    Holder { what, bytes: Some(mib << 20) }
}

/// What holds the camera's resources in the camera's own example: HDMI, the USB stream, the menu's drawing, a picture.
fn holders(picture: &str) -> Vec<Holder> {
    vec![
        held(Held::Hdmi { timing: Timing::P2160_30 }, 71),
        held(Held::Usb { codec: Codec::Mjpeg, w: 3840, h: 2160, fps: 45 }, 50),
        held(Held::Drawing, 12),
        held(Held::Picture { id: picture.into() }, 11),
    ]
}

fn said(r: &mut Rig, lang: Lang) -> Option<String> {
    r.core.st.own.lang = lang;
    r.toast()
}

#[test]
fn a_refusal_for_resources_names_what_to_turn_off_largest_first_in_both_languages() {
    let mut r = Rig::new();
    r.press(Key::Gallery);
    let mut hs = holders("/mnt/PHOTO/P00040.jpg");
    hs.push(held(Held::Picture { id: "/mnt/PHOTO/P00039.jpg".into() }, 2));
    r.cam.short = Some((|q| matches!(q, Request::Photo), hs));
    r.press(Key::Gallery);
    r.press(Photo);
    assert_eq!(
        said(&mut r, Lang::Ru).as_deref(),
        Some("Снимок: камере не хватает ресурсов\nПонизьте режим HDMI (2160p30), остановите поток USB на компьютере (3840×2160 · 45 к/с), закройте галерею"),
        "the menu's drawing not named, the gallery's pictures once"
    );
    assert_eq!(
        said(&mut r, Lang::En).as_deref(),
        Some("Photo: the camera is out of resources\nLower the HDMI mode (2160p30), stop the USB stream on the computer (3840×2160 · 45 fps), close the gallery")
    );
    let at = r.now;
    r.wait(5900);
    assert!(r.toast().is_some(), "a list to read stays longer than a plain toast");
    r.wait(100);
    assert!(r.toast().is_none());
    assert!(r.now - at == 6000);
}

#[test]
fn a_refusal_that_only_the_drawing_holds_is_one_line() {
    let mut r = Rig::new();
    r.cam.short = Some((|q| matches!(q, Request::Play { .. }), vec![held(Held::Drawing, 12)]));
    r.press(Key::Gallery);
    r.keys(&[Down, Down, Right]);
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Видео: камере не хватает ресурсов"));
    assert_eq!(r.gallery().view, GalleryView::Alone, "the video refused: its file alone again");
}

#[test]
fn a_control_refused_for_resources_says_it_the_same_way() {
    let mut r = Rig::new();
    let rec = vec![held(Held::Recording { w: 2560, h: 1440 }, 40), held(Held::Player { file: "/mnt/VIDEO/V00038.mp4".into() }, 20)];
    r.cam.short = Some((|q| matches!(q, Request::Set { name, .. } if name == "hdmi"), rec.clone()));
    r.open(&["Установка", "Режим HDMI"]);
    r.keys(&[Down, Right]);
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Режим HDMI: камере не хватает ресурсов\nОстановите запись (1440p), остановите видео (V00038.mp4)"));
    assert_eq!(said(&mut r, Lang::En).as_deref(), Some("HDMI mode: the camera is out of resources\nStop the recording (1440p), stop the video (V00038.mp4)"));
    r.press(Menu);
    r.cam.short = Some((|q| matches!(q, Request::Adjust { .. }), rec));
    r.press(ZoomIn);
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Зум: камере не хватает ресурсов\nОстановите запись (1440p), остановите видео (V00038.mp4)"), "an adjust too");
}

#[test]
fn a_failed_action_or_output_for_resources_says_it_the_same_way() {
    let mut r = Rig::new();
    r.event(Event::Failed { what: Action::Output(Out::Usb), code: Code::Resources, error: "e".into(), holders: vec![held(Held::Hdmi { timing: Timing::P2160_30 }, 71)] });
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Поток USB: камере не хватает ресурсов\nПонизьте режим HDMI (2160p30)"));
    assert_eq!(said(&mut r, Lang::En).as_deref(), Some("USB stream: the camera is out of resources\nLower the HDMI mode (2160p30)"));
    r.event(Event::Failed { what: Action::Output(Out::Hdmi), code: Code::Resources, error: "e".into(), holders: vec![held(Held::Usb { codec: Codec::H264, w: 1920, h: 1080, fps: 30 }, 20)] });
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Выход HDMI: камере не хватает ресурсов\nОстановите поток USB на компьютере (1920×1080 · 30 к/с)"));
    r.event(Event::Failed { what: Action::Control("record-size".into()), code: Code::Resources, error: "e".into(), holders: vec![] });
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Разрешение видео: камере не хватает ресурсов"));
    r.event(Event::Failed { what: Action::Photo, code: Code::Refused, error: "e".into(), holders: vec![] });
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Снимок не сохранён: камера не смогла"), "other codes as before");
}

#[test]
fn a_recording_pressed_while_frozen_drops_the_freeze_and_asks_again() {
    let mut r = Rig::new();
    r.press(Key::Freeze);
    r.cam.short = Some((|q| matches!(q, Request::Record), holders(&frozen_id(1))));
    r.asked();
    r.press(Record);
    assert_eq!(r.asked(), [Request::Record, Request::Record], "refused naming the frozen picture, asked again");
    assert_eq!(r.core.st.freeze, Freeze::Off);
    assert!(r.core.st.cam.running.record.is_some(), "the recording runs");
    assert!(r.toast().is_none(), "nothing to say: the recording outranks the freeze");
}

#[test]
fn a_recording_that_failed_for_the_frozen_picture_is_asked_again_once() {
    let mut r = Rig::new();
    r.keys(&[Key::Freeze, Key::Freeze]);
    r.asked();
    r.event(Event::Failed { what: Action::Record, code: Code::Resources, error: "e".into(), holders: holders(&frozen_id(1)) });
    assert_eq!((r.core.st.freeze, r.asked()), (Freeze::Off, vec![Request::Record]), "the compare's picture too");
    r.event(Event::Running { running: Running::default() });
    r.cam.short = Some((|q| matches!(q, Request::Record), holders(&frozen_id(1))));
    r.press(Record);
    assert_eq!(r.asked(), [Request::Record], "no freeze now: not asked a third time");
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Запись: камере не хватает ресурсов\nПонизьте режим HDMI (2160p30), остановите поток USB на компьютере (3840×2160 · 45 к/с), снимите стоп-кадр"));
    assert_eq!(said(&mut r, Lang::En).as_deref(), Some("Recording: the camera is out of resources\nLower the HDMI mode (2160p30), stop the USB stream on the computer (3840×2160 · 45 fps), unfreeze the picture"));
}

#[test]
fn a_freeze_that_does_not_fit_beside_a_recording_is_refused_naming_it() {
    let mut r = Rig::new();
    r.press(Record);
    r.press(Key::Freeze);
    r.event(Event::Resource { id: frozen_id(1), state: ResState::Failed { code: ResCode::Resources, error: "e".into(), holders: vec![held(Held::Recording { w: 3840, h: 2160 }, 60)] } });
    assert_eq!(r.core.st.freeze, Freeze::Off);
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Стоп-кадр: камере не хватает ресурсов\nОстановите запись (4K)"));
    assert_eq!(said(&mut r, Lang::En).as_deref(), Some("Frozen: the camera is out of resources\nStop the recording (4K)"));
}

#[test]
fn the_half_picture_of_a_freeze_is_the_same_freeze() {
    let mut r = Rig::new();
    r.press(Key::Freeze);
    r.cam.short = Some((|q| matches!(q, Request::Record), holders(&frozen_half_id(1))));
    r.asked();
    r.press(Record);
    assert_eq!((r.asked(), r.core.st.freeze), (vec![Request::Record, Request::Record], Freeze::Off), "the recording takes its room too");
    r.press(Key::Freeze);
    r.event(Event::Resource { id: frozen_half_id(2), state: ResState::Failed { code: ResCode::Resources, error: "e".into(), holders: vec![held(Held::Recording { w: 3840, h: 2160 }, 60)] } });
    assert_eq!((r.core.st.freeze, said(&mut r, Lang::Ru).as_deref()), (Freeze::Off, Some("Стоп-кадр: камере не хватает ресурсов\nОстановите запись (4K)")));
    r.event(Event::Running { running: Running::default() });
    r.cam.short = Some((|q| matches!(q, Request::Photo), holders(&frozen_half_id(9))));
    r.press(Photo);
    assert!(said(&mut r, Lang::Ru).unwrap().ends_with("снимите стоп-кадр"), "named as a freeze");
}

#[test]
fn a_picture_short_of_room_says_so_unless_it_is_a_freeze_already_left() {
    let mut r = Rig::new();
    r.press(Key::Freeze);
    r.press(Key::Freeze);
    r.press(Key::Freeze);
    let failed = |id: String| Event::Resource { id, state: ResState::Failed { code: ResCode::Resources, error: "e".into(), holders: vec![held(Held::Hdmi { timing: Timing::P2160_30 }, 71)] } };
    r.event(failed(frozen_id(1)));
    assert!(r.toast().is_none(), "the freeze is already off");
    r.press(Key::Gallery);
    r.event(failed("/mnt/PHOTO/P00040.jpg".into()));
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Галерея: камере не хватает ресурсов\nПонизьте режим HDMI (2160p30)"));
}

#[test]
fn the_menu_missing_on_an_output_is_said_once_until_it_presents_there_again() {
    let mut r = Rig::new();
    let not_shown = |revision, code| Event::NotShown { output: Out::Hdmi, revision, code, reason: "e".into(), holders: vec![held(Held::Usb { codec: Codec::Mjpeg, w: 3840, h: 2160, fps: 45 }, 50)] };
    r.event(not_shown(3, Code::Refused));
    assert!(r.toast().is_none(), "an output that cannot carry the menu leaves the user nothing to do");
    r.event(not_shown(4, Code::Resources));
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Меню на HDMI: камере не хватает ресурсов\nОстановите поток USB на компьютере (3840×2160 · 45 к/с)"));
    assert_eq!(said(&mut r, Lang::En).as_deref(), Some("The menu on HDMI: the camera is out of resources\nStop the USB stream on the computer (3840×2160 · 45 fps)"));
    r.wait(6000);
    r.event(not_shown(5, Code::Resources));
    assert!(r.toast().is_none(), "the toast's own revision not shown either: not said again");
    r.event(Event::Presented { output: Out::Hdmi, revision: 6, complete: true });
    r.event(not_shown(7, Code::Resources));
    assert!(r.toast().is_some(), "shown in between: said again");
    r.wait(5000);
    r.event(Event::Presented { output: Out::Hdmi, revision: 8, complete: true });
    r.wait(5000);
    assert!(r.toast().is_some(), "its time starts again once the output shows the menu");
    r.event(Event::Presented { output: Out::Hdmi, revision: 9, complete: true });
    r.wait(1000);
    assert!(r.toast().is_none(), "once only");
}

#[test]
fn a_compare_that_does_not_fit_goes_back_to_the_freeze_said_once_for_every_output() {
    let mut r = Rig::new();
    r.keys(&[Key::Freeze, Record, Key::Freeze]);
    assert_eq!(r.core.st.freeze, Freeze::Compare(1));
    let hs = vec![held(Held::Recording { w: 3840, h: 2160 }, 80), held(Held::Hdmi { timing: Timing::P2160_30 }, 71), held(Held::Picture { id: frozen_id(1) }, 11)];
    for output in [Out::Usb, Out::Media, Out::Hdmi] {
        r.event(Event::NotShown { output, revision: 9, code: Code::Resources, reason: "e".into(), holders: hs.clone() });
    }
    assert_eq!(r.core.st.freeze, Freeze::Frozen(1), "the recording outranks the compare");
    assert_eq!(said(&mut r, Lang::Ru).as_deref(), Some("Стоп-кадр: камере не хватает ресурсов\nОстановите запись (4K), понизьте режим HDMI (2160p30), снимите стоп-кадр"), "one toast, not one an output");
    r.wait(5000);
    r.event(Event::Presented { output: Out::Usb, revision: 10, complete: true });
    r.wait(5000);
    assert!(r.toast().is_some(), "seen only from now on: its time starts again");
}

#[test]
fn a_photo_that_fails_gives_the_card_back_to_the_computer() {
    let mut r = Rig::new();
    r.open(&["Система", "SD-карта", "Режим картридера"]);
    r.press(Menu);
    r.asked();
    r.cam.short = Some((|q| matches!(q, Request::Photo), vec![]));
    r.press(Photo);
    let set = |on| Request::Set { name: "card-reader".into(), value: Value::Bool(on) };
    assert_eq!(r.asked(), [set(false), Request::Photo, set(true)]);
    assert_eq!(r.core.st.cam.card.at, Card::Host, "as it was before the press");
}

#[test]
fn a_card_the_camera_cannot_read_is_given_back_and_only_formatted() {
    let mut r = Rig::new();
    r.open(&["Система", "SD-карта", "Режим картридера"]);
    r.press(Menu);
    r.asked();
    r.cam.unreadable = true;
    r.press(Photo);
    let set = |on| Request::Set { name: "card-reader".into(), value: Value::Bool(on) };
    assert_eq!(r.asked(), [set(false), set(true)]);
    assert_eq!(r.toast().as_deref(), Some("Снимок невозможен: камера не читает эту карту, отформатируйте её в меню камеры"));
    assert_eq!(r.core.st.cam.card.at, Card::Host);
    r.open(&["Система", "SD-карта", "Форматировать", "Форматировать"]);
    assert_eq!(r.asked(), [set(false), Request::Format]);
    assert_eq!((r.core.st.cam.card.at, r.toast().as_deref()), (Card::Camera, Some("SD-карта отформатирована")));
}
