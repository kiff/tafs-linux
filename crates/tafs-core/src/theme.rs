//! The iPad's palette (`Theme.swift`), each colour for the light appearance and for dark.

/// A colour as `0xRRGGBB` and its opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba(pub u32, pub f32);

impl Rgba {
    /// As CSS: `#rrggbb`, or `rgba(…)` when not opaque.
    pub fn css(self) -> String {
        let Rgba(hex, a) = self;
        let (r, g, b) = ((hex >> 16) & 0xFF, (hex >> 8) & 0xFF, hex & 0xFF);
        if a >= 1.0 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("rgba({r}, {g}, {b}, {a:.2})")
        }
    }
}

/// One of the palette's colours: its light and dark values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub name: &'static str,
    pub light: Rgba,
    pub dark: Rgba,
}

const fn both(name: &'static str, light: u32, dark: u32) -> Tone {
    Tone {
        name,
        light: Rgba(light, 1.0),
        dark: Rgba(dark, 1.0),
    }
}

const fn same(name: &'static str, hex: u32) -> Tone {
    both(name, hex, hex)
}

/// `Theme`'s colours, by the name the app's style sheet gives them.
pub const PALETTE: [Tone; 18] = [
    both("background", 0xFBF7EE, 0x141311),
    both("panel", 0xFFFFFF, 0x1F1E1B),
    both("raised", 0xF3ECDF, 0x2A2824),
    both("ink", 0x1B1726, 0xF2EFE9),
    both("soft", 0x4F4A5E, 0xA8A49C),
    both("dim", 0x736D7F, 0x96918A),
    Tone {
        name: "line",
        light: Rgba(0x1B1726, 0.10),
        dark: Rgba(0xF2EFE9, 0.12),
    },
    same("amber", 0xFF8A1F),
    both("amber_text", 0xB35500, 0xF0A45A),
    same("amber_ink", 0xFFFFFF),
    both("on_fill", 0xFFFFFF, 0x141311),
    both("ok", 0x198056, 0x4CC38A),
    both("no", 0xDB1F25, 0xFF8A73),
    same("orange", 0xFF9A2E),
    same("pink", 0xFF4F7B),
    both("blue", 0x2A64F5, 0x7FA2FF),
    both("coach", 0x2A64F5, 0x7FA2FF),
    same("sky", 0x3EC2FF),
];

/// The brand sweep's stops, top-leading to bottom-trailing (`Theme.brand`).
pub const BRAND: [u32; 3] = [0xB35C00, 0xE8003A, 0x2A64F5];

/// Taf colours (`Theme.tafColors`).
pub const TAF_COLORS: [u32; 6] = [0xC25100, 0xE51255, 0x7B4DFF, 0x2A64F5, 0x0B7DA4, 0x128462];

/// `Theme.tafColor`: wraps around both ways.
pub fn taf_color(index: i64) -> u32 {
    let n = TAF_COLORS.len() as i64;
    TAF_COLORS[(((index % n) + n) % n) as usize]
}

pub fn tone(name: &str) -> Option<Tone> {
    PALETTE.iter().copied().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_and_wrapping() {
        assert_eq!(tone("background").unwrap().light.css(), "#fbf7ee");
        assert_eq!(
            tone("line").unwrap().dark.css(),
            "rgba(242, 239, 233, 0.12)"
        );
        assert_eq!(taf_color(-1), 0x128462);
        assert_eq!(taf_color(7), 0xE51255);
    }
}
