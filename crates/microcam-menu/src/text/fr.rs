//! Français.

use super::{capital, clock, todos, Asked, Keys, Msg, Named, Subject, Todo, Words};
use crate::menu::{Item, Section};
use microcam_core::controls::Code;
use microcam_core::protocol::{Action, Card, Output};

const fn n(name: &'static str, help: &'static str) -> Named {
    Named { name, help }
}

pub const WORDS: Words = Words {
    code: "fr",
    name: "Français",
    decimal: ',',

    menu: "Menu",
    gallery: n("Galerie", "Photos et vidéos de la carte SD."),
    off: "désactivé",
    on: "activé",
    auto: "auto",
    locked: "verrouillée",
    cancel: "Annuler",
    levels: [
        n("image seule", "Rien par-dessus l'image."),
        n("avec lignes", "Le réticule et les lignes."),
        n("avec indicateurs", "Les lignes et les indicateurs d'état\u{a0}: zoom, verrouillages, enregistrement."),
        n("tout", "Tout, y compris le menu."),
    ],
    mirrors: ["aucun", "horizontal", "vertical", "les deux axes"],
    line_h: n("Horizontale", "Une ligne de gauche à droite\u{a0}; elle se déplace vers le haut et vers le bas."),
    line_v: n("Verticale", "Une ligne de haut en bas\u{a0}; elle se déplace vers la gauche et vers la droite."),

    card_state: "Carte",
    card_free: "Libre",
    gb: "Go",
    card_in: "dans la caméra",
    card_none: "Pas de carte",
    card_host: "côté ordinateur",
    card_unreadable: "illisible",
    card_formatting: "formatage en cours",
    card_reader: n("Mode lecteur de carte", "L'ordinateur voit aussi la caméra comme un lecteur contenant cette carte. Tant que ce mode est activé, la carte est à l'ordinateur."),
    format: n("Formater", "Efface toutes les photos et vidéos de la carte et la prépare à nouveau."),
    format_ask: "Tout effacer de la carte SD\u{a0}?",
    format_cancel_help: "Ne rien effacer.",
    format_yes_help: "Toutes les photos et vidéos de la carte seront effacées définitivement.",
    reset_ask: "Rétablir les réglages d'usine\u{a0}?",
    reset_cancel_help: "Garder les réglages tels quels.",
    reset_yes: "Réinitialiser",
    no_stream: "pas de flux",
    fps: "ips",
    hz: "Hz",

    mirror,
    ae_auto: "Exposition\u{a0}: auto",
    wb_auto: "Balance des blancs\u{a0}: auto",
    ae_locked: "Exposition\u{a0}: verrouillée",
    wb_locked: "Balance des blancs\u{a0}: verrouillée",
    frozen: "Arrêt sur image",
    compare: "À gauche, arrêt sur image · à droite, caméra",
    paused: "Pause",
    sd: "SD",
    no_sd: "pas de SD",
    sd_host: "SD\u{a0}: mode lecteur de carte",
    sd_unreadable: "SD\u{a0}: illisible",
    of,

    keys: Keys {
        choose: "sélection",
        enter: "OK",
        back: "retour",
        close: "fermer",
        value: "valeur",
        setting: "réglage",
        done: "terminé",
        shift: "déplacer",
        put: "placer",
        cancel: "annuler",
        file: "fichier",
        watch: "lire",
        full: "plein écran",
        to_menu: "au menu",
        to_camera: "à la caméra",
        to_strip: "à la bande",
        pause: "pause",
        go_on: "reprendre",
        seek: "±5 s",
    },
    section,
    item,
    say,
};

fn of(part: &str, whole: &str) -> String {
    format!("{part} sur {whole}")
}

fn mirror(state: &str) -> String {
    format!("Miroir\u{a0}: {state}")
}

fn section(s: Section) -> Named {
    match s {
        Section::Picture => n("Image", "Luminosité, contraste, saturation et netteté de l'image."),
        Section::Exposure => n("Exposition", "Réglage automatique de la luminosité et balance des blancs\u{a0}: laisser faire la caméra, corriger ou verrouiller."),
        Section::Overlays => n("Incrustations", "Le réticule et les lignes par-dessus l'image\u{a0}; sur quelles sorties les afficher."),
        Section::Recording => n("Enregistrement", "Résolution de la vidéo que la caméra écrit sur la carte SD."),
        Section::Setup => n("Installation", "Ce qu'on règle une fois pour le poste de travail\u{a0}: miroir, secteur, moniteur."),
        Section::System => n("Système", "Langue, carte SD, réinitialisation des réglages et informations sur la caméra."),
    }
}

fn item(i: Item) -> Named {
    use Item::*;
    match i {
        Brightness => n("Luminosité", "Luminosité générale de l'image. Valeur d'usine\u{a0}: 50."),
        Contrast => n("Contraste", "Écart entre le clair et le sombre. Valeur d'usine\u{a0}: 50."),
        Saturation => n("Saturation", "Intensité des couleurs\u{a0}: d'une image en noir et blanc à des couleurs très vives. Valeur d'usine\u{a0}: 50."),
        Sharpness => n("Netteté", "Accentuation des contours. Les valeurs élevées donnent un liseré clair autour des pistes et du bruit dans les zones sombres."),
        ResetPicture => n("Réinitialiser la section", "Remet la luminosité, le contraste, la saturation et la netteté à leurs valeurs d'usine."),
        AutoExposure => n("Exposition auto", "Activé\u{a0}: la caméra choisit elle-même le temps de pose et le gain selon ce qui est dans l'image. Désactivé\u{a0}: ils restent tels qu'ils étaient au moment de la désactivation."),
        ExposureBias => n("Correction d'exposition", "Éclaircit ou assombrit l'image par rapport au réglage automatique. Plus, quand un circuit sombre paraît trop sombre\u{a0}; moins, quand les reflets sur les soudures saturent en blanc. Ne fonctionne que si l'exposition auto est activée."),
        ExposureLock => n("Verrouillage de l'exposition", "Fige la luminosité actuelle de l'image\u{a0}: elle ne varie plus quand un fer à souder ou une pince entre dans le champ. La touche AE de la télécommande fait la même chose. Le verrouillage est levé à l'extinction de la caméra."),
        WhiteBalanceLock => n("Balance des blancs", "Auto\u{a0}: la caméra corrige elle-même la dominante de couleur de l'éclairage. Verrouillée\u{a0}: la balance actuelle est conservée, pour que les couleurs ne dérivent pas quand une couleur domine l'image, par exemple le vernis épargne vert d'un circuit imprimé. La touche AWB de la télécommande fait la même chose."),
        Cross => n("Réticule", "Un repère au centre de l'image. La touche + de la télécommande fait la même chose."),
        AddLine => n("Ajouter une ligne", "Une droite sur tout l'écran, pour aligner le circuit ou comparer la position des composants. La ligne est liée à l'écran et ne suit pas le circuit. On peut placer autant de lignes qu'on veut\u{a0}; elles ne sont pas conservées à l'extinction de la caméra."),
        ClearLines => n("Supprimer toutes les lignes", "Supprime toutes les lignes placées. Le réticule reste."),
        LevelHdmi => n("Affichage HDMI", "Ce qui, parmi les incrustations de la caméra, apparaît sur le moniteur\u{a0}: l'image seule, l'image avec le réticule et les lignes, les indicateurs d'état en plus, ou tout avec le menu."),
        LevelUsb => n("Affichage USB", "La même chose pour l'image envoyée à l'ordinateur par USB."),
        LevelMedia => n("Dans les fichiers", "Ce qui est inscrit dans les vidéos et les photos de la carte SD. La valeur d'usine est l'image seule\u{a0}: les lignes et le menu ne sont pas enregistrés dans le fichier."),
        RecordSize => n("Résolution vidéo", "4K\u{a0}: 3840×2160, fichiers plus gros. 1080p\u{a0}: 1920×1080, la carte en contient davantage."),
        Mirror => n("Miroir", "Retourne l'image quand la caméra est montée tournée au-dessus de la table. Choisissez le réglage avec lequel le circuit se déplace à l'écran dans le sens où la main le pousse. Les touches ⇔ et ⇕ de la télécommande font la même chose."),
        PowerLine => n("Fréquence secteur", "Fréquence du réseau électrique qui alimente l'éclairage\u{a0}: 50 Hz en Europe, 60 Hz en Amérique du Nord. Avec le mauvais réglage, des bandes sombres traversent l'image."),
        Aspect => n("Format d'écran", "Rapport largeur/hauteur du moniteur. Si un objet rond paraît ovale à l'écran, le format choisi n'est pas le bon."),
        Hdmi => n("Mode HDMI", "Résolution et fréquence auxquelles la caméra envoie l'image au moniteur. Auto\u{a0}: le meilleur mode annoncé par le moniteur."),
        Priority => n("Priorité", "Quelle sortie compte le plus. Quand la caméra ne peut pas assurer les deux à la fois, celle-ci garde sa pleine qualité et la qualité de l'autre est réduite."),
        Language => n("Langue", "Langue du menu."),
        Card => n("Carte SD", "Espace libre sur la carte, mode lecteur de carte et formatage."),
        Reset => n("Réinitialiser les réglages", "Remet tous les réglages à leurs valeurs d'usine. Les fichiers de la carte SD ne sont pas touchés."),
        About => n("À propos de la caméra", "Version du micrologiciel et modes actuels des sorties."),
    }
}

fn say(m: &Msg) -> String {
    match m {
        Msg::PhotoSaved(f) => format!("Photo enregistrée · {f}"),
        Msg::Recorded(f, s) => format!("Vidéo enregistrée · {f} · {clock}", clock = clock(*s)),
        Msg::PictureReset => "Image\u{a0}: valeurs d'usine".into(),
        Msg::LinesCleared => "Lignes supprimées".into(),
        Msg::Formatted => "Carte SD formatée".into(),
        Msg::SettingsReset => "Réglages réinitialisés".into(),
        Msg::NoFiles => "Aucune photo ni vidéo sur la carte".into(),
        Msg::Recording => "Galerie indisponible\u{a0}: enregistrement en cours".into(),
        Msg::CardOpen(what) => format!("{cannot}\u{a0}: la carte est ouverte sur l'ordinateur, éjectez-la depuis l'ordinateur", cannot = cannot(what)),
        Msg::CardUnreadable(what) => format!("{cannot}\u{a0}: la caméra ne peut pas lire cette carte, formatez-la dans le menu de la caméra", cannot = cannot(what)),
        Msg::Card(Card::Host) => "Le mode lecteur de carte est activé".into(),
        Msg::Card(Card::Unreadable) => "La caméra ne peut pas lire cette carte\u{a0}: formatez-la dans le menu de la caméra".into(),
        Msg::Card(_) => "Pas de carte SD".into(),
        Msg::NoFreeze => "Arrêt sur image impossible".into(),
        Msg::Resources(what, holders) => {
            let head = match what.subject() {
                Subject::Asked(a) => asked(&a),
                Subject::HdmiOutput => "Sortie HDMI",
                Subject::UsbStream => "Flux USB",
                Subject::Freeze => WORDS.frozen,
                Subject::Menu(Output::Hdmi) => "Menu sur HDMI",
                Subject::Menu(Output::Usb) => "Menu dans le flux USB",
                Subject::Menu(Output::Media) => "Menu dans l'enregistrement",
            };
            // what to do on the next line, the first with a capital
            let todo: Vec<String> = todos(holders)
                .into_iter()
                .map(|t| match t {
                    Todo::LowerHdmi(timing) => format!("réduisez le mode HDMI ({timing})"),
                    Todo::StopUsb { w, h, fps } => format!("arrêtez le flux USB sur l'ordinateur ({w}×{h} · {fps} {unit})", unit = WORDS.fps),
                    Todo::StopRecording(size) => format!("arrêtez l'enregistrement ({size})"),
                    Todo::WaitPhoto => "attendez la fin de la photo".into(),
                    Todo::StopVideo(file) => format!("arrêtez la vidéo ({file})"),
                    Todo::Unfreeze => "quittez l'arrêt sur image".into(),
                    Todo::CloseGallery => "fermez la galerie".into(),
                })
                .collect();
            let s = format!("{head}\u{a0}: {why}", why = why(Code::Resources));
            if todo.is_empty() { s } else { format!("{s}\n{}", capital(&todo.join(", "))) }
        }
        Msg::Refused(what, code) => {
            let reason = match (what, code) {
                (Asked::Control(name), Some(Code::Unavailable)) => unavailable(name).unwrap_or(why(Code::Unavailable)),
                (_, Some(c)) => why(*c),
                (_, None) => "échec",
            };
            format!("{asked}\u{a0}: {reason}", asked = asked(what))
        }
        Msg::Failed(what, code) => {
            let head = match what {
                Action::Photo => "Photo non enregistrée",
                // the card taken out ends a recording; anything else stops it at its start
                Action::Record if *code == Code::NoCard => "Enregistrement interrompu",
                Action::Record => "L'enregistrement n'a pas démarré",
                Action::RecordStop => "L'enregistrement ne s'est pas arrêté",
                Action::Format => "Carte non formatée",
                Action::Reset => "Réglages non réinitialisés",
                Action::Play => "La vidéo n'a pas démarré",
                Action::PlayStop => "La vidéo ne s'est pas arrêtée",
                Action::Control(name) => control(name),
                Action::Output(_) => "La sortie n'a pas démarré",
            };
            format!("{head}\u{a0}: {why}", why = why(*code))
        }
    }
}

/// What could not be done, as a toast that says why opens.
fn cannot(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Photo impossible",
        Asked::Record => "Enregistrement impossible",
        Asked::Gallery => "Galerie indisponible",
        Asked::Format => "Formatage impossible",
        Asked::Control(name) if name == "card-reader" => "Le mode lecteur de carte reste activé",
        what => asked(what),
    }
}

/// What was asked, as a refusal's head names it.
fn asked(what: &Asked) -> &'static str {
    match what {
        Asked::Photo => "Photo",
        Asked::Record => "Enregistrement",
        Asked::RecordStop => "Arrêt de l'enregistrement",
        Asked::Format => "Formatage",
        Asked::Reset => "Réinitialisation",
        Asked::Play | Asked::Player => "Vidéo",
        Asked::Gallery => "Galerie",
        Asked::Control(name) => control(name),
    }
}

/// Why a thing was refused or failed.
fn why(code: Code) -> &'static str {
    match code {
        Code::NoCard => "pas de carte SD",
        Code::CardShared => "le mode lecteur de carte est activé",
        Code::CardOpen => "la carte est ouverte sur l'ordinateur",
        Code::CardUnreadable => "la caméra ne peut pas lire cette carte",
        Code::Busy => "la caméra est encore occupée",
        Code::Playing => "lecture d'une vidéo en cours",
        Code::Recording => "enregistrement en cours",
        Code::Formatting => "formatage de la carte en cours",
        Code::Unavailable => "indisponible pour le moment",
        Code::Refused => "la caméra n'a pas pu le faire",
        Code::Resources => "la caméra manque de ressources",
        Code::Unknown | Code::Invalid => "la caméra a refusé la demande",
    }
}

/// A control's name in a refusal.
fn control(name: &str) -> &'static str {
    match name {
        "brightness" => "Luminosité",
        "contrast" => "Contraste",
        "saturation" => "Saturation",
        "sharpness" => "Netteté",
        "exposure-bias" => "Correction d'exposition",
        "auto-exposure" => "Exposition auto",
        "exposure-lock" => "Verrouillage de l'exposition",
        "white-balance-lock" => "Verrouillage de la balance des blancs",
        "zoom" => "Zoom",
        "mirror" | "flip" => "Miroir",
        "power-line" => "Fréquence secteur",
        "aspect" => "Format d'écran",
        "hdmi" => "Mode HDMI",
        "precedence" => "Priorité",
        "record-size" => "Résolution vidéo",
        "card-reader" => WORDS.card_reader.name,
        "level-hdmi" | "level-usb" | "level-media" => "Affichage",
        _ => "Réglage",
    }
}

/// Why a control cannot be changed now, from its name: the rule microcamd states in `why`, worded.
fn unavailable(name: &str) -> Option<&'static str> {
    Some(match name {
        "exposure-bias" | "exposure-lock" => "l'exposition auto est désactivée",
        "white-balance-lock" => "la balance des blancs auto est désactivée",
        "record-size" => "enregistrement en cours",
        _ => return None,
    })
}
