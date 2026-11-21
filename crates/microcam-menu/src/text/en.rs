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
        n("with indicators", "The lines and the status indicators: zoom, locks, recording."),
        n("everything", "Everything, the menu included."),
    ],
    mirrors: ["none", "horizontal", "vertical", "both axes"],
    line_h: n("Horizontal", "A line from left to right; it moves up and down."),
    line_v: n("Vertical", "A line from top to bottom; it moves left and right."),

    card_state: "Card",
    card_free: "Free",
    gb: "GB",
    card_in: "in the camera",
    card_none: "No card",
    card_host: "in use by the computer",
    card_unreadable: "unreadable",
    card_formatting: "formatting",
    card_reader: n("Card reader mode", "The computer also sees the camera as a card reader with this card in it. While the mode is on, only the computer can access the card."),
    format: n("Format", "Erases all photos and videos from the card and prepares it from scratch."),
    format_ask: "Erase everything on the SD card?",
    format_cancel_help: "Erase nothing.",
    format_yes_help: "All photos and videos on the card will be deleted. This cannot be undone.",
    reset_ask: "Restore factory settings?",
    reset_cancel_help: "Leave the settings as they are.",
    reset_yes: "Reset",
    no_stream: "no stream",
    fps: "fps",
    hz: "Hz",

    mirror,
    ae_auto: "Exposure: auto",
    wb_auto: "White balance: auto",
    ae_locked: "Exposure: locked",
    wb_locked: "White balance: locked",
    frozen: "Frozen",
    compare: "Left: frozen · right: live",
    paused: "Paused",
    sd: "SD",
    no_sd: "no SD",
    sd_host: "SD: card reader mode",
    sd_unreadable: "SD: unreadable",
    of,

    keys: Keys {
        choose: "select",
        enter: "OK",
        back: "back",
        close: "close",
        value: "value",
        setting: "setting",
        done: "done",
        shift: "move",
        put: "place",
        cancel: "cancel",
        file: "file",
        watch: "play",
        full: "full screen",
        to_menu: "to the menu",
        to_camera: "to the camera",
        to_strip: "to thumbnails",
        pause: "pause",
        go_on: "resume",
        seek: "±5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} of {whole}")
}

fn mirror(state: &str) -> String {
    format!("Mirror: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Picture", "The picture's brightness, contrast, saturation and sharpness."),
        Section::Exposure => n("Exposure", "Automatic frame brightness and white balance: leave them to the camera, adjust them or lock them."),
        Section::Overlays => n("Overlays", "The crosshair and lines over the picture, and the outputs that show them."),
        Section::Recording => n("Recording", "The resolution of the video the camera writes to the SD card."),
        Section::Setup => n("Setup", "Settings made once for the workstation: mirror, mains frequency, monitor."),
        Section::System => n("System", "Language, SD card, settings reset and information about the camera."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Brightness", "The picture's overall brightness. The factory value is 50."),
        Contrast => n("Contrast", "The difference between light and dark. The factory value is 50."),
        Saturation => n("Saturation", "The strength of colour: from a black-and-white picture to very vivid colours. The factory value is 50."),
        Sharpness => n("Sharpness", "Edge enhancement. High values give a light fringe around traces and noise in dark areas."),
        ResetPicture => n("Reset section", "Returns brightness, contrast, saturation and sharpness to their factory values."),
        AutoExposure => n("Auto exposure", "On — the camera picks the exposure time and gain to suit the frame. Off — they stay as they were when it was switched off."),
        ExposureBias => n("Exposure compensation", "Makes the picture lighter or darker than auto exposure would choose. Plus when a dark board comes out too dark, minus when highlights on solder blow out to white. Works only with auto exposure on."),
        ExposureLock => n("Exposure lock", "Holds the frame's brightness as it is now: it stops drifting when a soldering iron or tweezers come into the frame. The AE key on the remote does the same. The lock is released when the camera is switched off."),
        WhiteBalanceLock => n("White balance", "Auto — the camera removes the colour cast of the light by itself. The lock holds the balance as it is now, so the colours do not drift when much of the frame is one colour, such as a board's green solder mask. The AWB key on the remote does the same."),
        Cross => n("Crosshair", "A mark in the centre of the frame. The + key on the remote does the same."),
        AddLine => n("Add line", "A straight line across the whole screen — to align the board or compare the positions of parts. The line is fixed to the screen and does not move with the board. You can add any number of lines; they are not kept after power-off."),
        ClearLines => n("Remove all lines", "Removes all the lines you placed. The crosshair stays."),
        LevelHdmi => n("HDMI overlays", "Which of the camera's overlays the monitor shows: the picture only, the picture with the crosshair and lines, the status indicators as well, or everything including the menu."),
        LevelUsb => n("USB overlays", "The same for the picture that goes to the computer over USB."),
        LevelMedia => n("In files", "What goes into the videos and photos on the SD card. The factory value is picture only: lines and the menu are not written to the file."),
        RecordSize => n("Video resolution", "4K — 3840×2160, larger files. 1080p — 1920×1080, more fits on the card."),
        Mirror => n("Mirror", "Flips the picture when the camera is mounted rotated above the desk. Choose the setting that makes the board on the screen move the same way your hand moves it. The ⇔ and ⇕ keys on the remote do the same."),
        PowerLine => n("Mains frequency", "The frequency of the mains that power the lighting: 50 Hz in Europe, 60 Hz in North America. With the wrong setting, dark bands run across the picture."),
        Aspect => n("Aspect ratio", "The shape of the monitor's screen. If something round looks oval on the screen, the wrong ratio is selected."),
        Hdmi => n("HDMI mode", "The resolution and refresh rate the camera sends to the monitor. Auto — the best mode the monitor reports."),
        Priority => n("Priority", "Which output matters more. When the camera cannot serve both in full, this one keeps full quality and the other is reduced."),
        Language => n("Language", "The menu's language."),
        Card => n("SD card", "The card's free space, card reader mode and formatting."),
        Reset => n("Reset settings", "Returns all settings to their factory values. Files on the SD card are not touched."),
        About => n("About the camera", "Firmware version and the current output modes."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Photo saved · {f}"),
        Msg::Recorded(f, s) => format!("Recording saved · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Picture: factory values".into(),
        Msg::LinesCleared => "Lines removed".into(),
        Msg::Formatted => "SD card formatted".into(),
        Msg::SettingsReset => "Settings reset".into(),
        Msg::NoFiles => "No photos or videos on the card".into(),
        Msg::Recording => "Gallery unavailable: recording in progress".into(),
        Msg::CardOpen(what) => format!("{cannot}: the card is open on the computer, eject it there", cannot = cannot(what)),
        Msg::CardUnreadable(what) => format!("{cannot}: the camera cannot read this card, format it in the camera's menu", cannot = cannot(what)),
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
                    Todo::StopUsb { w, h, fps } => format!("stop the USB stream on the computer ({w}×{h} · {fps} {unit})", unit = WORDS.fps),
                    Todo::StopRecording(size) => format!("stop the recording ({size})"),
                    Todo::WaitPhoto => "wait for the photo to finish".into(),
                    Todo::StopVideo(file) => format!("stop the video ({file})"),
                    Todo::Unfreeze => "unfreeze the picture".into(),
                    Todo::CloseGallery => "close the gallery".into(),
                })
                .collect();
            let s = format!("{head}: {why}", why = why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "failed",
            };
            format!("{asked}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Photo not saved",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Recording interrupted",
                Action::Record => "The recording did not start",
                Action::RecordStop => "The recording did not stop",
                Action::Format => "Card not formatted",
                Action::Reset => "Settings not reset",
                Action::Play => "The video did not start",
                Action::PlayStop => "The video did not stop",
                Action::Control(name) => control(name),
                Action::Output(_) => "Output did not start",
            };
            format!("{head}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Cannot take a photo",
        Asked::Record => "Cannot record",
        Asked::Gallery => "Gallery unavailable",
        Asked::Format => "Cannot format",
        Asked::Control(name) if name == "card-reader" => "Cannot turn off card reader mode",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Photo",
        Asked::Record => "Recording",
        Asked::RecordStop => "Stop recording",
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
        Code::Busy => "the camera is still busy",
        Code::Playing => "a video is playing",
        Code::Recording => "recording in progress",
        Code::Formatting => "the card is being formatted",
        Code::Unavailable => "not available right now",
        Code::Refused => "the camera could not do it",
        Code::Resources => "the camera is out of resources",
        Code::Unknown | Code::Invalid => "the camera rejected it",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Brightness",
        "contrast" => "Contrast",
        "saturation" => "Saturation",
        "sharpness" => "Sharpness",
        "exposure-bias" => "Exposure compensation",
        "auto-exposure" => "Auto exposure",
        "exposure-lock" => "Exposure lock",
        "white-balance-lock" => "White balance lock",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Mirror",
        "power-line" => "Mains frequency",
        "aspect" => "Aspect ratio",
        "hdmi" => "HDMI mode",
        "precedence" => "Priority",
        "record-size" => "Video resolution",
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
        "record-size" => "recording in progress",
        _ => return None,
    })
}
