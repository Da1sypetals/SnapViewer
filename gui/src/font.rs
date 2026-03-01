// Font module for JetBrains Mono monospaced font
use iced::Font;

/// JetBrains Mono font bytes loaded at compile time
pub const FONT_BYTES: &[u8] = include_bytes!("../../assets/JetBrainsMono-Medium.ttf");

/// Font name used to reference JetBrains Mono in text widgets
pub const JETBRAINS_MONO: Font = Font::with_name("JetBrains Mono");
