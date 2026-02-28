from dataclasses import dataclass


@dataclass
class ColorPalette:
    accent: str        # titles, selection bg, cursor, prompt
    window_bg: str     # root window background
    panel_bg: str      # Panel.TFrame background
    text_area_bg: str  # ScrolledText background
    text_fg: str       # ScrolledText foreground
    select_fg: str     # selection foreground (text on highlight)
    entry_bg: str      # Entry widget background (input box)


CUTE = ColorPalette(
    accent="#e91e63",
    window_bg="#fff5f8",
    panel_bg="#f8bbdd",
    text_area_bg="#fce4ec",
    text_fg="#2d2d2d",
    select_fg="white",
    entry_bg="#fce4ec",
)

DEFAULT = ColorPalette(
    accent="#1565c0",
    window_bg="#e8eaf0",
    panel_bg="#d0d4e0",
    text_area_bg="#ffffff",
    text_fg="#1a1a2e",
    select_fg="white",
    entry_bg="#ffffff",
)

NIGHT = ColorPalette(
    accent="#ad70f7",
    window_bg="#121212",
    panel_bg="#1e1e1e",
    text_area_bg="#2d2d2d",
    text_fg="#e0e0e0",
    select_fg="#121212",
    entry_bg="#3a3a3a",
)
