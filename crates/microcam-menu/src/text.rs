//! The menu's words in each of its languages: a row's name and description, a value's name, a message. One table per
//! kind; a word is a pair, Russian and English.

use microcam_core::controls::{Code, Held, Holder};
use microcam_core::protocol::{Action, Output};

/// The menu's language, kept in its record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Ru, Lang::En];

    /// Its name in the record.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    pub fn of(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }
}

/// A word in both languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct T(pub &'static str, pub &'static str);

impl T {
    pub fn get(self, lang: Lang) -> &'static str {
        match lang {
            Lang::Ru => self.0,
            Lang::En => self.1,
        }
    }
}

pub const MENU: T = T("Меню", "Menu");
pub const GALLERY: T = T("Галерея", "Gallery");
pub const GALLERY_HELP: T = T("Снимки и видео с SD-карты.", "Photos and videos on the SD card.");
pub const OFF: T = T("выкл", "off");
pub const ON: T = T("вкл", "on");
pub const AUTO: T = T("авто", "auto");
pub const LOCKED: T = T("замок", "locked");
pub const CANCEL: T = T("Отмена", "Cancel");
pub const NO_VALUE: T = T("—", "—");
pub const WAITING: T = T("…", "…");

pub const LEVELS: [T; 4] = [T("только картинка", "picture only"), T("с линиями", "with lines"), T("с плашками", "with chips"), T("всё", "everything")];
pub const LEVELS_HELP: [T; 4] = [
    T("Ничего поверх изображения.", "Nothing over the picture."),
    T("Перекрестие и линии.", "The crosshair and the lines."),
    T("Линии и плашки состояния: зум, замки, запись.", "The lines and the state chips: zoom, locks, recording."),
    T("Всё, включая меню.", "Everything, the menu included."),
];
pub const MIRRORS: [T; 4] = [T("нет", "none"), T("по горизонтали", "horizontal"), T("по вертикали", "vertical"), T("оба", "both")];
pub const LANGUAGES: [T; 2] = [T("Русский", "Русский"), T("English", "English")];

pub const LINE_H: (T, T) = (T("Горизонтальная", "Horizontal"), T("Линия слева направо; двигается вверх и вниз.", "A line from left to right; it moves up and down."));
pub const LINE_V: (T, T) = (T("Вертикальная", "Vertical"), T("Линия сверху вниз; двигается влево и вправо.", "A line from top to bottom; it moves left and right."));

pub const CARD_STATE: T = T("Карта", "Card");
pub const CARD_FREE: T = T("Свободно", "Free");
pub const GB: T = T("ГБ", "GB");
pub const CARD_IN: T = T("в камере", "in the camera");
pub const CARD_NONE: T = T("Карты нет", "No card");
pub const CARD_HOST: T = T("у компьютера", "with the computer");
pub const CARD_UNREADABLE: T = T("не читается", "unreadable");
pub const CARD_FORMATTING: T = T("форматируется", "formatting");
pub const CARD_READER: T = T("Режим картридера", "Card reader mode");
pub const CARD_READER_HELP: T = T("Компьютер видит камеру ещё и как картридер с этой картой. Пока режим включён, карта у компьютера.", "The computer sees the camera as a card reader with this card in it, too. While the mode is on, the card is the computer's.");
pub const FORMAT: T = T("Форматировать", "Format");
pub const FORMAT_HELP: T = T("Стирает с карты все снимки и видео и размечает её заново.", "Erases every photo and video on the card and lays it out anew.");
pub const FORMAT_ASK: T = T("Стереть всё с SD-карты?", "Erase everything on the SD card?");
pub const FORMAT_CANCEL_HELP: T = T("Ничего не стирать.", "Erase nothing.");
pub const FORMAT_YES_HELP: T = T("Все снимки и видео на карте будут удалены, вернуть их нельзя.", "Every photo and video on the card will be deleted, with no way back.");
pub const RESET_ASK: T = T("Вернуть заводские настройки?", "Back to the factory settings?");
pub const RESET_CANCEL_HELP: T = T("Оставить настройки как есть.", "Leave the settings as they are.");
pub const RESET_YES: T = T("Сбросить", "Reset");
pub const ABOUT_NAME: &str = "OpenMicroCam";
pub const NO_STREAM: T = T("нет потока", "no stream");
pub const FPS: T = T("к/с", "fps");

pub const ZOOM: T = T("Зум", "Zoom");
pub const MIRROR_IS: T = T("Зеркало", "Mirror");
pub const AE_AUTO: T = T("Экспозиция: авто", "Exposure: auto");
pub const WB_AUTO: T = T("Баланс белого: авто", "White balance: auto");
pub const AE_LOCKED: T = T("Экспозиция: замок", "Exposure: locked");
pub const WB_LOCKED: T = T("Баланс: замок", "Balance: locked");
pub const FROZEN: T = T("Стоп-кадр", "Frozen");
pub const COMPARE: T = T("Слева стоп-кадр · справа камера", "Frozen on the left · the camera on the right");
pub const PAUSED: T = T("Пауза", "Paused");
pub const OF: T = T("из", "of");
pub const SD: T = T("SD", "SD");
pub const NO_SD: T = T("нет SD", "no SD");
pub const SD_HOST: T = T("SD: режим картридера", "SD: card reader mode");
pub const SD_UNREADABLE: T = T("SD: не читается", "SD: unreadable");

// what a key does, in the hints
pub const H_CHOOSE: T = T("выбор", "choose");
pub const H_ENTER: T = T("войти", "enter");
pub const H_BACK: T = T("назад", "back");
pub const H_CLOSE: T = T("закрыть", "close");
pub const H_VALUE: T = T("значение", "value");
pub const H_SETTING: T = T("параметр", "setting");
pub const H_DONE: T = T("готово", "done");
pub const H_MOVE: T = T("двигать", "move");
pub const H_PUT: T = T("поставить", "put");
pub const H_CANCEL: T = T("отмена", "cancel");
pub const H_FILE: T = T("файл", "file");
pub const H_WATCH: T = T("смотреть", "play");
pub const H_FULL: T = T("во весь экран", "full screen");
pub const H_TO_MENU: T = T("в меню", "to the menu");
pub const H_TO_CAMERA: T = T("к камере", "to the camera");
pub const H_TO_STRIP: T = T("к ленте", "to the strip");
pub const H_PAUSE: T = T("пауза", "pause");
pub const H_GO_ON: T = T("дальше", "go on");
pub const H_5S: T = T("5 с", "5 s");

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

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> T {
    match what {
        Asked::Photo => T("Снимок невозможен", "No photo"),
        Asked::Record => T("Запись невозможна", "No recording"),
        Asked::Gallery => T("Галерея недоступна", "No gallery"),
        Asked::Format => T("Форматирование невозможно", "No format"),
        Asked::Control(name) if name == "card-reader" => T("Режим картридера не выключен", "Card reader mode stays on"),
        what => asked(what),
    }
}

/// Why a thing was refused or failed, in words.
fn why(code: Code, lang: Lang) -> &'static str {
    match code {
        Code::NoCard => T("нет SD-карты", "no SD card"),
        Code::CardShared => T("включён режим картридера", "card reader mode is on"),
        Code::CardOpen => T("карта открыта на компьютере", "the card is open on the computer"),
        Code::CardUnreadable => T("камера не читает эту карту", "the camera cannot read this card"),
        Code::Busy => T("камера ещё занята прошлым", "the camera is still busy with the last one"),
        Code::Playing => T("идёт просмотр видео", "a video is playing"),
        Code::Recording => T("идёт запись", "a recording is running"),
        Code::Formatting => T("карта форматируется", "the card is being formatted"),
        Code::Unavailable => T("сейчас недоступно", "not available now"),
        Code::Refused => T("камера не смогла", "the camera could not"),
        Code::Resources => T("камере не хватает ресурсов", "the camera lacks the resources"),
        Code::Unknown | Code::Invalid => T("камера не приняла", "the camera did not take it"),
    }
    .get(lang)
}

/// A control's name in a refusal.
fn control(name: &str) -> T {
    match name {
        "brightness" => T("Яркость", "Brightness"),
        "contrast" => T("Контраст", "Contrast"),
        "saturation" => T("Насыщенность", "Saturation"),
        "sharpness" => T("Резкость", "Sharpness"),
        "exposure-bias" => T("Сдвиг экспозиции", "Exposure shift"),
        "auto-exposure" => T("Автоэкспозиция", "Auto exposure"),
        "exposure-lock" => T("Замок экспозиции", "Exposure lock"),
        "white-balance-lock" => T("Замок баланса", "White balance lock"),
        "zoom" => T("Зум", "Zoom"),
        "mirror" | "flip" => T("Зеркало", "Mirror"),
        "power-line" => T("Частота сети", "Mains frequency"),
        "aspect" => T("Формат экрана", "Screen format"),
        "hdmi" => T("Режим HDMI", "HDMI mode"),
        "precedence" => T("Приоритет", "Priority"),
        "record-size" => T("Качество видео", "Video quality"),
        "card-reader" => CARD_READER,
        "level-hdmi" | "level-usb" | "level-media" => T("Показ", "Display"),
        _ => T("Настройка", "Setting"),
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<T> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => T("автоэкспозиция выключена", "auto exposure is off"),
        "white-balance-lock" => T("автобаланс белого выключен", "auto white balance is off"),
        "record-size" => T("идёт запись", "a recording is running"),
        _ => return None,
    })
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> T {
    match what {
        Asked::Photo => T("Снимок", "Photo"),
        Asked::Record => T("Запись", "Recording"),
        Asked::RecordStop => T("Остановка записи", "Stopping the recording"),
        Asked::Format => T("Форматирование", "Formatting"),
        Asked::Reset => T("Сброс настроек", "Settings reset"),
        Asked::Play | Asked::Player => T("Видео", "Video"),
        Asked::Gallery => T("Галерея", "Gallery"),
        Asked::Control(name) => control(name),
    }
}

/// The resources' refusal, said one way whatever was refused: what could not start, that the camera lacks the
/// resources for it, and on the next line what to turn off or lower — each holder microcamd named that the user can
/// act on, largest first. The menu's drawing beside its pictures is not named: nothing the user does frees it.
fn resources(what: &Wanted, holders: &[Holder], lang: Lang) -> String {
    let head = match what {
        Wanted::Asked(a) => asked(a),
        Wanted::Failed(Action::Photo) => asked(&Asked::Photo),
        Wanted::Failed(Action::Record) => asked(&Asked::Record),
        Wanted::Failed(Action::RecordStop) => asked(&Asked::RecordStop),
        Wanted::Failed(Action::Format) => asked(&Asked::Format),
        Wanted::Failed(Action::Reset) => asked(&Asked::Reset),
        Wanted::Failed(Action::Play | Action::PlayStop) => asked(&Asked::Play),
        Wanted::Failed(Action::Control(name)) => control(name),
        Wanted::Failed(Action::Output(Output::Hdmi)) => T("Выход HDMI", "HDMI output"),
        Wanted::Failed(Action::Output(Output::Usb)) => T("Поток USB", "USB stream"),
        Wanted::Failed(Action::Output(Output::Media)) => asked(&Asked::Record),
        Wanted::Freeze => FROZEN,
        Wanted::Gallery => GALLERY,
        Wanted::Menu(Output::Hdmi) => T("Меню на HDMI", "The menu on HDMI"),
        Wanted::Menu(Output::Usb) => T("Меню в потоке USB", "The menu in the USB stream"),
        Wanted::Menu(Output::Media) => T("Меню в записи", "The menu in the recording"),
    };
    let w = |t: T| t.get(lang);
    let mut todo: Vec<String> = Vec::new();
    for h in holders {
        let one = match &h.what {
            Held::Hdmi { timing } => format!("{} ({})", w(T("понизьте режим HDMI", "lower the HDMI mode")), timing.name()),
            Held::Usb { w: x, h: y, fps, .. } => format!("{} ({x}×{y} · {fps} {})", w(T("остановите поток USB на компьютере", "stop the USB stream on the computer")), w(FPS)),
            Held::Recording { h: y, .. } => {
                let size = if *y >= 2160 { "4K".to_string() } else { format!("{y}p") };
                format!("{} ({size})", w(T("остановите запись", "stop the recording")))
            }
            Held::Photo { .. } => w(T("дождитесь снимка", "wait for the photo")).to_string(),
            Held::Player { file } => format!("{} ({})", w(T("остановите видео", "stop the video")), file.rsplit('/').next().unwrap_or(file)),
            Held::Picture { id } if crate::core::is_frozen(id) => w(T("снимите стоп-кадр", "unfreeze the picture")).to_string(),
            Held::Picture { .. } => w(T("закройте галерею", "close the gallery")).to_string(),
            Held::Drawing => continue,
        };
        if !todo.contains(&one) {
            todo.push(one);
        }
    }
    let mut s = format!("{}: {}", w(head), why(Code::Resources, lang));
    if let Some(first) = todo.first_mut() {
        let mut c = first.chars();
        *first = c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default();
        s += "\n";
        s += &todo.join(", ");
    }
    s
}

pub fn clock(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

impl Msg {
    pub fn say(&self, lang: Lang) -> String {
        let t = |t: T| t.get(lang).to_string();
        match self {
            Msg::PhotoSaved(f) => format!("{} · {f}", T("Снимок сохранён", "Photo saved").get(lang)),
            Msg::Recorded(f, s) => format!("{} · {f} · {}", T("Запись сохранена", "Recording saved").get(lang), clock(*s)),
            Msg::PictureReset => t(T("Картинка: заводские значения", "Picture: factory values")),
            Msg::LinesCleared => t(T("Линии убраны", "Lines removed")),
            Msg::Formatted => t(T("SD-карта отформатирована", "SD card formatted")),
            Msg::SettingsReset => t(T("Настройки сброшены", "Settings reset")),
            Msg::NoFiles => t(T("На карте нет снимков и видео", "No photos or videos on the card")),
            Msg::Recording => t(T("Идёт запись", "Recording")),
            Msg::CardOpen(what) => {
                format!("{}: {}", cannot(what).get(lang), T("карта открыта на компьютере, извлеките её там", "the card is open on the computer, eject it there").get(lang))
            }
            Msg::CardUnreadable(what) => {
                format!("{}: {}", cannot(what).get(lang), T("камера не читает эту карту, отформатируйте её в меню камеры", "the camera cannot read this card, format it in the camera's menu").get(lang))
            }
            Msg::Card(microcam_core::protocol::Card::Host) => t(T("Включён режим картридера", "Card reader mode is on")),
            Msg::Card(microcam_core::protocol::Card::Unreadable) => t(T("Камера не читает эту карту: отформатируйте её в меню камеры", "The camera cannot read this card: format it in the camera's menu")),
            Msg::Card(_) => t(T("Нет SD-карты", "No SD card")),
            Msg::NoFreeze => t(T("Стоп-кадр не получился", "Could not freeze the picture")),
            Msg::Resources(what, holders) => resources(what, holders, lang),
            Msg::Refused(what, code) => {
                let head = asked(what);
                let reason = match (what, code) {
                    (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).map_or(why(Code::Unavailable, lang), |t| t.get(lang)),
                    (_, Some(c)) => why(*c, lang),
                    (_, None) => T("не получилось", "did not work").get(lang),
                };
                format!("{}: {reason}", head.get(lang))
            }
            Msg::Failed(what, code) => {
                let head = match what {
                    Action::Photo => T("Снимок не сохранён", "Photo not saved"),
                    // the card taken out ends a recording; anything else stops it at its start
                    Action::Record if *code == Code::NoCard => T("Запись прервана", "Recording broken off"),
                    Action::Record => T("Запись не запустилась", "The recording did not start"),
                    Action::RecordStop => T("Запись не остановилась", "The recording did not stop"),
                    Action::Format => T("Карта не отформатирована", "Card not formatted"),
                    Action::Reset => T("Настройки не сброшены", "Settings not reset"),
                    Action::Play => T("Видео не запустилось", "The video did not start"),
                    Action::PlayStop => T("Видео не остановилось", "The video did not stop"),
                    Action::Control(name) => control(name),
                    Action::Output(_) => T("Выход не показан", "An output left out"),
                };
                format!("{}: {}", head.get(lang), why(*code, lang))
            }
        }
    }
}
