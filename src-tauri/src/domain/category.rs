//! The category palette, the curated icons and the default color (DESIGN_SYSTEM §3.4, §6).

/// The palette keys a category's color may be. The colors themselves live in the CSS tokens.
pub const PALETTE: [&str; 12] = [
    "cat-1", "cat-2", "cat-3", "cat-4", "cat-5", "cat-6", "cat-7", "cat-8", "cat-9", "cat-10",
    "cat-11", "cat-12",
];

/// The Lucide icons a category may use. `src/lib/categoryIcons.ts` must list the same names.
pub const ICONS: [&str; 40] = [
    // The seed categories' icons (DATA_MODEL §7).
    "briefcase",
    "award",
    "laptop",
    "percent",
    "gift",
    "rotate-ccw",
    "circle-plus",
    "shopping-cart",
    "utensils",
    "car",
    "house",
    "zap",
    "smartphone",
    "shopping-bag",
    "heart-pulse",
    "shield",
    "film",
    "repeat",
    "sparkles",
    "graduation-cap",
    "heart-handshake",
    "plane",
    "receipt",
    "ellipsis",
    // Extra choices for categories the user adds.
    "coffee",
    "bus",
    "fuel",
    "baby",
    "paw-print",
    "dumbbell",
    "gamepad-2",
    "music",
    "book-open",
    "shirt",
    "wrench",
    "piggy-bank",
    "banknote",
    "coins",
    "pill",
    "wifi",
];

/// The default color for a new category: the palette key `used` holds least often, the lowest
/// number on a tie. `used` is the colors of the active categories of the new one's kind.
pub fn least_used_color<'a>(used: impl IntoIterator<Item = &'a str>) -> &'static str {
    let mut counts = [0_usize; PALETTE.len()];
    for color in used {
        if let Some(i) = PALETTE.iter().position(|key| *key == color) {
            counts[i] += 1;
        }
    }
    // min_by_key keeps the first of equal minimums, which is the lowest number.
    PALETTE
        .into_iter()
        .zip(counts)
        .min_by_key(|(_, count)| *count)
        .map_or(PALETTE[0], |(key, _)| key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn least_used_color_picks_the_rarest_and_breaks_ties_by_number() {
        assert_eq!(least_used_color([]), "cat-1");
        assert_eq!(least_used_color(["cat-1"]), "cat-2");
        let mut every_key_and_cat_1_again = PALETTE.to_vec();
        every_key_and_cat_1_again.push("cat-1");
        assert_eq!(least_used_color(every_key_and_cat_1_again), "cat-2");
        // cat-10 sorts before cat-2 as text, but the tie goes to the lower number.
        let all_but_2_and_10: Vec<&str> = PALETTE
            .into_iter()
            .filter(|key| !["cat-2", "cat-10"].contains(key))
            .collect();
        assert_eq!(least_used_color(all_but_2_and_10), "cat-2");
        assert_eq!(least_used_color(["#ff0000", "cat-1"]), "cat-2");
    }

    #[test]
    fn curated_icons_match_the_frontend() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/categoryIcons.ts");
        let source = std::fs::read_to_string(path).expect("src/lib/categoryIcons.ts exists");
        // Entries look like `"shopping-cart": ShoppingCart,` or, unquoted, `car: Car,`.
        let mut frontend: Vec<&str> = source
            .lines()
            .filter_map(|line| line.trim().split_once(": "))
            .filter(|(_, value)| value.starts_with(|c: char| c.is_ascii_uppercase()))
            .map(|(key, _)| key.trim_matches('"'))
            .collect();
        let mut backend = ICONS.to_vec();
        frontend.sort_unstable();
        backend.sort_unstable();
        assert_eq!(frontend, backend);
    }
}
