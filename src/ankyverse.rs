/// The Ankyverse — 8 kingdoms mapped to the chakra system.
///
/// Each kingdom has 3 cities. Stories are placed in a kingdom based on the
/// chakra that resonates with the parent's writing, and in a random city
/// within that kingdom.
use rand::prelude::*;

pub struct Kingdom {
    pub number: u8,
    pub name: &'static str,
    pub chakra: &'static str,
    pub theme: &'static str,
    pub element: &'static str,
    pub lesson: &'static str,
    pub cities: [&'static str; 3],
}

pub const KINGDOMS: [Kingdom; 8] = [
    Kingdom {
        number: 1,
        name: "Primordia",
        chakra: "Root",
        theme: "Survival",
        element: "Earth",
        lesson: "You are here. You are alive. Start there.",
        cities: ["Rubicund Ridge", "Bleeding Bay", "Marsh Metropolis"],
    },
    Kingdom {
        number: 2,
        name: "Emblazion",
        chakra: "Sacral",
        theme: "Passion",
        element: "Fire",
        lesson: "What do you want so badly it terrifies you?",
        cities: ["Lava Landing", "Frond Fiesta", "Amber Atrium"],
    },
    Kingdom {
        number: 3,
        name: "Chryseos",
        chakra: "Solar Plexus",
        theme: "Willpower",
        element: "Gold",
        lesson: "You are not waiting for permission.",
        cities: ["Lustrous Landing", "Savanna Soiree", "Sandstone Square"],
    },
    Kingdom {
        number: 4,
        name: "Eleasis",
        chakra: "Heart",
        theme: "Compassion",
        element: "Air",
        lesson: "The wall around your heart is made of the same material as the prison.",
        cities: ["Grove Galleria", "Leaf Spot", "Pond Pavilion"],
    },
    Kingdom {
        number: 5,
        name: "Voxlumis",
        chakra: "Throat",
        theme: "Communication",
        element: "Sound",
        lesson: "Say the thing you're afraid to say. That is the one that matters.",
        cities: ["Echo Enclave", "Sapphire Settlement", "Woodland Wharf"],
    },
    Kingdom {
        number: 6,
        name: "Insightia",
        chakra: "Third Eye",
        theme: "Intuition",
        element: "Light",
        lesson: "You already know. You have always known.",
        cities: ["Maze Metropolis", "Veil Venue", "Dreamweaver's Dwelling"],
    },
    Kingdom {
        number: 7,
        name: "Claridium",
        chakra: "Crown",
        theme: "Enlightenment",
        element: "Crystal",
        lesson: "Who is the one asking who am I?",
        cities: ["Crystal City", "Ascent Arrival", "Echo Empire"],
    },
    Kingdom {
        number: 8,
        name: "Poiesis",
        chakra: "Transcendence",
        theme: "Creativity",
        element: "Creation",
        lesson: "You are not the creator. You are the channel. Get out of the way.",
        cities: ["Creation City", "Inlet Island", "Muse's Metropolis"],
    },
];

pub fn kingdom_by_number(n: u8) -> Option<&'static Kingdom> {
    KINGDOMS.iter().find(|k| k.number == n)
}

pub fn random_city(kingdom: &Kingdom) -> &'static str {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..kingdom.cities.len());
    kingdom.cities[index]
}

/// Build a lore snippet for inclusion in story prompts.
pub fn kingdom_lore_snippet(kingdom: &Kingdom, city: &str) -> String {
    format!(
        "Kingdom: {} (the {} chakra — {})\n\
         City: {}\n\
         Element: {}\n\
         The lesson of this land: \"{}\"",
        kingdom.name, kingdom.chakra, kingdom.theme, city, kingdom.element, kingdom.lesson
    )
}

/// Full Ankyverse context for the story generation prompt.
pub fn ankyverse_context() -> &'static str {
    "The Ankyverse is a world of 8 kingdoms, each mapped to a chakra. \
     Every kingdom has 3 cities. The kingdoms are: \
     Primordia (Root/Survival), Emblazion (Sacral/Passion), \
     Chryseos (Solar Plexus/Willpower), Eleasis (Heart/Compassion), \
     Voxlumis (Throat/Communication), Insightia (Third Eye/Intuition), \
     Claridium (Crown/Enlightenment), and Poiesis (Transcendence/Creativity). \
     Children and their parents live in houses that move between kingdoms — \
     the house always feels like the same house, but it travels to wherever \
     the story needs to be told. The house is special because it carries the \
     feeling of home no matter where it lands."
}

/// The 5 target languages for story TTS.
pub const LANGUAGES: [(&str, &str); 5] = [
    ("en", "English"),
    ("es", "Spanish"),
    ("zh", "Mandarin Chinese"),
    ("hi", "Hindi"),
    ("ar", "Arabic"),
];
