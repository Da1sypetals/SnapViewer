use iced::theme::palette::Palette;
use iced::{Color, Theme};

#[allow(dead_code)]
pub struct ColorPalette {
    pub accent: Color,
    pub window_bg: Color,
    pub panel_bg: Color,
    pub text_area_bg: Color,
    pub text_fg: Color,
    pub select_fg: Color,
    pub entry_bg: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub static CUTE: ColorPalette = ColorPalette {
    accent: rgb(0xe9, 0x1e, 0x63),
    window_bg: rgb(0xff, 0xf5, 0xf8),
    panel_bg: rgb(0xf8, 0xbb, 0xdd),
    text_area_bg: rgb(0xfc, 0xe4, 0xec),
    text_fg: rgb(0x2d, 0x2d, 0x2d),
    select_fg: Color::WHITE,
    entry_bg: rgb(0xfc, 0xe4, 0xec),
};

pub static DEFAULT: ColorPalette = ColorPalette {
    accent: rgb(0x15, 0x65, 0xc0),
    window_bg: rgb(0xe8, 0xea, 0xf0),
    panel_bg: rgb(0xd0, 0xd4, 0xe0),
    text_area_bg: Color::WHITE,
    text_fg: rgb(0x1a, 0x1a, 0x2e),
    select_fg: Color::WHITE,
    entry_bg: Color::WHITE,
};

pub static NIGHT: ColorPalette = ColorPalette {
    accent: rgb(0xad, 0x70, 0xf7),
    window_bg: rgb(0x12, 0x12, 0x12),
    panel_bg: rgb(0x1e, 0x1e, 0x1e),
    text_area_bg: rgb(0x2d, 0x2d, 0x2d),
    text_fg: rgb(0xe0, 0xe0, 0xe0),
    select_fg: rgb(0x12, 0x12, 0x12),
    entry_bg: rgb(0x3a, 0x3a, 0x3a),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PaletteName {
    Cute,
    Default,
    Night,
}

impl PaletteName {
    pub fn palette(&self) -> &'static ColorPalette {
        match self {
            PaletteName::Cute => &CUTE,
            PaletteName::Default => &DEFAULT,
            PaletteName::Night => &NIGHT,
        }
    }

    pub fn to_theme(&self) -> Theme {
        let cp = self.palette();
        let pal = Palette {
            background: cp.window_bg,
            text: cp.text_fg,
            primary: cp.accent,
            success: rgb(0x2e, 0xa4, 0x43),
            warning: rgb(0xed, 0xa0, 0x12),
            danger: rgb(0xd1, 0x2d, 0x2d),
        };
        Theme::custom(self.label().to_string(), pal)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PaletteName::Cute => "cute",
            PaletteName::Default => "default",
            PaletteName::Night => "night",
        }
    }
}

impl std::fmt::Display for PaletteName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}
