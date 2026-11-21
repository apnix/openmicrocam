//! Polski.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "pl",
    name: "Polski",
    decimal: ',',

    menu: "Menu",
    gallery: n("Galeria", "Zdjęcia i filmy z karty SD."),
    off: "wył.",
    on: "wł.",
    auto: "auto",
    locked: "blokada",
    cancel: "Anuluj",
    levels: [
        n("sam obraz", "Żadnych nakładek na obrazie."),
        n("z liniami", "Krzyż celowniczy i linie."),
        n("ze wskaźnikami", "Linie i wskaźniki stanu: zoom, blokady, nagrywanie."),
        n("wszystko", "Wszystko, razem z menu."),
    ],
    mirrors: ["brak", "w poziomie", "w pionie", "w obu osiach"],
    line_h: n("Pozioma", "Linia od lewej do prawej; przesuwa się w górę i w dół."),
    line_v: n("Pionowa", "Linia z góry na dół; przesuwa się w lewo i w prawo."),

    card_state: "Karta",
    card_free: "Wolne",
    gb: "GB",
    card_in: "w kamerze",
    card_none: "Brak karty",
    card_host: "oddana komputerowi",
    card_unreadable: "nieczytelna",
    card_formatting: "formatowanie",
    card_reader: n("Tryb czytnika kart", "Komputer widzi kamerę także jako czytnik z tą kartą. Dopóki tryb jest włączony, kartą dysponuje komputer."),
    format: n("Formatuj", "Usuwa z karty wszystkie zdjęcia i filmy i przygotowuje ją od nowa."),
    format_ask: "Usunąć wszystko z karty SD?",
    format_cancel_help: "Niczego nie usuwać.",
    format_yes_help: "Wszystkie zdjęcia i filmy na karcie zostaną usunięte, nie da się ich odzyskać.",
    reset_ask: "Przywrócić ustawienia fabryczne?",
    reset_cancel_help: "Zostawić ustawienia bez zmian.",
    reset_yes: "Przywróć",
    no_stream: "brak strumienia",
    fps: "kl./s",
    hz: "Hz",

    mirror,
    ae_auto: "Ekspozycja: auto",
    wb_auto: "Balans bieli: auto",
    ae_locked: "Ekspozycja: blokada",
    wb_locked: "Balans bieli: blokada",
    frozen: "Stop-klatka",
    compare: "Z lewej stop-klatka · z prawej kamera",
    paused: "Pauza",
    sd: "SD",
    no_sd: "brak SD",
    sd_host: "SD: tryb czytnika kart",
    sd_unreadable: "SD: nieczytelna",
    of,

    keys: Keys {
        choose: "wybór",
        enter: "OK",
        back: "wstecz",
        close: "zamknij",
        value: "wartość",
        setting: "parametr",
        done: "gotowe",
        shift: "przesuń",
        put: "postaw",
        cancel: "anuluj",
        file: "plik",
        watch: "odtwórz",
        full: "pełny ekran",
        to_menu: "do menu",
        to_camera: "do kamery",
        to_strip: "do paska",
        pause: "pauza",
        go_on: "wznów",
        seek: "±5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} z {whole}")
}

fn mirror(state: &str) -> String {
    format!("Odbicie lustrzane: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Obraz", "Jasność, kontrast, nasycenie i ostrość obrazu."),
        Section::Exposure => n("Ekspozycja", "Automatyka jasności kadru i balans bieli: zostawić kamerze, skorygować albo zablokować."),
        Section::Overlays => n("Nakładki", "Krzyż celowniczy i linie na obrazie; na których wyjściach je pokazywać."),
        Section::Recording => n("Nagrywanie", "Rozdzielczość filmu, który kamera zapisuje na karcie SD."),
        Section::Setup => n("Konfiguracja", "To, co ustawia się raz dla stanowiska pracy: odbicie lustrzane, sieć elektryczna, monitor."),
        Section::System => n("System", "Język, karta SD, reset ustawień i informacje o kamerze."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Jasność", "Ogólna jasność obrazu. Wartość fabryczna: 50."),
        Contrast => n("Kontrast", "Różnica między jasnym a ciemnym. Wartość fabryczna: 50."),
        Saturation => n("Nasycenie", "Siła koloru: od obrazu czarno-białego do bardzo soczystego. Wartość fabryczna: 50."),
        Sharpness => n("Ostrość", "Podkreślanie konturów. Duże wartości dają jasną obwódkę wokół ścieżek i szum w ciemnych miejscach."),
        ResetPicture => n("Resetuj sekcję", "Przywraca fabryczne wartości jasności, kontrastu, nasycenia i ostrości."),
        AutoExposure => n("Autoekspozycja", "Włączona – kamera sama dobiera czas naświetlania i wzmocnienie do tego, co jest w kadrze. Wyłączona – zostają takie, jakie były w chwili wyłączenia."),
        ExposureBias => n("Kompensacja ekspozycji", "Rozjaśnia lub przyciemnia obraz względem tego, co wybrałaby automatyka. Plus – gdy ciemna płytka wychodzi za ciemna, minus – gdy odblaski na lutach wypalają się do bieli. Działa tylko przy włączonej autoekspozycji."),
        ExposureLock => n("Blokada ekspozycji", "Utrzymuje bieżącą jasność kadru: przestaje ona pływać, gdy w kadr wchodzi lutownica lub pęseta. To samo robi przycisk AE na pilocie. Po wyłączeniu kamery blokada znika."),
        WhiteBalanceLock => n("Balans bieli", "Auto – kamera sama usuwa barwną dominantę światła. Blokada utrzymuje bieżący balans, żeby kolory się nie zmieniały, gdy w kadrze jest dużo jednego koloru – na przykład zielonej soldermaski płytki. To samo robi przycisk AWB na pilocie."),
        Cross => n("Krzyż celowniczy", "Znacznik na środku kadru. To samo robi przycisk + na pilocie."),
        AddLine => n("Dodaj linię", "Prosta przez cały ekran – do wyrównania płytki albo porównania położenia elementów. Linia jest związana z ekranem i nie przesuwa się razem z płytką. Linii może być dowolnie wiele; po wyłączeniu kamery nie są zachowywane."),
        ClearLines => n("Usuń wszystkie linie", "Usuwa wszystkie postawione linie. Krzyż celowniczy zostaje."),
        LevelHdmi => n("Pokaż na HDMI", "Co z tego, co rysuje kamera, widać na monitorze: sam obraz, obraz z krzyżem celowniczym i liniami, do tego wskaźniki stanu albo wszystko razem z menu."),
        LevelUsb => n("Pokaż na USB", "To samo dla obrazu, który idzie do komputera przez USB."),
        LevelMedia => n("W plikach", "Co trafia do filmów i zdjęć na karcie SD. Wartość fabryczna to sam obraz: linie i menu nie są zapisywane w pliku."),
        RecordSize => n("Rozdzielczość filmu", "4K – 3840×2160, pliki są większe. 1080p – 1920×1080, na karcie mieści się więcej."),
        Mirror => n("Odbicie lustrzane", "Odwraca obraz, gdy kamera stoi nad stołem obrócona. Dobierz tak, żeby płytka na ekranie przesuwała się w tę stronę, w którą przesuwa ją ręka. To samo robią przyciski ⇔ i ⇕ na pilocie."),
        PowerLine => n("Częstotliwość sieci", "Częstotliwość sieci elektrycznej, z której zasilane jest oświetlenie: 50 Hz w Europie, 60 Hz w Ameryce Północnej. Przy złym ustawieniu przez obraz przechodzą ciemne pasy."),
        Aspect => n("Format ekranu", "Proporcje monitora. Jeśli coś okrągłego wygląda na ekranie jak owal, wybrano zły format."),
        Hdmi => n("Tryb HDMI", "Rozdzielczość i częstotliwość, z jakimi kamera podaje obraz na monitor. Auto – najlepszy tryb z tych, które zgłosił monitor."),
        Priority => n("Priorytet", "Które wyjście jest ważniejsze. Gdy kamerze nie starcza mocy na oba naraz, to wyjście zachowuje pełną jakość, a w drugim jakość spada."),
        Language => n("Język", "Język menu."),
        Card => n("Karta SD", "Wolne miejsce na karcie, tryb czytnika kart i formatowanie."),
        Reset => n("Reset ustawień", "Przywraca wszystkie ustawienia fabryczne. Plików na karcie SD nie rusza."),
        About => n("O kamerze", "Wersja oprogramowania i bieżące tryby wyjść."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Zdjęcie zapisane · {f}"),
        Msg::Recorded(f, s) => format!("Nagranie zapisane · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Obraz: wartości fabryczne".into(),
        Msg::LinesCleared => "Linie usunięte".into(),
        Msg::Formatted => "Karta SD sformatowana".into(),
        Msg::SettingsReset => "Przywrócono ustawienia fabryczne".into(),
        Msg::NoFiles => "Na karcie nie ma zdjęć ani filmów".into(),
        Msg::Recording => "Galeria niedostępna: trwa nagrywanie".into(),
        Msg::CardOpen(what) => format!("{cannot}: karta jest otwarta na komputerze, wysuń ją tam", cannot = cannot(what)),
        Msg::CardUnreadable(what) => format!("{cannot}: kamera nie odczytuje tej karty, sformatuj ją w menu kamery", cannot = cannot(what)),
        Msg::Card(Card::Host) => "Włączony tryb czytnika kart".into(),
        Msg::Card(Card::Unreadable) => "Kamera nie odczytuje tej karty: sformatuj ją w menu kamery".into(),
        Msg::Card(_) => "Brak karty SD".into(),
        Msg::NoFreeze => "Stop-klatka się nie udała".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "Wyjście HDMI",
                Subject::UsbStream => "Strumień USB",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "Menu na HDMI",
                Subject::Menu(Output::Usb) => "Menu w strumieniu USB",
                Subject::Menu(Output::Media) => "Menu w nagraniu",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("obniż tryb HDMI ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!("zatrzymaj strumień USB na komputerze ({w}×{h} · {fps} {unit})", unit = WORDS.fps),
                    Todo::StopRecording(size) => format!("zatrzymaj nagrywanie ({size})"),
                    Todo::WaitPhoto => "poczekaj na zdjęcie".into(),
                    Todo::StopVideo(file) => format!("zatrzymaj film ({file})"),
                    Todo::Unfreeze => "wyłącz stop-klatkę".into(),
                    Todo::CloseGallery => "zamknij galerię".into(),
                })
                .collect();
            let s = format!("{head}: {why}", why = why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "nie udało się",
            };
            format!("{asked}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Zdjęcie niezapisane",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Nagrywanie przerwane",
                Action::Record => "Nagrywanie się nie rozpoczęło",
                Action::RecordStop => "Nagrywanie się nie zatrzymało",
                Action::Format => "Karta niesformatowana",
                Action::Reset => "Ustawienia nieprzywrócone",
                Action::Play => "Odtwarzanie się nie rozpoczęło",
                Action::PlayStop => "Odtwarzanie się nie zatrzymało",
                Action::Control(name) => control(name),
                Action::Output(_) => "Wyjście się nie włączyło",
            };
            format!("{head}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Nie można zrobić zdjęcia",
        Asked::Record => "Nie można nagrywać",
        Asked::Gallery => "Galeria niedostępna",
        Asked::Format => "Nie można sformatować",
        Asked::Control(name) if name == "card-reader" => "Tryb czytnika kart nie został wyłączony",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Zdjęcie",
        Asked::Record => "Nagrywanie",
        Asked::RecordStop => "Zatrzymanie nagrywania",
        Asked::Format => "Formatowanie",
        Asked::Reset => "Reset ustawień",
        Asked::Play | Asked::Player => "Film",
        Asked::Gallery => "Galeria",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "brak karty SD",
        Code::CardShared => "włączony tryb czytnika kart",
        Code::CardOpen => "karta jest otwarta na komputerze",
        Code::CardUnreadable => "kamera nie odczytuje tej karty",
        Code::Busy => "kamera jest jeszcze zajęta",
        Code::Playing => "trwa odtwarzanie filmu",
        Code::Recording => "trwa nagrywanie",
        Code::Formatting => "karta jest formatowana",
        Code::Unavailable => "teraz niedostępne",
        Code::Refused => "kamera nie mogła tego wykonać",
        Code::Resources => "kamerze brakuje zasobów",
        Code::Unknown | Code::Invalid => "kamera tego nie przyjęła",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Jasność",
        "contrast" => "Kontrast",
        "saturation" => "Nasycenie",
        "sharpness" => "Ostrość",
        "exposure-bias" => "Kompensacja ekspozycji",
        "auto-exposure" => "Autoekspozycja",
        "exposure-lock" => "Blokada ekspozycji",
        "white-balance-lock" => "Blokada balansu bieli",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Odbicie lustrzane",
        "power-line" => "Częstotliwość sieci",
        "aspect" => "Format ekranu",
        "hdmi" => "Tryb HDMI",
        "precedence" => "Priorytet",
        "record-size" => "Rozdzielczość filmu",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Wyświetlanie",
        _ => "Ustawienie",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "autoekspozycja jest wyłączona",
        "white-balance-lock" => "automatyczny balans bieli jest wyłączony",
        "record-size" => "trwa nagrywanie",
        _ => return None,
    })
}
