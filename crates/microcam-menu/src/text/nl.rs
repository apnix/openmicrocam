//! Nederlands.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "nl",
    name: "Nederlands",
    decimal: ',',

    menu: "Menu",
    gallery: n("Galerij", "Foto's en video's op de SD-kaart."),
    off: "uit",
    on: "aan",
    auto: "auto",
    locked: "vergrendeld",
    cancel: "Annuleren",
    levels: [
        n("alleen beeld", "Geen elementen over het beeld."),
        n("met lijnen", "Het richtkruis en de lijnen."),
        n("met indicatoren", "De lijnen en statusindicatoren: zoom, vergrendelingen en opname."),
        n("alles", "Alles, inclusief het menu."),
    ],
    mirrors: ["geen", "horizontaal", "verticaal", "beide assen"],
    line_h: n("Horizontaal", "Een lijn van links naar rechts; deze beweegt omhoog en omlaag."),
    line_v: n("Verticaal", "Een lijn van boven naar beneden; deze beweegt naar links en rechts."),

    card_state: "Kaart",
    card_free: "Vrij",
    gb: "GB",
    card_in: "in de camera",
    card_none: "Geen geheugenkaart",
    card_host: "in gebruik door de computer",
    card_unreadable: "onleesbaar",
    card_formatting: "wordt geformatteerd",
    card_reader: n("Kaartlezermodus", "De computer herkent de camera ook als kaartlezer voor de geheugenkaart. Zolang deze modus actief is, heeft alleen de computer toegang tot de geheugenkaart."),
    format: n("Formatteren", "Verwijdert alle foto's en video's van de geheugenkaart en maakt deze opnieuw klaar voor gebruik."),
    format_ask: "Alles van de SD-kaart verwijderen?",
    format_cancel_help: "Niets verwijderen.",
    format_yes_help: "Alle foto's en video's op de geheugenkaart worden definitief verwijderd. Dit kan niet ongedaan worden gemaakt.",
    reset_ask: "Fabrieksinstellingen herstellen?",
    reset_cancel_help: "De instellingen laten zoals ze zijn.",
    reset_yes: "Herstellen",
    no_stream: "geen stream",
    fps: "fps",
    hz: "Hz",

    mirror,
    ae_auto: "Belichting: auto",
    wb_auto: "Witbalans: auto",
    ae_locked: "Belichting: vergrendeld",
    wb_locked: "Witbalans: vergrendeld",
    frozen: "Bevroren beeld",
    compare: "Links: bevroren · rechts: live",
    paused: "Gepauzeerd",
    sd: "SD-kaart",
    no_sd: "Geen SD-kaart",
    sd_host: "SD-kaart: kaartlezermodus",
    sd_unreadable: "SD-kaart: niet leesbaar",
    of,

    keys: Keys {
        choose: "selecteren",
        enter: "OK",
        back: "terug",
        close: "sluiten",
        value: "waarde",
        setting: "instelling",
        done: "klaar",
        shift: "verplaatsen",
        put: "plaatsen",
        cancel: "annuleren",
        file: "bestand",
        watch: "afspelen",
        full: "volledig scherm",
        to_menu: "naar het menu",
        to_camera: "naar de camera",
        to_strip: "naar miniaturen",
        pause: "pauzeren",
        go_on: "hervatten",
        seek: "±5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} van {whole}")
}

fn mirror(state: &str) -> String {
    format!("Spiegeling: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Beeld", "Helderheid, contrast, verzadiging en scherpte van het beeld."),
        Section::Exposure => n("Belichting", "Automatische helderheid en witbalans: aan de camera overlaten, aanpassen of vergrendelen."),
        Section::Overlays => n("Weergave", "Het richtkruis, de hulplijnen en de uitgangen waarop deze worden weergegeven."),
        Section::Recording => n("Opname", "De resolutie van de video die de camera op de SD-kaart opslaat."),
        Section::Setup => n("Setup", "Eenmalige camera-instellingen, zoals spiegeling, netfrequentie en monitor."),
        Section::System => n("Systeem", "Taal, SD-kaart, instellingen herstellen en informatie over de camera."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Helderheid", "De algemene helderheid van het beeld. Fabriekswaarde: 50."),
        Contrast => n("Contrast", "Het verschil tussen licht en donker. Fabriekswaarde: 50."),
        Saturation => n("Verzadiging", "De kleurintensiteit: van zwart-wit tot zeer levendige kleuren. Fabriekswaarde: 50."),
        Sharpness => n("Scherpte", "Versterkt randen. Hoge waarden kunnen lichte contouren rond details en extra ruis in donkere delen veroorzaken."),
        ResetPicture => n("Sectie herstellen", "Zet helderheid, contrast, verzadiging en scherpte terug naar de fabriekswaarden."),
        AutoExposure => n("Automatische belichting", "Aan — de camera bepaalt automatisch de belichtingstijd en versterking op basis van het beeld. Uit — de laatst ingestelde waarden blijven behouden."),
        ExposureBias => n("Belichtingscorrectie", "Maakt het beeld lichter of donkerder dan de automatische belichting zou kiezen. Positief als een donkere print te donker is; negatief als soldeerreflecties wit uitbijten. Werkt alleen als automatische belichting aanstaat."),
        ExposureLock => n("Belichting vergrendelen", "Houdt de huidige helderheid van het beeld vast, zodat deze niet verandert wanneer een soldeerbout of pincet in beeld komt. De AE-knop op de afstandsbediening doet hetzelfde. De vergrendeling vervalt wanneer de camera wordt uitgezet."),
        WhiteBalanceLock => n("Witbalans", "Automatisch — de camera corrigeert automatisch kleurafwijkingen door de verlichting. Vergrendeld — de huidige witbalans blijft behouden, zodat kleuren niet verschuiven wanneer één kleur het beeld overheerst, bijvoorbeeld door het groene soldeermasker van een printplaat. De AWB-knop op de afstandsbediening doet hetzelfde."),
        Cross => n("Richtkruis", "Een markering in het midden van het beeld. De +-knop op de afstandsbediening doet hetzelfde."),
        AddLine => n("Lijn toevoegen", "Een rechte lijn over het hele scherm, om de print uit te lijnen of de positie van onderdelen te vergelijken. De lijn staat vast op het scherm en beweegt niet mee met de print. Je kunt meerdere lijnen toevoegen; ze blijven niet bewaard nadat de camera is uitgezet."),
        ClearLines => n("Alle lijnen wissen", "Verwijdert alle geplaatste lijnen. Het richtkruis blijft staan."),
        LevelHdmi => n("HDMI-overlays", "Welke overlays van de camera op de monitor zichtbaar zijn: alleen het beeld, het beeld met richtkruis en lijnen, ook de statusindicatoren, of alles inclusief het menu."),
        LevelUsb => n("USB-overlays", "Hetzelfde voor het beeld dat via USB naar de computer gaat."),
        LevelMedia => n("In bestanden", "Wat in video's en foto's op de SD-kaart terechtkomt. De fabriekswaarde is alleen het beeld: lijnen en menu worden niet in het bestand opgenomen."),
        RecordSize => n("Videoresolutie", "4K — 3840×2160, grotere bestanden. 1080p — 1920×1080, er past meer op de kaart."),
        Mirror => n("Spiegeling", "Keert het beeld om wanneer de camera gedraaid boven de werktafel is gemonteerd. Kies de instelling waarbij de print op het scherm dezelfde kant op beweegt als je hand. De ⇔- en ⇕-knoppen op de afstandsbediening doen hetzelfde."),
        PowerLine => n("Netfrequentie", "De frequentie van het lichtnet dat de verlichting voedt: 50 Hz in Europa, 60 Hz in Noord-Amerika. Met de verkeerde instelling lopen er donkere banden door het beeld."),
        Aspect => n("Beeldverhouding", "De verhouding van het monitorscherm. Als een rond voorwerp ovaal lijkt, is de verkeerde verhouding gekozen."),
        Hdmi => n("HDMI-modus", "De resolutie en verversingssnelheid die de camera naar de monitor stuurt. Auto — de beste modus die de monitor meldt."),
        Priority => n("Prioriteit", "Geeft aan welke uitgang voorrang krijgt. Als de camera niet beide uitgangen op volledige kwaliteit kan aansturen, behoudt deze uitgang de volledige kwaliteit en wordt de kwaliteit van de andere uitgang verlaagd."),
        Language => n("Taal", "De taal van het menu."),
        Card => n("SD-kaart", "De vrije ruimte op de geheugenkaart, kaartlezermodus en formatteren."),
        Reset => n("Instellingen herstellen", "Zet alle instellingen terug naar de fabrieksinstellingen. Bestanden op de SD-kaart blijven behouden."),
        About => n("Over de camera", "Firmwareversie en de huidige video-uitgangsmodi."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Foto opgeslagen · {f}"),
        Msg::Recorded(f, s) => format!("Opname opgeslagen · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Beeld: fabriekswaarden".into(),
        Msg::LinesCleared => "Lijnen gewist".into(),
        Msg::Formatted => "SD-kaart geformatteerd".into(),
        Msg::SettingsReset => "Instellingen hersteld".into(),
        Msg::NoFiles => "Geen foto's of video's op de geheugenkaart".into(),
        Msg::Recording => "Galerij niet beschikbaar: opname bezig".into(),
        Msg::CardOpen(what) => format!(
            "{cannot}: de geheugenkaart is in gebruik op de computer. Werp deze eerst veilig uit.",
            cannot = cannot(what)
        ),
        Msg::CardUnreadable(what) => format!(
            "{cannot}: de camera kan de geheugenkaart niet lezen; formatteer de kaart in het cameramenu",
            cannot = cannot(what)
        ),
        Msg::Card(Card::Host) => "Kaartlezermodus staat aan".into(),
        Msg::Card(Card::Unreadable) => {
            "De camera kan deze geheugenkaart niet lezen: formatteer de kaart in het cameramenu".into()
        }
        Msg::Card(_) => "Geen SD-kaart".into(),
        Msg::NoFreeze => "Beeld bevriezen is mislukt".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "HDMI-uitgang",
                Subject::UsbStream => "USB-stream",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "Menu op HDMI",
                Subject::Menu(Output::Usb) => "Menu in de USB-stream",
                Subject::Menu(Output::Media) => "Menu in de opname",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("kies een lagere HDMI-modus ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!(
                        "stop de USB-stream op de computer ({w}×{h} · {fps} {unit})",
                        unit = WORDS.fps
                    ),
                    Todo::StopRecording(size) => format!("stop de opname ({size})"),
                    Todo::WaitPhoto => "wacht tot de foto klaar is".into(),
                    Todo::StopVideo(file) => format!("stop de video ({file})"),
                    Todo::Unfreeze => "stop de beeldbevriezing".into(),
                    Todo::CloseGallery => "sluit de galerij".into(),
                })
                .collect();
            let s = format!("{head}: {why}", why = why(Code::Resources));
            if todo.is_empty() {
                s
            } else {
                format!("{s}\n{}", capital(&todo.join(", ")))
            }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => {
                    unavailable(name).unwrap_or(why(Code::Unavailable))
                }
                (_, Some(c)) => why(*c),
                (_, None) => "mislukt",
            };
            format!("{asked}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Foto niet opgeslagen",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Opname onderbroken",
                Action::Record => "De opname is niet gestart",
                Action::RecordStop => "De opname is niet gestopt",
                Action::Format => "Geheugenkaart niet geformatteerd",
                Action::Reset => "Instellingen niet hersteld",
                Action::Play => "De video is niet gestart",
                Action::PlayStop => "De video is niet gestopt",
                Action::Control(name) => control(name),
                Action::Output(_) => "Uitgang is niet gestart",
            };
            format!("{head}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Kan geen foto maken",
        Asked::Record => "Kan niet opnemen",
        Asked::Gallery => "Galerij niet beschikbaar",
        Asked::Format => "Kan niet formatteren",
        Asked::Control(name) if name == "card-reader" => "Kan kaartlezermodus niet uitschakelen",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Foto",
        Asked::Record => "Opname",
        Asked::RecordStop => "Opname stoppen",
        Asked::Format => "Formatteren",
        Asked::Reset => "Instellingen herstellen",
        Asked::Play | Asked::Player => "Video",
        Asked::Gallery => "Galerij",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "geen SD-kaart",
        Code::CardShared => "kaartlezermodus staat aan",
        Code::CardOpen => "de geheugenkaart is geopend op de computer",
        Code::CardUnreadable => "de camera kan deze geheugenkaart niet lezen",
        Code::Busy => "de camera is nog bezig",
        Code::Playing => "er wordt een video afgespeeld",
        Code::Recording => "opname bezig",
        Code::Formatting => "de geheugenkaart wordt geformatteerd",
        Code::Unavailable => "nu niet beschikbaar",
        Code::Refused => "de camera kon dit niet uitvoeren",
        Code::Resources => "de camera heeft onvoldoende capaciteit",
        Code::Unknown | Code::Invalid => "de camera heeft het geweigerd",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Helderheid",
        "contrast" => "Contrast",
        "saturation" => "Verzadiging",
        "sharpness" => "Scherpte",
        "exposure-bias" => "Belichtingscorrectie",
        "auto-exposure" => "Automatische belichting",
        "exposure-lock" => "Belichting vergrendelen",
        "white-balance-lock" => "Witbalans vergrendelen",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Spiegeling",
        "power-line" => "Netfrequentie",
        "aspect" => "Beeldverhouding",
        "hdmi" => "HDMI-modus",
        "precedence" => "Prioriteit",
        "record-size" => "Videoresolutie",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Weergave",
        _ => "Instelling",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "automatische belichting staat uit",
        "white-balance-lock" => "automatische witbalans staat uit",
        "record-size" => "opname bezig",
        _ => return None,
    })
}
