// Space-age hacker theme for Descartes GUI
//
// Features:
// - Deep space black with cyan/green terminal accents
// - JetBrains Mono monospace font throughout
// - Neon glow effects on interactive elements
// - Matrix-inspired color scheme
// - Sharp edges, minimal border radius

use iced::Theme;
use iced::theme::Palette;

/// Font definitions for JetBrains Mono
pub mod fonts {
    use iced::Font;

    /// JetBrains Mono Regular - default weight for body text
    pub const MONO: Font = Font::with_name("JetBrains Mono");

    /// JetBrains Mono Medium - for slightly emphasized text
    pub const MONO_MEDIUM: Font = Font {
        weight: iced::font::Weight::Medium,
        ..Font::with_name("JetBrains Mono")
    };

    /// JetBrains Mono Bold - for headings and emphasis
    pub const MONO_BOLD: Font = Font {
        weight: iced::font::Weight::Bold,
        ..Font::with_name("JetBrains Mono")
    };
}

/// Space-age hacker color palette
pub mod colors {
    use iced::Color;

    // Base colors - Deep space blacks
    pub const BACKGROUND: Color = Color::from_rgb(0.02, 0.02, 0.03);       // #050508 - Near black
    pub const SURFACE: Color = Color::from_rgb(0.055, 0.06, 0.075);        // #0e0f13 - Dark panels
    pub const SURFACE_HOVER: Color = Color::from_rgb(0.08, 0.09, 0.11);    // #14171c - Hover state
    pub const SURFACE_ACTIVE: Color = Color::from_rgb(0.1, 0.11, 0.14);    // #1a1c24 - Active state

    // Border colors - Subtle cyan tint
    pub const BORDER: Color = Color::from_rgb(0.12, 0.18, 0.22);           // #1f2e38 - Subtle borders
    pub const BORDER_FOCUS: Color = Color::from_rgb(0.0, 0.8, 0.8);        // #00cccc - Cyan focus glow

    // Text colors - Terminal green/white
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.85, 0.95, 0.9);      // #d9f2e6 - Slight green tint
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.5, 0.65, 0.6);     // #80a699 - Muted green
    pub const TEXT_MUTED: Color = Color::from_rgb(0.3, 0.4, 0.38);         // #4d6660 - Very muted

    // Primary accent - Neon cyan
    pub const PRIMARY: Color = Color::from_rgb(0.0, 0.9, 0.9);             // #00e6e6 - Bright cyan
    pub const PRIMARY_HOVER: Color = Color::from_rgb(0.2, 1.0, 1.0);       // #33ffff - Lighter cyan
    pub const PRIMARY_DIM: Color = Color::from_rgb(0.0, 0.35, 0.4);        // #005966 - Dim cyan

    // Status colors - Neon style
    pub const SUCCESS: Color = Color::from_rgb(0.0, 1.0, 0.5);             // #00ff80 - Neon green
    pub const SUCCESS_DIM: Color = Color::from_rgb(0.0, 0.3, 0.15);        // #004d26 - Dim green
    pub const WARNING: Color = Color::from_rgb(1.0, 0.8, 0.0);             // #ffcc00 - Amber
    pub const WARNING_DIM: Color = Color::from_rgb(0.35, 0.28, 0.0);       // #594700 - Dim amber
    pub const ERROR: Color = Color::from_rgb(1.0, 0.2, 0.3);               // #ff334d - Neon red
    pub const ERROR_DIM: Color = Color::from_rgb(0.4, 0.08, 0.12);         // #66141f - Dim red
    pub const INFO: Color = Color::from_rgb(0.3, 0.6, 1.0);                // #4d99ff - Electric blue

    // Semantic colors for task priorities
    pub const PRIORITY_CRITICAL: Color = ERROR;
    pub const PRIORITY_HIGH: Color = Color::from_rgb(1.0, 0.5, 0.0);       // #ff8000 - Orange
    pub const PRIORITY_MEDIUM: Color = WARNING;
    pub const PRIORITY_LOW: Color = Color::from_rgb(0.4, 0.7, 0.9);        // #66b3e6 - Light blue

    // Agent/status indicators
    pub const AGENT_ACTIVE: Color = SUCCESS;
    pub const AGENT_IDLE: Color = Color::from_rgb(0.35, 0.45, 0.42);       // #597369 - Gray-green
    pub const AGENT_ERROR: Color = ERROR;
    pub const AGENT_PAUSED: Color = WARNING;

    // DAG/Graph colors
    pub const NODE_DEFAULT: Color = SURFACE;
    pub const NODE_SELECTED: Color = PRIMARY_DIM;
    pub const NODE_RUNNING: Color = Color::from_rgb(0.0, 0.25, 0.3);       // #00404d - Teal
    pub const NODE_COMPLETE: Color = SUCCESS_DIM;
    pub const NODE_ERROR: Color = ERROR_DIM;
    pub const EDGE_DEFAULT: Color = BORDER;
    pub const EDGE_ACTIVE: Color = PRIMARY;

    // Special effects
    pub const GLOW_CYAN: Color = Color::from_rgba(0.0, 0.9, 0.9, 0.3);     // Cyan glow
    pub const GLOW_GREEN: Color = Color::from_rgba(0.0, 1.0, 0.5, 0.3);    // Green glow
    pub const SCANLINE: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.1);      // CRT scanline effect
}

/// Create the space-age hacker theme
pub fn humanlayer_theme() -> Theme {
    Theme::custom(
        "SpaceHacker".to_string(),
        Palette {
            background: colors::BACKGROUND,
            text: colors::TEXT_PRIMARY,
            primary: colors::PRIMARY,
            success: colors::SUCCESS,
            danger: colors::ERROR,
        }
    )
}

/// Container styles
pub mod container_styles {
    use iced::{widget::container, Color, Border, Theme};
    use super::colors;

    /// Main panel container style - sharp edges
    pub fn panel(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::SURFACE.into()),
            border: Border {
                width: 1.0,
                color: colors::BORDER,
                radius: 2.0.into(),  // Sharp edges
            },
            ..Default::default()
        }
    }

    /// Card style for individual items
    pub fn card(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::SURFACE.into()),
            border: Border {
                width: 1.0,
                color: colors::BORDER,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Header/toolbar style
    pub fn header(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::BACKGROUND.into()),
            border: Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }

    /// Sidebar navigation style
    pub fn sidebar(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::SURFACE.into()),
            border: Border {
                width: 1.0,
                color: colors::BORDER,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }

    /// Active navigation item - with glow effect
    pub fn nav_active(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::PRIMARY_DIM.into()),
            border: Border {
                width: 1.0,
                color: colors::PRIMARY,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Status badge - success
    pub fn badge_success(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::SUCCESS_DIM.into()),
            border: Border {
                width: 1.0,
                color: colors::SUCCESS,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Status badge - warning
    pub fn badge_warning(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::WARNING_DIM.into()),
            border: Border {
                width: 1.0,
                color: colors::WARNING,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Status badge - error
    pub fn badge_error(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::ERROR_DIM.into()),
            border: Border {
                width: 1.0,
                color: colors::ERROR,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Terminal-style container
    pub fn terminal(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(Color::from_rgb(0.01, 0.01, 0.02).into()),
            border: Border {
                width: 1.0,
                color: colors::PRIMARY_DIM,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }

    /// Tooltip style
    pub fn tooltip(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(colors::SURFACE_ACTIVE.into()),
            border: Border {
                width: 1.0,
                color: colors::BORDER_FOCUS,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Button styles
pub mod button_styles {
    use iced::{widget::button, Border, Color};
    use super::colors;

    /// Primary action button - neon glow effect
    pub fn primary(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, border_color, text) = match status {
            button::Status::Active => (colors::PRIMARY_DIM, colors::PRIMARY, colors::PRIMARY),
            button::Status::Hovered => (colors::PRIMARY, colors::PRIMARY_HOVER, colors::BACKGROUND),
            button::Status::Pressed => (colors::PRIMARY_DIM, colors::PRIMARY, colors::PRIMARY),
            button::Status::Disabled => (colors::SURFACE_ACTIVE, colors::BORDER, colors::TEXT_MUTED),
        };

        button::Style {
            background: Some(bg.into()),
            text_color: text,
            border: Border {
                width: 1.0,
                color: border_color,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Secondary/ghost button
    pub fn secondary(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, border_color, text) = match status {
            button::Status::Active => (colors::SURFACE, colors::BORDER, colors::TEXT_PRIMARY),
            button::Status::Hovered => (colors::SURFACE_HOVER, colors::PRIMARY, colors::PRIMARY),
            button::Status::Pressed => (colors::SURFACE_ACTIVE, colors::PRIMARY_DIM, colors::TEXT_PRIMARY),
            button::Status::Disabled => (colors::SURFACE, colors::BORDER, colors::TEXT_MUTED),
        };

        button::Style {
            background: Some(bg.into()),
            text_color: text,
            border: Border {
                width: 1.0,
                color: border_color,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Navigation button
    pub fn nav(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, text) = match status {
            button::Status::Active => (Color::TRANSPARENT, colors::TEXT_SECONDARY),
            button::Status::Hovered => (colors::SURFACE_HOVER, colors::PRIMARY),
            button::Status::Pressed => (colors::SURFACE_ACTIVE, colors::PRIMARY),
            button::Status::Disabled => (Color::TRANSPARENT, colors::TEXT_MUTED),
        };

        button::Style {
            background: Some(bg.into()),
            text_color: text,
            border: Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Active navigation button (selected state)
    pub fn nav_active(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, border_color) = match status {
            button::Status::Active => (colors::PRIMARY_DIM, colors::PRIMARY),
            button::Status::Hovered => (colors::PRIMARY_DIM, colors::PRIMARY_HOVER),
            button::Status::Pressed => (colors::PRIMARY_DIM, colors::PRIMARY),
            button::Status::Disabled => (colors::SURFACE_ACTIVE, colors::BORDER),
        };

        button::Style {
            background: Some(bg.into()),
            text_color: colors::PRIMARY,
            border: Border {
                width: 1.0,
                color: border_color,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Danger/destructive button
    pub fn danger(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, border_color, text) = match status {
            button::Status::Active => (colors::ERROR_DIM, colors::ERROR, colors::ERROR),
            button::Status::Hovered => (colors::ERROR, colors::ERROR, colors::BACKGROUND),
            button::Status::Pressed => (colors::ERROR_DIM, colors::ERROR, colors::ERROR),
            button::Status::Disabled => (colors::SURFACE_ACTIVE, colors::BORDER, colors::TEXT_MUTED),
        };

        button::Style {
            background: Some(bg.into()),
            text_color: text,
            border: Border {
                width: 1.0,
                color: border_color,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }

    /// Icon-only button (toolbar)
    pub fn icon(_theme: &iced::Theme, status: button::Status) -> button::Style {
        let (bg, text) = match status {
            button::Status::Active => (Color::TRANSPARENT, colors::TEXT_SECONDARY),
            button::Status::Hovered => (colors::SURFACE_HOVER, colors::PRIMARY),
            button::Status::Pressed => (colors::SURFACE_ACTIVE, colors::PRIMARY),
            button::Status::Disabled => (Color::TRANSPARENT, colors::TEXT_MUTED),
        };

        button::Style {
            background: Some(bg.into()),
            text_color: text,
            border: Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: 2.0.into(),
            },
            ..Default::default()
        }
    }
}
