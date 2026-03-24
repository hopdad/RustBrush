use image::{Rgba, RgbaImage};

mod generate;

pub struct LibraryItem {
    pub name: &'static str,
    pub category: &'static str,
    generator: fn(u32, u32) -> RgbaImage,
}

impl LibraryItem {
    /// Render this library item at the given dimensions.
    pub fn render(&self, width: u32, height: u32) -> RgbaImage {
        (self.generator)(width, height)
    }

    /// Render a small thumbnail (64x64) for gallery display.
    pub fn thumbnail(&self) -> RgbaImage {
        (self.generator)(64, 64)
    }
}

/// All available library items.
pub fn all_items() -> &'static [LibraryItem] {
    &ITEMS
}

/// Get unique category names.
pub fn categories() -> Vec<&'static str> {
    let mut cats: Vec<&'static str> = ITEMS.iter().map(|i| i.category).collect();
    cats.dedup();
    cats
}

/// Get items in a specific category.
pub fn items_in_category(category: &str) -> Vec<&'static LibraryItem> {
    ITEMS.iter().filter(|i| i.category == category).collect()
}

const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const RED: Rgba<u8> = Rgba([220, 40, 40, 255]);
const YELLOW: Rgba<u8> = Rgba([255, 220, 0, 255]);
const GREEN: Rgba<u8> = Rgba([40, 180, 40, 255]);
const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

static ITEMS: [LibraryItem; 24] = [
    // --- Arrows ---
    LibraryItem { name: "Arrow Up", category: "Arrows", generator: generate::arrow_up },
    LibraryItem { name: "Arrow Down", category: "Arrows", generator: generate::arrow_down },
    LibraryItem { name: "Arrow Left", category: "Arrows", generator: generate::arrow_left },
    LibraryItem { name: "Arrow Right", category: "Arrows", generator: generate::arrow_right },
    // --- Warning Signs ---
    LibraryItem { name: "Danger Triangle", category: "Warning Signs", generator: generate::danger_triangle },
    LibraryItem { name: "Radiation", category: "Warning Signs", generator: generate::radiation },
    LibraryItem { name: "No Entry", category: "Warning Signs", generator: generate::no_entry },
    LibraryItem { name: "High Voltage", category: "Warning Signs", generator: generate::high_voltage },
    LibraryItem { name: "Skull", category: "Warning Signs", generator: generate::skull },
    LibraryItem { name: "Biohazard", category: "Warning Signs", generator: generate::biohazard },
    // --- Symbols ---
    LibraryItem { name: "Checkmark", category: "Symbols", generator: generate::checkmark },
    LibraryItem { name: "X Mark", category: "Symbols", generator: generate::x_mark },
    LibraryItem { name: "Heart", category: "Symbols", generator: generate::heart },
    LibraryItem { name: "Star", category: "Symbols", generator: generate::star },
    LibraryItem { name: "Peace", category: "Symbols", generator: generate::peace },
    LibraryItem { name: "Crosshair", category: "Symbols", generator: generate::crosshair },
    // --- Borders ---
    LibraryItem { name: "Simple Border", category: "Borders", generator: generate::simple_border },
    LibraryItem { name: "Double Border", category: "Borders", generator: generate::double_border },
    LibraryItem { name: "Corner Frame", category: "Borders", generator: generate::corner_frame },
    LibraryItem { name: "Dashed Border", category: "Borders", generator: generate::dashed_border },
    // --- Text Labels ---
    LibraryItem { name: "KEEP OUT", category: "Text Labels", generator: generate::label_keep_out },
    LibraryItem { name: "DANGER", category: "Text Labels", generator: generate::label_danger },
    LibraryItem { name: "PRIVATE", category: "Text Labels", generator: generate::label_private },
    LibraryItem { name: "SHOP", category: "Text Labels", generator: generate::label_shop },
];
