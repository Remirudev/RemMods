use remmods_core::settings as core;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ColorScheme {
    Dark,
    Light,
    System,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TitleBarMode {
    Custom,
    System,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub scheme: ColorScheme,
    /// Always finite (core normalizes it), so never the `null` serde_json writes for NaN.
    #[specta(type = specta_typescript::Number)]
    pub ui_scale: f64,
    pub title_bar: TitleBarMode,
    pub hide_pictures: bool,
    pub hover_preview: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    pub appearance: Appearance,
    pub read_only: bool,
    pub recovered: bool,
}

/// Partial update: omitted or `null` fields keep their current value.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceUpdate {
    #[specta(optional)]
    pub scheme: Option<ColorScheme>,
    #[specta(optional)]
    pub ui_scale: Option<f64>,
    #[specta(optional)]
    pub title_bar: Option<TitleBarMode>,
    #[specta(optional)]
    pub hide_pictures: Option<bool>,
    #[specta(optional)]
    pub hover_preview: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
    #[specta(optional)]
    pub appearance: Option<AppearanceUpdate>,
}

impl From<core::ColorScheme> for ColorScheme {
    fn from(scheme: core::ColorScheme) -> Self {
        match scheme {
            core::ColorScheme::Dark => Self::Dark,
            core::ColorScheme::Light => Self::Light,
            core::ColorScheme::System => Self::System,
        }
    }
}

impl From<ColorScheme> for core::ColorScheme {
    fn from(scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Dark => Self::Dark,
            ColorScheme::Light => Self::Light,
            ColorScheme::System => Self::System,
        }
    }
}

impl From<core::TitleBarMode> for TitleBarMode {
    fn from(mode: core::TitleBarMode) -> Self {
        match mode {
            core::TitleBarMode::Custom => Self::Custom,
            core::TitleBarMode::System => Self::System,
        }
    }
}

impl From<TitleBarMode> for core::TitleBarMode {
    fn from(mode: TitleBarMode) -> Self {
        match mode {
            TitleBarMode::Custom => Self::Custom,
            TitleBarMode::System => Self::System,
        }
    }
}

impl From<core::Appearance> for Appearance {
    fn from(appearance: core::Appearance) -> Self {
        Self {
            scheme: appearance.scheme.into(),
            ui_scale: appearance.ui_scale,
            title_bar: appearance.title_bar.into(),
            hide_pictures: appearance.hide_pictures,
            hover_preview: appearance.hover_preview,
        }
    }
}

impl From<core::SettingsSnapshot> for Settings {
    fn from(snapshot: core::SettingsSnapshot) -> Self {
        Self {
            version: snapshot.settings.version,
            appearance: snapshot.settings.appearance.into(),
            read_only: snapshot.read_only,
            recovered: snapshot.recovered,
        }
    }
}

impl From<AppearanceUpdate> for core::AppearanceUpdate {
    fn from(update: AppearanceUpdate) -> Self {
        Self {
            scheme: update.scheme.map(Into::into),
            ui_scale: update.ui_scale,
            title_bar: update.title_bar.map(Into::into),
            hide_pictures: update.hide_pictures,
            hover_preview: update.hover_preview,
        }
    }
}

impl From<SettingsUpdate> for core::SettingsUpdate {
    fn from(update: SettingsUpdate) -> Self {
        Self {
            appearance: update.appearance.map(Into::into),
        }
    }
}
