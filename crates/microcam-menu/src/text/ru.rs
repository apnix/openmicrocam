//! Русский.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "ru",
    name: "Русский",
    decimal: ',',

    menu: "Меню",
    gallery: n("Галерея", "Снимки и видео с SD-карты."),
    off: "выкл",
    on: "вкл",
    auto: "авто",
    locked: "фиксация",
    cancel: "Отмена",
    levels: [
        n("только картинка", "Ничего поверх изображения."),
        n("с линиями", "Перекрестие и линии."),
        n("с индикаторами", "Линии и индикаторы состояния: зум, фиксация, запись."),
        n("всё", "Всё, включая меню."),
    ],
    mirrors: ["нет", "по горизонтали", "по вертикали", "по обеим осям"],
    line_h: n("Горизонтальная", "Линия слева направо; двигается вверх и вниз."),
    line_v: n("Вертикальная", "Линия сверху вниз; двигается влево и вправо."),

    card_state: "Карта",
    card_free: "Свободно",
    gb: "ГБ",
    card_in: "в камере",
    card_none: "Карты нет",
    card_host: "у компьютера",
    card_unreadable: "не читается",
    card_formatting: "форматируется",
    card_reader: n("Режим картридера", "Компьютер видит камеру ещё и как картридер с этой картой. Пока режим включён, карта у компьютера."),
    format: n("Форматировать", "Стирает с карты все снимки и видео и размечает её заново."),
    format_ask: "Стереть всё с SD-карты?",
    format_cancel_help: "Ничего не стирать.",
    format_yes_help: "Все снимки и видео на карте будут удалены, вернуть их нельзя.",
    reset_ask: "Вернуть заводские настройки?",
    reset_cancel_help: "Оставить настройки как есть.",
    reset_yes: "Сбросить",
    no_stream: "нет потока",
    fps: "к/с",
    hz: "Гц",

    mirror,
    ae_auto: "Экспозиция: авто",
    wb_auto: "Баланс белого: авто",
    ae_locked: "Экспозиция: фиксация",
    wb_locked: "Баланс белого: фиксация",
    frozen: "Стоп-кадр",
    compare: "Слева стоп-кадр · справа камера",
    paused: "Пауза",
    sd: "SD",
    no_sd: "нет SD",
    sd_host: "SD: режим картридера",
    sd_unreadable: "SD: не читается",
    of,

    keys: Keys {
        choose: "выбор",
        enter: "ОК",
        back: "назад",
        close: "закрыть",
        value: "значение",
        setting: "параметр",
        done: "готово",
        shift: "двигать",
        put: "поставить",
        cancel: "отмена",
        file: "файл",
        watch: "смотреть",
        full: "во весь экран",
        to_menu: "в меню",
        to_camera: "к камере",
        to_strip: "к ленте",
        pause: "пауза",
        go_on: "продолжить",
        seek: "±5 с",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} из {whole}")
}

fn mirror(state: &str) -> String {
    format!("Зеркало: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Картинка", "Яркость, контраст, насыщенность и резкость изображения."),
        Section::Exposure => n("Экспозиция", "Автоматика яркости кадра и баланс белого: доверить камере, сдвинуть или зафиксировать."),
        Section::Overlays => n("Наложения", "Перекрестие и линии поверх картинки; на каких выходах их показывать."),
        Section::Recording => n("Запись", "Разрешение видео, которое камера пишет на SD-карту."),
        Section::Setup => n("Установка", "То, что настраивают один раз под рабочее место: зеркало, электросеть, монитор."),
        Section::System => n("Система", "Язык, SD-карта, сброс настроек и сведения о камере."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Яркость", "Общая яркость изображения. Заводское значение — 50."),
        Contrast => n("Контраст", "Разница между светлым и тёмным. Заводское значение — 50."),
        Saturation => n("Насыщенность", "Сила цвета: от чёрно-белой картинки до очень сочной. Заводское значение — 50."),
        Sharpness => n("Резкость", "Подчёркивание контуров. Большие значения дают светлую кайму вокруг дорожек и шум на тёмном."),
        ResetPicture => n("Сбросить раздел", "Вернуть яркость, контраст, насыщенность и резкость к заводским значениям."),
        AutoExposure => n("Автоэкспозиция", "Вкл — камера сама подбирает выдержку и усиление под то, что в кадре. Выкл — они остаются такими, какими были в момент выключения."),
        ExposureBias => n("Коррекция экспозиции", "Делает картинку светлее или темнее, чем выбрала бы автоматика. Плюс — когда тёмная плата выходит слишком тёмной, минус — когда блики на припое выгорают в белое. Работает только при включённой автоэкспозиции."),
        ExposureLock => n("Фиксация экспозиции", "Фиксирует текущую яркость кадра: она перестаёт плавать, когда в кадр входит паяльник или пинцет. То же делает кнопка AE на пульте. После выключения камеры фиксация снимается."),
        WhiteBalanceLock => n("Баланс белого", "Авто — камера сама убирает цветной оттенок света. Фиксация сохраняет текущий баланс, чтобы цвета не уплывали, когда в кадре много одного цвета — например, зелёной маски платы. То же делает кнопка AWB на пульте."),
        Cross => n("Перекрестие", "Метка в центре кадра. То же делает кнопка + на пульте."),
        AddLine => n("Добавить линию", "Прямая через весь экран — выровнять плату или сверить положение деталей. Линия привязана к экрану и за платой не двигается. Линий может быть сколько угодно; после выключения камеры они не сохраняются."),
        ClearLines => n("Убрать все линии", "Убирает все поставленные линии. Перекрестие остаётся."),
        LevelHdmi => n("Показ на HDMI", "Что из нарисованного камерой видно на мониторе: только картинка, картинка с перекрестием и линиями, ещё и индикаторы состояния, или всё вместе с меню."),
        LevelUsb => n("Показ на USB", "То же для картинки, которая идёт в компьютер по USB."),
        LevelMedia => n("В файлах", "Что попадает в видео и снимки на SD-карте. Заводское значение — только картинка: линии и меню в файл не пишутся."),
        RecordSize => n("Разрешение видео", "4K — 3840×2160, файлы крупнее. 1080p — 1920×1080, на карту помещается больше."),
        Mirror => n("Зеркало", "Переворачивает картинку, если камера стоит над столом повёрнутой. Подберите так, чтобы плата на экране двигалась туда же, куда её двигает рука. То же делают кнопки ⇔ и ⇕ на пульте."),
        PowerLine => n("Частота сети", "Частота электросети, от которой питается освещение: 50 Гц в Европе и России, 60 Гц в Америке. При неверной по картинке идут тёмные полосы."),
        Aspect => n("Формат экрана", "Соотношение сторон монитора. Если круглое на экране выглядит овалом — выбран не тот формат."),
        Hdmi => n("Режим HDMI", "Разрешение и частота, с которыми камера выдаёт картинку на монитор. Авто — лучшее из того, что монитор сообщил о себе."),
        Priority => n("Приоритет", "Какой выход важнее. Когда камере не хватает сил на оба сразу, этот остаётся в полном качестве, а у второго качество снижается."),
        Language => n("Язык", "Язык меню."),
        Card => n("SD-карта", "Свободное место на карте, режим картридера и форматирование."),
        Reset => n("Сброс настроек", "Возвращает все настройки к заводским. Файлы на SD-карте не трогает."),
        About => n("О камере", "Версия прошивки и текущие режимы выходов."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Снимок сохранён · {f}"),
        Msg::Recorded(f, s) => format!("Запись сохранена · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Картинка: заводские значения".into(),
        Msg::LinesCleared => "Линии убраны".into(),
        Msg::Formatted => "SD-карта отформатирована".into(),
        Msg::SettingsReset => "Настройки сброшены".into(),
        Msg::NoFiles => "На карте нет снимков и видео".into(),
        Msg::Recording => "Галерея недоступна: идёт запись".into(),
        Msg::CardOpen(what) => format!("{cannot}: карта открыта на компьютере, извлеките её там", cannot = cannot(what)),
        Msg::CardUnreadable(what) => format!("{cannot}: камера не читает эту карту, отформатируйте её в меню камеры", cannot = cannot(what)),
        Msg::Card(Card::Host) => "Включён режим картридера".into(),
        Msg::Card(Card::Unreadable) => "Камера не читает эту карту: отформатируйте её в меню камеры".into(),
        Msg::Card(_) => "Нет SD-карты".into(),
        Msg::NoFreeze => "Стоп-кадр не получился".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "Выход HDMI",
                Subject::UsbStream => "Поток USB",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "Меню на HDMI",
                Subject::Menu(Output::Usb) => "Меню в потоке USB",
                Subject::Menu(Output::Media) => "Меню в записи",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("понизьте режим HDMI ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!("остановите поток USB на компьютере ({w}×{h} · {fps} {unit})", unit = WORDS.fps),
                    Todo::StopRecording(size) => format!("остановите запись ({size})"),
                    Todo::WaitPhoto => "дождитесь снимка".into(),
                    Todo::StopVideo(file) => format!("остановите видео ({file})"),
                    Todo::Unfreeze => "снимите стоп-кадр".into(),
                    Todo::CloseGallery => "закройте галерею".into(),
                })
                .collect();
            let s = format!("{head}: {why}", why = why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "не получилось",
            };
            format!("{asked}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Снимок не сохранён",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Запись прервана",
                Action::Record => "Запись не запустилась",
                Action::RecordStop => "Запись не остановилась",
                Action::Format => "Карта не отформатирована",
                Action::Reset => "Настройки не сброшены",
                Action::Play => "Видео не запустилось",
                Action::PlayStop => "Видео не остановилось",
                Action::Control(name) => control(name),
                Action::Output(_) => "Выход не включился",
            };
            format!("{head}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Снимок невозможен",
        Asked::Record => "Запись невозможна",
        Asked::Gallery => "Галерея недоступна",
        Asked::Format => "Форматирование невозможно",
        Asked::Control(name) if name == "card-reader" => "Режим картридера не выключен",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Снимок",
        Asked::Record => "Запись",
        Asked::RecordStop => "Остановка записи",
        Asked::Format => "Форматирование",
        Asked::Reset => "Сброс настроек",
        Asked::Play | Asked::Player => "Видео",
        Asked::Gallery => "Галерея",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "нет SD-карты",
        Code::CardShared => "включён режим картридера",
        Code::CardOpen => "карта открыта на компьютере",
        Code::CardUnreadable => "камера не читает эту карту",
        Code::Busy => "камера ещё занята",
        Code::Playing => "идёт просмотр видео",
        Code::Recording => "идёт запись",
        Code::Formatting => "карта форматируется",
        Code::Unavailable => "сейчас недоступно",
        Code::Refused => "камера не смогла",
        Code::Resources => "камере не хватает ресурсов",
        Code::Unknown | Code::Invalid => "камера не приняла",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Яркость",
        "contrast" => "Контраст",
        "saturation" => "Насыщенность",
        "sharpness" => "Резкость",
        "exposure-bias" => "Коррекция экспозиции",
        "auto-exposure" => "Автоэкспозиция",
        "exposure-lock" => "Фиксация экспозиции",
        "white-balance-lock" => "Фиксация баланса белого",
        "zoom" => "Зум",
        "mirror" | "flip" => "Зеркало",
        "power-line" => "Частота сети",
        "aspect" => "Формат экрана",
        "hdmi" => "Режим HDMI",
        "precedence" => "Приоритет",
        "record-size" => "Разрешение видео",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Показ",
        _ => "Настройка",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "автоэкспозиция выключена",
        "white-balance-lock" => "автобаланс белого выключен",
        "record-size" => "идёт запись",
        _ => return None,
    })
}
