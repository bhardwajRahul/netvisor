use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString, IntoStaticStr};
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Default,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    Display,
    EnumString,
)]
pub enum Color {
    Pink,
    Rose,
    Red,
    Amber,
    Orange,
    Green,
    Emerald,
    Teal,
    Cyan,
    Blue,
    Indigo,
    Purple,
    Fuchsia,
    Violet,
    Sky,
    Gray,
    Lime,
    #[default]
    #[serde(other)]
    Yellow,
}

impl Color {
    /// Tailwind-palette hex pair `(background, text)` for rendering an
    /// entity tag in an HTML email. Background is the 100 shade; text is
    /// the 800 shade — mirrors the `GenericCard` tag styling in the UI.
    pub fn email_tag_hex(&self) -> (&'static str, &'static str) {
        match self {
            Color::Pink => ("#fce7f3", "#9d174d"),
            Color::Rose => ("#ffe4e6", "#9f1239"),
            Color::Red => ("#fee2e2", "#991b1b"),
            Color::Amber => ("#fef3c7", "#92400e"),
            Color::Orange => ("#ffedd5", "#9a3412"),
            Color::Green => ("#dcfce7", "#166534"),
            Color::Emerald => ("#d1fae5", "#065f46"),
            Color::Teal => ("#ccfbf1", "#115e59"),
            Color::Cyan => ("#cffafe", "#155e75"),
            Color::Blue => ("#dbeafe", "#1e40af"),
            Color::Indigo => ("#e0e7ff", "#3730a3"),
            Color::Purple => ("#f3e8ff", "#6b21a8"),
            Color::Fuchsia => ("#fae8ff", "#86198f"),
            Color::Violet => ("#ede9fe", "#5b21b6"),
            Color::Sky => ("#e0f2fe", "#075985"),
            Color::Gray => ("#f3f4f6", "#1f2937"),
            Color::Lime => ("#ecfccb", "#3f6212"),
            Color::Yellow => ("#fef9c3", "#854d0e"),
        }
    }
}
