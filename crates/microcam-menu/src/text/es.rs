//! Español.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "es",
    name: "Español",
    decimal: ',',

    menu: "Menú",
    gallery: n("Galería", "Fotos y vídeos de la tarjeta SD."),
    off: "desactivado",
    on: "activado",
    auto: "auto",
    locked: "bloqueado",
    cancel: "Cancelar",
    levels: [
        n("solo imagen", "Nada sobre la imagen."),
        n("con líneas", "La retícula y las líneas."),
        n("con indicadores", "Las líneas y los indicadores de estado: zoom, bloqueos, grabación."),
        n("todo", "Todo, incluido el menú."),
    ],
    mirrors: ["ninguno", "horizontal", "vertical", "ambos ejes"],
    line_h: n("Horizontal", "Una línea de izquierda a derecha; se mueve hacia arriba y hacia abajo."),
    line_v: n("Vertical", "Una línea de arriba abajo; se mueve hacia la izquierda y hacia la derecha."),

    card_state: "Tarjeta",
    card_free: "Libre",
    gb: "GB",
    card_in: "en la cámara",
    card_none: "Sin tarjeta",
    card_host: "en uso por el ordenador",
    card_unreadable: "ilegible",
    card_formatting: "formateando",
    card_reader: n("Modo lector de tarjetas", "El ordenador ve la cámara también como un lector con esta tarjeta. Mientras el modo está activado, solo el ordenador puede acceder a la tarjeta."),
    format: n("Formatear", "Borra todas las fotos y vídeos de la tarjeta y la prepara de nuevo."),
    format_ask: "¿Borrar todo de la tarjeta SD?",
    format_cancel_help: "No borrar nada.",
    format_yes_help: "Se borrarán todas las fotos y vídeos de la tarjeta y no se podrán recuperar.",
    reset_ask: "¿Restablecer los ajustes de fábrica?",
    reset_cancel_help: "Dejar los ajustes como están.",
    reset_yes: "Restablecer",
    no_stream: "sin transmisión",
    fps: "fps",
    hz: "Hz",

    mirror,
    ae_auto: "Exposición: auto",
    wb_auto: "Balance de blancos: auto",
    ae_locked: "Exposición: bloqueada",
    wb_locked: "Balance de blancos: bloqueado",
    frozen: "Imagen congelada",
    compare: "A la izquierda, imagen congelada · a la derecha, cámara",
    paused: "Pausa",
    sd: "SD",
    no_sd: "sin SD",
    sd_host: "SD: modo lector de tarjetas",
    sd_unreadable: "SD: ilegible",
    of,

    keys: Keys {
        choose: "selección",
        enter: "OK",
        back: "atrás",
        close: "cerrar",
        value: "valor",
        setting: "ajuste",
        done: "listo",
        shift: "mover",
        put: "colocar",
        cancel: "cancelar",
        file: "archivo",
        watch: "reproducir",
        full: "pantalla completa",
        to_menu: "al menú",
        to_camera: "a la cámara",
        to_strip: "a las miniaturas",
        pause: "pausa",
        go_on: "reanudar",
        seek: "±5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} de {whole}")
}

fn mirror(state: &str) -> String {
    format!("Espejo: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Imagen", "Brillo, contraste, saturación y nitidez de la imagen."),
        Section::Exposure => n("Exposición", "Ajuste automático del brillo y balance de blancos: dejarlos a la cámara, corregirlos o bloquearlos."),
        Section::Overlays => n("Superposiciones", "La retícula y las líneas sobre la imagen; en qué salidas mostrarlas."),
        Section::Recording => n("Grabación", "Resolución del vídeo que la cámara graba en la tarjeta SD."),
        Section::Setup => n("Instalación", "Ajustes que se hacen una sola vez para el puesto de trabajo: espejo, frecuencia de red, monitor."),
        Section::System => n("Sistema", "Idioma, tarjeta SD, restablecimiento de ajustes e información de la cámara."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Brillo", "Brillo general de la imagen. Valor de fábrica: 50."),
        Contrast => n("Contraste", "Diferencia entre lo claro y lo oscuro. Valor de fábrica: 50."),
        Saturation => n("Saturación", "Intensidad del color: desde una imagen en blanco y negro hasta colores muy vivos. Valor de fábrica: 50."),
        Sharpness => n("Nitidez", "Realce de los contornos. Los valores altos producen un halo claro alrededor de las pistas y ruido en las zonas oscuras."),
        ResetPicture => n("Restablecer sección", "Devuelve el brillo, el contraste, la saturación y la nitidez a sus valores de fábrica."),
        AutoExposure => n("Exposición automática", "Activado: la cámara elige por sí misma el tiempo de exposición y la ganancia según lo que hay en la imagen. Desactivado: se mantienen como estaban al desactivarla."),
        ExposureBias => n("Compensación de exposición", "Hace la imagen más clara o más oscura de lo que elegiría el ajuste automático. Más, cuando una placa oscura sale demasiado oscura; menos, cuando los reflejos de la soldadura salen quemados. Solo funciona con la exposición automática activada."),
        ExposureLock => n("Bloqueo de exposición", "Fija el brillo actual de la imagen: deja de variar cuando un soldador o unas pinzas entran en la imagen. El botón AE del mando a distancia hace lo mismo. Al apagar la cámara, el bloqueo se desactiva."),
        WhiteBalanceLock => n("Balance de blancos", "Auto: la cámara corrige por sí misma la dominante de color de la luz. Bloqueado: se mantiene el balance actual para que los colores no deriven cuando un solo color domina la imagen, por ejemplo la máscara de soldadura verde de la placa. El botón AWB del mando a distancia hace lo mismo."),
        Cross => n("Retícula", "Una marca en el centro de la imagen. El botón + del mando a distancia hace lo mismo."),
        AddLine => n("Añadir línea", "Una recta que cruza toda la pantalla, para alinear la placa o comparar la posición de los componentes. La línea está fijada a la pantalla y no se mueve con la placa. Puede haber tantas líneas como se quiera; no se conservan al apagar la cámara."),
        ClearLines => n("Eliminar todas las líneas", "Elimina todas las líneas colocadas. La retícula se mantiene."),
        LevelHdmi => n("Visualización HDMI", "Qué superposiciones de la cámara se ven en el monitor: solo la imagen, la imagen con la retícula y las líneas, también con los indicadores de estado, o todo junto con el menú."),
        LevelUsb => n("Visualización USB", "Lo mismo para la imagen que va al ordenador por USB."),
        LevelMedia => n("En los archivos", "Lo que entra en los vídeos y las fotos de la tarjeta SD. El valor de fábrica es solo la imagen: las líneas y el menú no se graban en el archivo."),
        RecordSize => n("Resolución de vídeo", "4K: 3840×2160, archivos más grandes. 1080p: 1920×1080, cabe más en la tarjeta."),
        Mirror => n("Espejo", "Invierte la imagen cuando la cámara está montada girada sobre la mesa. Elija el ajuste con el que la placa se mueve en pantalla en la misma dirección que su mano. Los botones ⇔ y ⇕ del mando a distancia hacen lo mismo."),
        PowerLine => n("Frecuencia de red", "Frecuencia de la red eléctrica que alimenta la iluminación: 50 Hz en Europa, 60 Hz en Norteamérica. Con el ajuste incorrecto, unas bandas oscuras recorren la imagen."),
        Aspect => n("Formato de pantalla", "Relación de aspecto del monitor. Si algo redondo se ve ovalado en la pantalla, el formato elegido no es el correcto."),
        Hdmi => n("Modo HDMI", "Resolución y frecuencia con las que la cámara envía la imagen al monitor. Auto: el mejor modo que admite el monitor."),
        Priority => n("Prioridad", "Qué salida es más importante. Cuando la cámara no puede atender las dos a la vez, esta conserva toda su calidad y la de la otra se reduce."),
        Language => n("Idioma", "Idioma del menú."),
        Card => n("Tarjeta SD", "Espacio libre en la tarjeta, modo lector de tarjetas y formateo."),
        Reset => n("Restablecer ajustes", "Devuelve todos los ajustes a los valores de fábrica. No toca los archivos de la tarjeta SD."),
        About => n("Acerca de la cámara", "Versión del firmware y modos actuales de las salidas."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Foto guardada · {f}"),
        Msg::Recorded(f, s) => format!("Grabación guardada · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Imagen: valores de fábrica".into(),
        Msg::LinesCleared => "Líneas eliminadas".into(),
        Msg::Formatted => "Tarjeta SD formateada".into(),
        Msg::SettingsReset => "Ajustes restablecidos".into(),
        Msg::NoFiles => "No hay fotos ni vídeos en la tarjeta".into(),
        Msg::Recording => "Galería no disponible: grabación en curso".into(),
        Msg::CardOpen(what) => format!("{cannot}: la tarjeta está abierta en el ordenador, expúlsela desde allí", cannot = cannot(what)),
        Msg::CardUnreadable(what) => format!("{cannot}: la cámara no puede leer esta tarjeta, formatéela en el menú de la cámara", cannot = cannot(what)),
        Msg::Card(Card::Host) => "El modo lector de tarjetas está activado".into(),
        Msg::Card(Card::Unreadable) => "La cámara no puede leer esta tarjeta: formatéela en el menú de la cámara".into(),
        Msg::Card(_) => "No hay tarjeta SD".into(),
        Msg::NoFreeze => "No se pudo congelar la imagen".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "Salida HDMI",
                Subject::UsbStream => "Transmisión USB",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "Menú en HDMI",
                Subject::Menu(Output::Usb) => "Menú en la transmisión USB",
                Subject::Menu(Output::Media) => "Menú en la grabación",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("baje el modo HDMI ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!("detenga la transmisión USB en el ordenador ({w}×{h} · {fps} {unit})", unit = WORDS.fps),
                    Todo::StopRecording(size) => format!("detenga la grabación ({size})"),
                    Todo::WaitPhoto => "espere a que termine la foto".into(),
                    Todo::StopVideo(file) => format!("detenga el vídeo ({file})"),
                    Todo::Unfreeze => "desactive la imagen congelada".into(),
                    Todo::CloseGallery => "cierre la galería".into(),
                })
                .collect();
            let s = format!("{head}: {why}", why = why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "no se pudo",
            };
            format!("{asked}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Foto no guardada",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Grabación interrumpida",
                Action::Record => "La grabación no se inició",
                Action::RecordStop => "La grabación no se detuvo",
                Action::Format => "Tarjeta no formateada",
                Action::Reset => "Ajustes no restablecidos",
                Action::Play => "El vídeo no se inició",
                Action::PlayStop => "El vídeo no se detuvo",
                Action::Control(name) => control(name),
                Action::Output(_) => "La salida no se inició",
            };
            format!("{head}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "No se puede hacer la foto",
        Asked::Record => "No se puede grabar",
        Asked::Gallery => "Galería no disponible",
        Asked::Format => "No se puede formatear",
        Asked::Control(name) if name == "card-reader" => "El modo lector de tarjetas sigue activado",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Foto",
        Asked::Record => "Grabación",
        Asked::RecordStop => "Detener la grabación",
        Asked::Format => "Formateo",
        Asked::Reset => "Restablecer ajustes",
        Asked::Play | Asked::Player => "Vídeo",
        Asked::Gallery => "Galería",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "no hay tarjeta SD",
        Code::CardShared => "el modo lector de tarjetas está activado",
        Code::CardOpen => "la tarjeta está abierta en el ordenador",
        Code::CardUnreadable => "la cámara no puede leer esta tarjeta",
        Code::Busy => "la cámara aún está ocupada",
        Code::Playing => "se está reproduciendo un vídeo",
        Code::Recording => "grabación en curso",
        Code::Formatting => "la tarjeta se está formateando",
        Code::Unavailable => "no disponible en este momento",
        Code::Refused => "la cámara no pudo hacerlo",
        Code::Resources => "a la cámara le faltan recursos",
        Code::Unknown | Code::Invalid => "la cámara no lo aceptó",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Brillo",
        "contrast" => "Contraste",
        "saturation" => "Saturación",
        "sharpness" => "Nitidez",
        "exposure-bias" => "Compensación de exposición",
        "auto-exposure" => "Exposición automática",
        "exposure-lock" => "Bloqueo de exposición",
        "white-balance-lock" => "Bloqueo del balance de blancos",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Espejo",
        "power-line" => "Frecuencia de red",
        "aspect" => "Formato de pantalla",
        "hdmi" => "Modo HDMI",
        "precedence" => "Prioridad",
        "record-size" => "Resolución de vídeo",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Visualización",
        _ => "Ajuste",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "la exposición automática está desactivada",
        "white-balance-lock" => "el balance de blancos automático está desactivado",
        "record-size" => "grabación en curso",
        _ => return None,
    })
}
