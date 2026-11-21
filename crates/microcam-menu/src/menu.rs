//! The menu's tables: its sections, the items in each, what an item is (a number on a slider, a value, an action),
//! which camera control it shows. Their names and descriptions are each language's (`text`).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Picture,
    Exposure,
    Overlays,
    Recording,
    Setup,
    System,
}

/// A row of the menu's first level: a section, or the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Section(Section),
    Gallery,
}

pub const ROOT: [Entry; 7] = [
    Entry::Section(Section::Picture),
    Entry::Section(Section::Exposure),
    Entry::Section(Section::Overlays),
    Entry::Section(Section::Recording),
    Entry::Gallery,
    Entry::Section(Section::Setup),
    Entry::Section(Section::System),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Brightness,
    Contrast,
    Saturation,
    Sharpness,
    ResetPicture,
    AutoExposure,
    ExposureBias,
    ExposureLock,
    WhiteBalanceLock,
    Cross,
    AddLine,
    ClearLines,
    LevelHdmi,
    LevelUsb,
    LevelMedia,
    RecordSize,
    Mirror,
    PowerLine,
    Aspect,
    Hdmi,
    Priority,
    Language,
    Card,
    Reset,
    About,
}

/// What an item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// a number edited on the slider: its control
    Number(&'static str),
    /// one of a list of values: two toggle in place, more open the list
    Value,
    /// an action, or a level of its own
    Action,
}

impl Section {
    pub fn items(self) -> &'static [Item] {
        use Item::*;
        match self {
            Section::Picture => &[Brightness, Contrast, Saturation, Sharpness, ResetPicture],
            Section::Exposure => &[AutoExposure, ExposureBias, ExposureLock, WhiteBalanceLock],
            Section::Overlays => &[Cross, AddLine, ClearLines, LevelHdmi, LevelUsb, LevelMedia],
            Section::Recording => &[RecordSize],
            Section::Setup => &[Mirror, PowerLine, Aspect, Hdmi, Priority],
            Section::System => &[Language, Card, Reset, About],
        }
    }

    pub fn of(item: Item) -> Section {
        [Section::Picture, Section::Exposure, Section::Overlays, Section::Recording, Section::Setup, Section::System]
            .into_iter()
            .find(|s| s.items().contains(&item))
            .expect("every item is in a section")
    }
}

impl Item {
    /// The ◐ key's round of the picture's numbers.
    pub const PICTURE: [Item; 4] = [Item::Brightness, Item::Contrast, Item::Saturation, Item::Sharpness];

    pub fn kind(self) -> Kind {
        use Item::*;
        match self {
            Brightness => Kind::Number("brightness"),
            Contrast => Kind::Number("contrast"),
            Saturation => Kind::Number("saturation"),
            Sharpness => Kind::Number("sharpness"),
            ExposureBias => Kind::Number("exposure-bias"),
            AutoExposure | ExposureLock | WhiteBalanceLock | Cross | LevelHdmi | LevelUsb | LevelMedia | RecordSize | Mirror | PowerLine | Aspect | Hdmi | Priority | Language => Kind::Value,
            ResetPicture | AddLine | ClearLines | Card | Reset | About => Kind::Action,
        }
    }

    /// The camera control a value item is one of (the mirror is two, `mirror` and `flip`; the crosshair and the
    /// language are the menu's own).
    pub fn control(self) -> Option<&'static str> {
        use Item::*;
        Some(match self {
            Brightness | Contrast | Saturation | Sharpness | ExposureBias => match self.kind() {
                Kind::Number(c) => c,
                _ => unreachable!("a number"),
            },
            AutoExposure => "auto-exposure",
            ExposureLock => "exposure-lock",
            WhiteBalanceLock => "white-balance-lock",
            LevelHdmi => "level-hdmi",
            LevelUsb => "level-usb",
            LevelMedia => "level-media",
            RecordSize => "record-size",
            PowerLine => "power-line",
            Aspect => "aspect",
            Hdmi => "hdmi",
            Priority => "precedence",
            _ => return None,
        })
    }
}
