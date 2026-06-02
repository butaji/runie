//! Theme module - re-exports from themes subdirectory.

pub mod themes;

pub use themes::{
    ThemeWrapper,
    ThemeColors,
    ColorPalette,
    ColorCapability,
    OpalineColor,
    resolve_theme,
};
