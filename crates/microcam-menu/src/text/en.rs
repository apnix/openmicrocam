//! English.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "en",
    name: "English",
    decimal: '.',

    menu: "Menu",
    gallery: n("Gallery", "Photos and videos on the SD card."),
    off: "off",
    on: "on",
    auto: "auto",
    locked: "locked",
    cancel: "Cancel",
    levels: [
        n("picture only", "Nothing over the picture."),
        n("with lines", "The crosshair and the lines."),
        n("with chips", "The lines and the state chips: zoom, locks, recording."),
        n("everything", "Everything, the menu included."),
    ],
    mirrors: ["none", "horizontal", "vertical", "both"],
    line_h: n("Horizontal", "A line from left to right; it moves up and down."),
    line_v: n("Vertical", "A line from top to bottom; it moves left and right."),

    card_state: "Card",
    card_free: "Free",
    gb: "GB",
    card_in: "in the camera",
    card_none: "No card",
    card_host: "with the computer",
    card_unreadable: "unreadable",
    card_formatting: "formatting",
    card_reader: n("Card reader mode", "The computer sees the camera as a card reader with this card in it, too. While the mode is on, the card is the computer's."),
    format: n("Format", "Erases every photo and video on the card and lays it out anew."),
    format_ask: "Erase everything on the SD card?",
    format_cancel_help: "Erase nothing.",
    format_yes_help: "Every photo and video on the card will be deleted, with no way back.",
    reset_ask: "Back to the factory settings?",
    reset_cancel_help: "Leave the settings as they are.",
    reset_yes: "Reset",
    no_stream: "no stream",
    fps: "fps",
    hz: "Hz",

    mirror_is: "Mirror",
    ae_auto: "Exposure: auto",
    wb_auto: "White balance: auto",
    ae_locked: "Exposure: locked",
    wb_locked: "Balance: locked",
    frozen: "Frozen",
    compare: "Frozen on the left · the camera on the right",
    paused: "Paused",
    sd: "SD",
    no_sd: "no SD",
    sd_host: "SD: card reader mode",
    sd_unreadable: "SD: unreadable",
    of,

    keys: Keys {
        choose: "choose",
        enter: "enter",
        back: "back",
        close: "close",
        value: "value",
        setting: "setting",
        done: "done",
        shift: "move",
        put: "put",
        cancel: "cancel",
        file: "file",
        watch: "play",
        full: "full screen",
        to_menu: "to the menu",
        to_camera: "to the camera",
        to_strip: "to the strip",
        pause: "pause",
        go_on: "go on",
        seek: "5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} of {whole}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Picture", "The picture's brightness, contrast, saturation and sharpness."),
        Section::Exposure => n("Exposure", "The automatic brightness of the frame and the white balance: leave them to the camera, shift them or hold them."),
        Section::Overlays => n("Overlays", "The crosshair and lines over the picture, and on which outputs to show them."),
        Section::Recording => n("Recording", "The quality of the video the camera writes to the SD card."),
        Section::Setup => n("Setup", "What is set once for the workplace: the mirror, the mains, the monitor."),
        Section::System => n("System", "The language, the SD card, the settings reset and what the camera is."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Brightness", "The picture's overall brightness. The factory value is 50."),
        Contrast => n("Contrast", "The difference between light and dark. The factory value is 50."),
        Saturation => n("Saturation", "The strength of colour: from a grey picture to a very bright one. The factory value is 50."),
        Sharpness => n("Sharpness", "How much edges are stressed. High values give a light fringe around the traces and noise in the dark."),
        ResetPicture => n("Reset the section", "Brightness, contrast, saturation and sharpness back to their factory values."),
        AutoExposure => n("Auto exposure", "On — the camera picks the exposure time and gain for what is in the frame. Off — they stay as they were at the moment it was switched off."),
        ExposureBias => n("Exposure shift", "Makes the picture lighter or darker than the automatics would. Plus when a dark board comes out too dark, minus when the glints on solder burn out to white. Works only with auto exposure on."),
        ExposureLock => n("Exposure lock", "Holds the frame's brightness as it is now: it stops drifting when an iron or tweezers come into the frame. The AE key on the remote does the same. After a power-off the lock is gone."),
        WhiteBalanceLock => n("White balance", "Auto — the camera takes the light's tint away by itself. The lock holds the balance as it is now, so the colours do not drift when much of the frame is one colour — the board's green mask, say. The AWB key on the remote does the same."),
        Cross => n("Crosshair", "A mark in the centre of the frame. The + key on the remote does the same."),
        AddLine => n("Add a line", "A straight line across the whole screen — to align the board or check where parts sit. The line belongs to the screen and does not follow the board. There may be any number of lines; after a power-off they are gone."),
        ClearLines => n("Remove all lines", "Removes every line put. The crosshair stays."),
        LevelHdmi => n("Shown on HDMI", "What of the camera's drawing the monitor shows: the picture alone, the picture with the crosshair and lines, the state chips as well, or everything with the menu."),
        LevelUsb => n("Shown on USB", "The same for the picture that goes to the computer over USB."),
        LevelMedia => n("In recordings and photos", "What goes into the videos and photos on the SD card. The factory value is the picture alone: lines and the menu are not written into the file."),
        RecordSize => n("Video quality", "4K — 3840×2160, larger files. 1080p — 1920×1080, more fits on the card."),
        Mirror => n("Mirror", "Turns the picture over when the camera stands turned above the desk. Pick it so the board on the screen moves where the hand moves it. The ⇔ and ⇕ keys on the remote do the same."),
        PowerLine => n("Mains frequency", "The frequency of the mains the lights run on: 50 Hz in Europe and Russia, 60 Hz in America. A wrong one sends dark bands over the picture."),
        Aspect => n("Screen format", "The monitor's aspect ratio. If something round looks oval on the screen, the format is wrong."),
        Hdmi => n("HDMI mode", "The resolution and rate the camera gives the monitor. Auto — the best of what the monitor said it takes."),
        Priority => n("Priority", "Which output matters more. When the camera cannot serve both in full, this one keeps its full quality and the other's is lowered."),
        Language => n("Language", "The menu's language."),
        Card => n("SD card", "The card's free space, card reader mode and formatting."),
        Reset => n("Settings reset", "Every setting back to the factory's. The files on the SD card stay."),
        About => n("About the camera", "The firmware's version and the outputs' modes now."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Photo saved · {f}"),
        Msg::Recorded(f, s) => format!("Recording saved · {f} · {}", clock(*s)),
        Msg::PictureReset => "Picture: factory values".into(),
        Msg::LinesCleared => "Lines removed".into(),
        Msg::Formatted => "SD card formatted".into(),
        Msg::SettingsReset => "Settings reset".into(),
        Msg::NoFiles => "No photos or videos on the card".into(),
        Msg::Recording => "Recording".into(),
        Msg::CardOpen(what) => format!("{}: the card is open on the computer, eject it there", cannot(what)),
        Msg::CardUnreadable(what) => format!("{}: the camera cannot read this card, format it in the camera's menu", cannot(what)),
        Msg::Card(Card::Host) => "Card reader mode is on".into(),
        Msg::Card(Card::Unreadable) => "The camera cannot read this card: format it in the camera's menu".into(),
        Msg::Card(_) => "No SD card".into(),
        Msg::NoFreeze => "Could not freeze the picture".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "HDMI output",
                Subject::UsbStream => "USB stream",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "The menu on HDMI",
                Subject::Menu(Output::Usb) => "The menu in the USB stream",
                Subject::Menu(Output::Media) => "The menu in the recording",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("lower the HDMI mode ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!("stop the USB stream on the computer ({w}×{h} · {fps} {})", WORDS.fps),
                    Todo::StopRecording(size) => format!("stop the recording ({size})"),
                    Todo::WaitPhoto => "wait for the photo".into(),
                    Todo::StopVideo(file) => format!("stop the video ({file})"),
                    Todo::Unfreeze => "unfreeze the picture".into(),
                    Todo::CloseGallery => "close the gallery".into(),
                })
                .collect();
            let s = format!("{head}: {}", why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "did not work",
            };
            format!("{}: {reason}", asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Photo not saved",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Recording broken off",
                Action::Record => "The recording did not start",
                Action::RecordStop => "The recording did not stop",
                Action::Format => "Card not formatted",
                Action::Reset => "Settings not reset",
                Action::Play => "The video did not start",
                Action::PlayStop => "The video did not stop",
                Action::Control(name) => control(name),
                Action::Output(_) => "An output left out",
            };
            format!("{head}: {}", why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "No photo",
        Asked::Record => "No recording",
        Asked::Gallery => "No gallery",
        Asked::Format => "No format",
        Asked::Control(name) if name == "card-reader" => "Card reader mode stays on",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Photo",
        Asked::Record => "Recording",
        Asked::RecordStop => "Stopping the recording",
        Asked::Format => "Formatting",
        Asked::Reset => "Settings reset",
        Asked::Play | Asked::Player => "Video",
        Asked::Gallery => "Gallery",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "no SD card",
        Code::CardShared => "card reader mode is on",
        Code::CardOpen => "the card is open on the computer",
        Code::CardUnreadable => "the camera cannot read this card",
        Code::Busy => "the camera is still busy with the last one",
        Code::Playing => "a video is playing",
        Code::Recording => "a recording is running",
        Code::Formatting => "the card is being formatted",
        Code::Unavailable => "not available now",
        Code::Refused => "the camera could not",
        Code::Resources => "the camera lacks the resources",
        Code::Unknown | Code::Invalid => "the camera did not take it",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Brightness",
        "contrast" => "Contrast",
        "saturation" => "Saturation",
        "sharpness" => "Sharpness",
        "exposure-bias" => "Exposure shift",
        "auto-exposure" => "Auto exposure",
        "exposure-lock" => "Exposure lock",
        "white-balance-lock" => "White balance lock",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Mirror",
        "power-line" => "Mains frequency",
        "aspect" => "Screen format",
        "hdmi" => "HDMI mode",
        "precedence" => "Priority",
        "record-size" => "Video quality",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Display",
        _ => "Setting",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "auto exposure is off",
        "white-balance-lock" => "auto white balance is off",
        "record-size" => "a recording is running",
        _ => return None,
    })
}
