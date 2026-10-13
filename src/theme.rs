use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use palette::{
    FromColor, Lab, Mix, Srgb, Xyz,
    chromatic_adaptation::AdaptIntoUnclamped,
    white_point::{D50, D65},
};
use ratatui::style::Color;
use serde::Deserialize;
use syntect::highlighting::{StyleModifier, Theme, ThemeItem};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
}

impl Appearance {
    pub fn toggle(self) -> Self {
        if self == Self::Dark {
            Self::Light
        } else {
            Self::Dark
        }
    }
    pub fn name(self) -> &'static str {
        if self == Self::Dark { "dark" } else { "light" }
    }
}

#[derive(Clone)]
pub struct Colors {
    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub chrome: Color,
    pub separator: Color,
    pub empty: Color,
    pub context: Color,
    pub added: Color,
    pub deleted: Color,
    pub accent: Color,
    pub add_bg: Color,
    pub del_bg: Color,
    pub add_word: Color,
    pub del_word: Color,
}

#[derive(Deserialize)]
struct ThemeData {
    colors: HashMap<String, String>,
    #[serde(rename = "tokenColors")]
    tokens: Vec<Token>,
}

#[derive(Deserialize)]
struct Token {
    scope: serde_json::Value,
    settings: HashMap<String, String>,
}

fn rgb(value: &str) -> Result<Color> {
    let value = value.trim_start_matches('#');
    let n = u32::from_str_radix(value.get(..6).unwrap_or(value), 16)?;
    Ok(Color::Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

fn syntect_color(color: Color) -> syntect::highlighting::Color {
    let Color::Rgb(r, g, b) = color else {
        unreachable!("theme colors are RGB")
    };
    syntect::highlighting::Color { r, g, b, a: 255 }
}

pub fn load(appearance: Appearance) -> Result<(Colors, Theme)> {
    let input = match appearance {
        Appearance::Dark => include_str!("../assets/pierre/pierre-dark.json"),
        Appearance::Light => include_str!("../assets/pierre/pierre-light.json"),
    };
    let data: ThemeData = serde_json::from_str(input)?;
    let get = |key: &str| -> Result<Color> { rgb(&data.colors[key]) };
    let bg = get("editor.background")?;
    let fg = get("editor.foreground")?;
    let added = get("gitDecoration.addedResourceForeground")?;
    let deleted = get("gitDecoration.deletedResourceForeground")?;
    let dark = appearance == Appearance::Dark;
    let add_bg = lab_mix(bg, added, if dark { 0.20 } else { 0.12 });
    let del_bg = lab_mix(bg, deleted, if dark { 0.20 } else { 0.12 });
    let colors = Colors {
        bg,
        fg,
        added,
        deleted,
        dim: get("editorLineNumber.foreground")?,
        chrome: get("sideBar.background")?,
        accent: get("gitDecoration.modifiedResourceForeground")?,
        separator: lab_mix(bg, fg, if dark { 0.15 } else { 0.04 }),
        empty: lab_mix(bg, fg, if dark { 0.18 } else { 0.10 }),
        context: lab_mix(bg, fg, if dark { 0.075 } else { 0.015 }),
        add_bg,
        del_bg,
        add_word: alpha_mix(add_bg, added, if dark { 0.20 } else { 0.15 }),
        del_word: alpha_mix(del_bg, deleted, if dark { 0.20 } else { 0.15 }),
    };
    let mut syntax = Theme::default();
    syntax.settings.foreground = Some(syntect_color(fg));
    syntax.settings.background = Some(syntect_color(bg));
    for token in data.tokens {
        let scopes = match token.scope {
            serde_json::Value::String(s) => s,
            serde_json::Value::Array(items) => items
                .iter()
                .filter_map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(","),
            _ => continue,
        };
        if let Some(fg) = token.settings.get("foreground") {
            syntax.scopes.push(ThemeItem {
                scope: FromStr::from_str(&scopes)?,
                style: StyleModifier {
                    foreground: Some(syntect_color(rgb(fg)?)),
                    ..StyleModifier::default()
                },
            });
        }
    }
    Ok((colors, syntax))
}

fn components(color: Color) -> (u8, u8, u8) {
    let Color::Rgb(r, g, b) = color else {
        unreachable!("theme colors are RGB")
    };
    (r, g, b)
}

fn lab_mix(a: Color, b: Color, amount: f32) -> Color {
    let (r, g, blue) = components(a);
    let a: Xyz<D65, f32> = Xyz::from_color(Srgb::new(r, g, blue).into_format::<f32>());
    let a: Xyz<D50, f32> = a.adapt_into_unclamped();
    let a = Lab::from_color(a);
    let (r, g, blue) = components(b);
    let b: Xyz<D65, f32> = Xyz::from_color(Srgb::new(r, g, blue).into_format::<f32>());
    let b: Xyz<D50, f32> = b.adapt_into_unclamped();
    let b = Lab::from_color(b);
    let mixed: Xyz<D50, f32> = Xyz::from_color(a.mix(b, amount));
    let adapted: Xyz<D65, f32> = mixed.adapt_into_unclamped();
    let color: Srgb<u8> = Srgb::from_color(adapted).into_format();
    Color::Rgb(color.red, color.green, color.blue)
}

fn alpha_mix(a: Color, b: Color, amount: f32) -> Color {
    let a = components(a);
    let b = components(b);
    let blend = |a: u8, b: u8| (a as f32 * (1.0 - amount) + b as f32 * amount).round() as u8;
    Color::Rgb(blend(a.0, b.0), blend(a.1, b.1), blend(a.2, b.2))
}
