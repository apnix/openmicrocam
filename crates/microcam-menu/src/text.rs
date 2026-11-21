//! The menu's words. A language is a file in `text/` holding every word and sentence the menu shows, as the fields of
//! one `Words`: a language that lacks one does not build. A message is a sentence its language makes whole from the
//! facts in it; the menu joins words only where the languages it has join them alike — a name and its value, a number
//! and its unit.

mod de;
mod en;
mod es;
mod fr;
mod pl;
mod ru;

use crate::menu::{Item, Section};
use microcam_core::controls::{Code, Held, Holder};
use microcam_core::protocol::{Action, Output};

/// The menu's languages. A new one: its file in `text/`, its name here, in `ALL` and in `words`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
    Pl,
    De,
    Fr,
    Es,
}

impl Lang {
    /// The languages in the order the menu lists them.
    pub const ALL: [Lang; 6] = [Lang::Ru, Lang::En, Lang::Pl, Lang::De, Lang::Fr, Lang::Es];

    pub fn words(self) -> &'static Words {
        match self {
            Lang::Ru => &ru::WORDS,
            Lang::En => &en::WORDS,
            Lang::Pl => &pl::WORDS,
            Lang::De => &de::WORDS,
            Lang::Fr => &fr::WORDS,
            Lang::Es => &es::WORDS,
        }
    }

    /// Its name in the menu's record.
    pub fn code(self) -> &'static str {
        self.words().code
    }

    pub fn of(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }
}

/// A row's name and its description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Named {
    pub name: &'static str,
    pub help: &'static str,
}

/// What a key does, in the hints.
pub struct Keys {
    pub choose: &'static str,
    pub enter: &'static str,
    pub back: &'static str,
    pub close: &'static str,
    pub value: &'static str,
    pub setting: &'static str,
    pub done: &'static str,
    /// a line being placed
    pub shift: &'static str,
    pub put: &'static str,
    pub cancel: &'static str,
    pub file: &'static str,
    pub watch: &'static str,
    pub full: &'static str,
    pub to_menu: &'static str,
    pub to_camera: &'static str,
    pub to_strip: &'static str,
    pub pause: &'static str,
    pub go_on: &'static str,
    /// the player's step back and forth
    pub seek: &'static str,
}

/// Every word and sentence of one language.
pub struct Words {
    /// its name in the menu's record
    pub code: &'static str,
    /// its own name, in the list of languages
    pub name: &'static str,
    /// the mark between a number's whole part and its tenths
    pub decimal: char,

    pub menu: &'static str,
    pub gallery: Named,
    pub off: &'static str,
    pub on: &'static str,
    pub auto: &'static str,
    pub locked: &'static str,
    pub cancel: &'static str,
    /// the drawing levels, from the picture alone to everything
    pub levels: [Named; 4],
    /// none, horizontal, vertical, both
    pub mirrors: [&'static str; 4],
    pub line_h: Named,
    pub line_v: Named,

    pub card_state: &'static str,
    pub card_free: &'static str,
    pub gb: &'static str,
    pub card_in: &'static str,
    pub card_none: &'static str,
    pub card_host: &'static str,
    pub card_unreadable: &'static str,
    pub card_formatting: &'static str,
    pub card_reader: Named,
    pub format: Named,
    pub format_ask: &'static str,
    pub format_cancel_help: &'static str,
    pub format_yes_help: &'static str,
    pub reset_ask: &'static str,
    pub reset_cancel_help: &'static str,
    pub reset_yes: &'static str,
    pub no_stream: &'static str,
    pub fps: &'static str,
    pub hz: &'static str,

    /// the mirror's chip: its state named
    pub mirror: fn(&str) -> String,
    pub ae_auto: &'static str,
    pub wb_auto: &'static str,
    pub ae_locked: &'static str,
    pub wb_locked: &'static str,
    pub frozen: &'static str,
    pub compare: &'static str,
    pub paused: &'static str,
    pub sd: &'static str,
    pub no_sd: &'static str,
    pub sd_host: &'static str,
    pub sd_unreadable: &'static str,
    /// a part and its whole: the free space of the card's size, the file's place among the files
    pub of: fn(&str, &str) -> String,

    pub keys: Keys,
    pub section: fn(Section) -> Named,
    pub item: fn(Item) -> Named,
    pub say: fn(&Msg) -> String,
}

impl Words {
    /// A number given in tenths, with one decimal.
    pub fn tenths(&self, t: u64) -> String {
        format!("{}{}{}", t / 10, self.decimal, t % 10)
    }
}

/// The same in every language.
pub const NO_VALUE: &str = "—";
pub const WAITING: &str = "…";
pub const ABOUT_NAME: &str = "OpenMicroCam";

pub fn clock(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// A message in a toast: a result or a refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    PhotoSaved(String),
    /// the file and the recording's length in seconds
    Recorded(String, u64),
    PictureReset,
    LinesCleared,
    Formatted,
    SettingsReset,
    NoFiles,
    /// the gallery refused while a recording runs
    Recording,
    /// what was asked could not be done: the computer keeps the card open, it is ejected there first
    CardOpen(Asked),
    /// what was asked could not be done: the camera cannot read the card, it is formatted in the menu first
    CardUnreadable(Asked),
    /// the card is not in the camera (none, or with the USB host), or cannot be read there
    Card(microcam_core::protocol::Card),
    /// a request refused: what was asked and why
    Refused(Asked, Option<Code>),
    /// an action accepted that did not get there
    Failed(Action, Code),
    /// the camera's picture for the freeze could not be taken
    NoFreeze,
    /// not started for lack of the camera's resources: what, and what holds them as microcamd named it
    Resources(Wanted, Vec<Holder>),
}

impl Msg {
    pub fn say(&self, lang: Lang) -> String {
        (lang.words().say)(self)
    }
}

/// What could not start for lack of resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// a request refused
    Asked(Asked),
    /// an action accepted that did not get there, or an output microcamd raises by itself
    Failed(Action),
    Freeze,
    /// a picture of the gallery
    Gallery,
    /// the menu on an output
    Menu(Output),
}

/// What was asked, as a refusal names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    Photo,
    Record,
    RecordStop,
    Format,
    Reset,
    Play,
    Player,
    Gallery,
    Control(String),
}

/// What a resources' refusal names as what could not start.
pub enum Subject {
    Asked(Asked),
    HdmiOutput,
    UsbStream,
    Freeze,
    Menu(Output),
}

impl Wanted {
    pub fn subject(&self) -> Subject {
        let asked = |a| Subject::Asked(a);
        match self {
            Wanted::Asked(a) => asked(a.clone()),
            Wanted::Failed(Action::Photo) => asked(Asked::Photo),
            Wanted::Failed(Action::Record | Action::Output(Output::Media)) => asked(Asked::Record),
            Wanted::Failed(Action::RecordStop) => asked(Asked::RecordStop),
            Wanted::Failed(Action::Format) => asked(Asked::Format),
            Wanted::Failed(Action::Reset) => asked(Asked::Reset),
            Wanted::Failed(Action::Play | Action::PlayStop) => asked(Asked::Play),
            Wanted::Failed(Action::Control(name)) => asked(Asked::Control(name.clone())),
            Wanted::Failed(Action::Output(Output::Hdmi)) => Subject::HdmiOutput,
            Wanted::Failed(Action::Output(Output::Usb)) => Subject::UsbStream,
            Wanted::Freeze => Subject::Freeze,
            Wanted::Gallery => asked(Asked::Gallery),
            Wanted::Menu(o) => Subject::Menu(*o),
        }
    }
}

/// What the user can turn off or lower to free the camera's resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Todo<'a> {
    /// HDMI at its timing
    LowerHdmi(&'a str),
    StopUsb { w: u32, h: u32, fps: u32 },
    /// the recording at its size: «4K» or «1080p»
    StopRecording(String),
    WaitPhoto,
    /// a video playing: its file's name
    StopVideo(&'a str),
    Unfreeze,
    CloseGallery,
}

/// Each holder microcamd named that the user can act on, once, in microcamd's order (the largest first). The menu's
/// drawing beside its pictures is not named: nothing the user does frees it.
pub fn todos(holders: &[Holder]) -> Vec<Todo<'_>> {
    let mut out = Vec::new();
    for h in holders {
        let one = match &h.what {
            Held::Hdmi { timing } => Todo::LowerHdmi(timing.name()),
            Held::Usb { w, h, fps, .. } => Todo::StopUsb { w: *w, h: *h, fps: *fps },
            Held::Recording { h, .. } => Todo::StopRecording(if *h >= 2160 { "4K".into() } else { format!("{h}p") }),
            Held::Photo { .. } => Todo::WaitPhoto,
            Held::Player { file } => Todo::StopVideo(file.rsplit('/').next().unwrap_or(file)),
            Held::Picture { id } if crate::core::is_frozen(id) => Todo::Unfreeze,
            Held::Picture { .. } => Todo::CloseGallery,
            Held::Drawing => continue,
        };
        if !out.contains(&one) {
            out.push(one);
        }
    }
    out
}

/// The first letter as a capital, for a part that opens a sentence.
pub fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}
