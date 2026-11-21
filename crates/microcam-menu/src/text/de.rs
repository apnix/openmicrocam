//! Deutsch.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "de",
    name: "Deutsch",
    decimal: ',',

    menu: "Menü",
    gallery: n("Galerie", "Fotos und Videos auf der SD-Karte."),
    off: "aus",
    on: "ein",
    auto: "auto",
    locked: "gesperrt",
    cancel: "Abbrechen",
    levels: [
        n("nur Bild", "Nichts über dem Bild."),
        n("mit Linien", "Fadenkreuz und Linien."),
        n("mit Statusanzeigen", "Linien und Statusanzeigen: Zoom, Sperren, Aufnahme."),
        n("alles", "Alles, auch das Menü."),
    ],
    mirrors: ["aus", "horizontal", "vertikal", "beide Achsen"],
    line_h: n("Horizontal", "Linie von links nach rechts; lässt sich nach oben und unten verschieben."),
    line_v: n("Vertikal", "Linie von oben nach unten; lässt sich nach links und rechts verschieben."),

    card_state: "Karte",
    card_free: "Frei",
    gb: "GB",
    card_in: "in der Kamera",
    card_none: "Keine Karte",
    card_host: "beim Computer",
    card_unreadable: "nicht lesbar",
    card_formatting: "wird formatiert",
    card_reader: n("Kartenleser-Modus", "Der Computer sieht die Kamera zusätzlich als Kartenleser mit dieser Karte. Solange der Modus eingeschaltet ist, gehört die Karte dem Computer."),
    format: n("Formatieren", "Löscht alle Fotos und Videos von der Karte und richtet sie neu ein."),
    format_ask: "Alles auf der SD-Karte löschen?",
    format_cancel_help: "Nichts löschen.",
    format_yes_help: "Alle Fotos und Videos auf der Karte werden gelöscht und lassen sich nicht wiederherstellen.",
    reset_ask: "Werkseinstellungen wiederherstellen?",
    reset_cancel_help: "Einstellungen so lassen, wie sie sind.",
    reset_yes: "Zurücksetzen",
    no_stream: "kein Stream",
    fps: "B/s",
    hz: "Hz",

    mirror,
    ae_auto: "Belichtung: auto",
    wb_auto: "Weißabgleich: auto",
    ae_locked: "Belichtung: gesperrt",
    wb_locked: "Weißabgleich: gesperrt",
    frozen: "Standbild",
    compare: "Links Standbild · rechts Kamera",
    paused: "Pause",
    sd: "SD",
    no_sd: "keine SD",
    sd_host: "SD: Kartenleser-Modus",
    sd_unreadable: "SD: nicht lesbar",
    of,

    keys: Keys {
        choose: "Auswahl",
        enter: "OK",
        back: "zurück",
        close: "schließen",
        value: "Wert",
        setting: "Parameter",
        done: "fertig",
        shift: "verschieben",
        put: "setzen",
        cancel: "abbrechen",
        file: "Datei",
        watch: "abspielen",
        full: "Vollbild",
        to_menu: "zum Menü",
        to_camera: "zur Kamera",
        to_strip: "zur Leiste",
        pause: "Pause",
        go_on: "fortsetzen",
        seek: "±5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} von {whole}")
}

fn mirror(state: &str) -> String {
    format!("Spiegelung: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Bild", "Helligkeit, Kontrast, Sättigung und Schärfe des Bildes."),
        Section::Exposure => n("Belichtung", "Belichtungsautomatik und Weißabgleich: der Kamera überlassen, korrigieren oder sperren."),
        Section::Overlays => n("Einblendungen", "Fadenkreuz und Linien über dem Bild; auf welchen Ausgängen sie zu sehen sind."),
        Section::Recording => n("Aufnahme", "Auflösung der Videos, die die Kamera auf die SD-Karte schreibt."),
        Section::Setup => n("Einrichtung", "Was einmal für den Arbeitsplatz eingestellt wird: Spiegelung, Stromnetz, Monitor."),
        Section::System => n("System", "Sprache, SD-Karte, Werkseinstellungen und Infos zur Kamera."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Helligkeit", "Gesamthelligkeit des Bildes. Werkswert: 50."),
        Contrast => n("Kontrast", "Unterschied zwischen Hell und Dunkel. Werkswert: 50."),
        Saturation => n("Sättigung", "Farbstärke: von einem Schwarzweißbild bis zu sehr kräftigen Farben. Werkswert: 50."),
        Sharpness => n("Schärfe", "Betont die Konturen. Hohe Werte erzeugen helle Säume um Leiterbahnen und Rauschen in dunklen Bereichen."),
        ResetPicture => n("Bereich zurücksetzen", "Setzt Helligkeit, Kontrast, Sättigung und Schärfe auf die Werkswerte zurück."),
        AutoExposure => n("Belichtungsautomatik", "Ein – die Kamera wählt Belichtungszeit und Verstärkung passend zum Bildinhalt. Aus – beide bleiben so, wie sie beim Ausschalten waren."),
        ExposureBias => n("Belichtungskorrektur", "Macht das Bild heller oder dunkler, als es die Automatik wählen würde. Plus, wenn eine dunkle Platine zu dunkel erscheint; Minus, wenn Reflexe auf dem Lot zu Weiß ausbrennen. Wirkt nur bei eingeschalteter Belichtungsautomatik."),
        ExposureLock => n("Belichtungssperre", "Hält die aktuelle Bildhelligkeit fest: Sie schwankt nicht mehr, wenn Lötkolben oder Pinzette ins Bild kommen. Dasselbe bewirkt die Taste AE auf der Fernbedienung. Nach dem Ausschalten der Kamera ist die Sperre aufgehoben."),
        WhiteBalanceLock => n("Weißabgleich", "Auto – die Kamera gleicht den Farbstich des Lichts selbst aus. Gesperrt – der aktuelle Abgleich bleibt erhalten, damit sich die Farben nicht verschieben, wenn eine Farbe das Bild beherrscht, etwa der grüne Lötstopplack einer Platine. Dasselbe bewirkt die Taste AWB auf der Fernbedienung."),
        Cross => n("Fadenkreuz", "Markierung in der Bildmitte. Dasselbe bewirkt die Taste + auf der Fernbedienung."),
        AddLine => n("Linie hinzufügen", "Eine Gerade über den ganzen Bildschirm – zum Ausrichten der Platine oder zum Vergleichen von Bauteilpositionen. Die Linie ist am Bildschirm fixiert und wandert nicht mit der Platine mit. Beliebig viele Linien sind möglich; nach dem Ausschalten der Kamera bleiben sie nicht erhalten."),
        ClearLines => n("Alle Linien entfernen", "Entfernt alle gesetzten Linien. Das Fadenkreuz bleibt."),
        LevelHdmi => n("Auf HDMI", "Welche Einblendungen der Kamera auf dem Monitor zu sehen sind: nur das Bild, das Bild mit Fadenkreuz und Linien, zusätzlich die Statusanzeigen oder alles samt Menü."),
        LevelUsb => n("Auf USB", "Dasselbe für das Bild, das über USB an den Computer geht."),
        LevelMedia => n("In Dateien", "Was in die Videos und Fotos auf der SD-Karte gelangt. Werkswert ist nur das Bild: Linien und Menü werden nicht in die Datei geschrieben."),
        RecordSize => n("Videoauflösung", "4K – 3840×2160, größere Dateien. 1080p – 1920×1080, es passt mehr auf die Karte."),
        Mirror => n("Spiegelung", "Spiegelt das Bild, wenn die Kamera verdreht über dem Tisch montiert ist. So einstellen, dass sich die Platine auf dem Bildschirm in dieselbe Richtung bewegt, in die die Hand sie schiebt. Dasselbe bewirken die Tasten ⇔ und ⇕ auf der Fernbedienung."),
        PowerLine => n("Netzfrequenz", "Frequenz des Stromnetzes, das die Beleuchtung speist: 50 Hz in Europa, 60 Hz in Nordamerika. Bei falscher Einstellung laufen dunkle Streifen durchs Bild."),
        Aspect => n("Bildformat", "Seitenverhältnis des Monitors. Erscheint etwas Rundes auf dem Bildschirm oval, ist das falsche Format gewählt."),
        Hdmi => n("HDMI-Modus", "Auflösung und Bildfrequenz, mit denen die Kamera das Bild an den Monitor ausgibt. Auto – der beste Modus, den der Monitor gemeldet hat."),
        Priority => n("Priorität", "Welcher Ausgang wichtiger ist. Reicht die Leistung der Kamera nicht für beide zugleich, behält dieser die volle Qualität, beim anderen wird sie verringert."),
        Language => n("Sprache", "Sprache des Menüs."),
        Card => n("SD-Karte", "Freier Speicher auf der Karte, Kartenleser-Modus und Formatieren."),
        Reset => n("Werkseinstellungen", "Setzt alle Einstellungen auf die Werkswerte zurück. Dateien auf der SD-Karte bleiben unberührt."),
        About => n("Über die Kamera", "Firmware-Version und aktuelle Modi der Ausgänge."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Foto gespeichert · {f}"),
        Msg::Recorded(f, s) => format!("Aufnahme gespeichert · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Bild: Werkswerte".into(),
        Msg::LinesCleared => "Linien entfernt".into(),
        Msg::Formatted => "SD-Karte formatiert".into(),
        Msg::SettingsReset => "Einstellungen zurückgesetzt".into(),
        Msg::NoFiles => "Keine Fotos oder Videos auf der Karte".into(),
        Msg::Recording => "Galerie nicht verfügbar: Aufnahme läuft".into(),
        Msg::CardOpen(what) => format!("{cannot}: Karte ist am Computer geöffnet, dort auswerfen", cannot = cannot(what)),
        Msg::CardUnreadable(what) => format!("{cannot}: Kamera kann diese Karte nicht lesen, im Kameramenü formatieren", cannot = cannot(what)),
        Msg::Card(Card::Host) => "Kartenleser-Modus ist eingeschaltet".into(),
        Msg::Card(Card::Unreadable) => "Kamera kann diese Karte nicht lesen: im Kameramenü formatieren".into(),
        Msg::Card(_) => "Keine SD-Karte".into(),
        Msg::NoFreeze => "Standbild fehlgeschlagen".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "HDMI-Ausgang",
                Subject::UsbStream => "USB-Stream",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "Menü auf HDMI",
                Subject::Menu(Output::Usb) => "Menü im USB-Stream",
                Subject::Menu(Output::Media) => "Menü in der Aufnahme",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("HDMI-Modus herabsetzen ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!("USB-Stream am Computer beenden ({w}×{h} · {fps} {unit})", unit = WORDS.fps),
                    Todo::StopRecording(size) => format!("Aufnahme beenden ({size})"),
                    Todo::WaitPhoto => "auf das Foto warten".into(),
                    Todo::StopVideo(file) => format!("Video beenden ({file})"),
                    Todo::Unfreeze => "Standbild aufheben".into(),
                    Todo::CloseGallery => "Galerie schließen".into(),
                })
                .collect();
            let s = format!("{head}: {why}", why = why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "fehlgeschlagen",
            };
            format!("{asked}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Foto nicht gespeichert",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Aufnahme abgebrochen",
                Action::Record => "Aufnahme nicht gestartet",
                Action::RecordStop => "Aufnahme nicht beendet",
                Action::Format => "Karte nicht formatiert",
                Action::Reset => "Einstellungen nicht zurückgesetzt",
                Action::Play => "Video nicht gestartet",
                Action::PlayStop => "Video nicht beendet",
                Action::Control(name) => control(name),
                Action::Output(_) => "Ausgang nicht gestartet",
            };
            format!("{head}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Foto nicht möglich",
        Asked::Record => "Aufnahme nicht möglich",
        Asked::Gallery => "Galerie nicht verfügbar",
        Asked::Format => "Formatieren nicht möglich",
        Asked::Control(name) if name == "card-reader" => "Kartenleser-Modus bleibt eingeschaltet",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Foto",
        Asked::Record => "Aufnahme",
        Asked::RecordStop => "Aufnahme beenden",
        Asked::Format => "Formatieren",
        Asked::Reset => "Zurücksetzen",
        Asked::Play | Asked::Player => "Video",
        Asked::Gallery => "Galerie",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "keine SD-Karte",
        Code::CardShared => "Kartenleser-Modus ist eingeschaltet",
        Code::CardOpen => "Karte ist am Computer geöffnet",
        Code::CardUnreadable => "Kamera kann diese Karte nicht lesen",
        Code::Busy => "Kamera ist noch beschäftigt",
        Code::Playing => "Video wird abgespielt",
        Code::Recording => "Aufnahme läuft",
        Code::Formatting => "Karte wird formatiert",
        Code::Unavailable => "derzeit nicht verfügbar",
        Code::Refused => "Kamera konnte es nicht ausführen",
        Code::Resources => "Kamera hat zu wenig Ressourcen",
        Code::Unknown | Code::Invalid => "Kamera hat es nicht angenommen",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Helligkeit",
        "contrast" => "Kontrast",
        "saturation" => "Sättigung",
        "sharpness" => "Schärfe",
        "exposure-bias" => "Belichtungskorrektur",
        "auto-exposure" => "Belichtungsautomatik",
        "exposure-lock" => "Belichtungssperre",
        "white-balance-lock" => "Weißabgleichsperre",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Spiegelung",
        "power-line" => "Netzfrequenz",
        "aspect" => "Bildformat",
        "hdmi" => "HDMI-Modus",
        "precedence" => "Priorität",
        "record-size" => "Videoauflösung",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Anzeige",
        _ => "Einstellung",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "Belichtungsautomatik ist ausgeschaltet",
        "white-balance-lock" => "automatischer Weißabgleich ist ausgeschaltet",
        "record-size" => "Aufnahme läuft",
        _ => return None,
    })
}
