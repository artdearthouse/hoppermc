//! NBT (Named Binary Tag) structures for Minecraft chunk data.
//!
//! These structures are serialized using fastnbt to create valid
//! Minecraft chunk data compatible with version 1.21.11.

use serde::{Deserialize, Serialize};

/// Minecraft data version for 1.21.11 (default).
/// Can be overridden by MC_DATA_VERSION env var.
pub fn get_data_version() -> i32 {
    std::env::var("MC_DATA_VERSION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4671)
}

/// Main chunk structure - the root of NBT hierarchy in .mca files.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChunkData {
    #[serde(rename = "DataVersion")]
    pub data_version: i32,

    // Chunk coordinates (absolute, not relative to region)
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    #[serde(rename = "yPos")]
    pub y_pos: i32,

    // "minecraft:full" tells the server the chunk is fully generated.
    #[serde(rename = "Status")]
    pub status: String,

    // Required timing fields
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,

    #[serde(rename = "InhabitedTime")]
    pub inhabited_time: i64,

    // Light calculation status
    #[serde(rename = "isLightOn", default)]
    pub is_light_on: Option<i8>,

    // Vertical slices of the chunk (16 blocks high each)
    pub sections: Vec<Section>,
}

// --- Section (16x16x16 Cube) ---
#[derive(Debug, Serialize, Deserialize)]
pub struct Section {
    // Vertical index of this section (e.g., -4 for the bottom, up to 19)
    #[serde(rename = "Y")]
    pub y: i8,

    // The blocks inside this section
    // Optional because empty sections might omit this.
    // Also aliased to handle potential capitalization differences.
    #[serde(rename = "block_states", alias = "BlockStates", default)]
    pub block_states: Option<BlockStates>,

    // The biomes inside this section
    #[serde(rename = "biomes", alias = "Biomes", default)]
    pub biomes: Option<Biomes>,
}

// --- Block Palette ---
// Minecraft uses "Paletted Storage". Instead of storing 4096 block IDs,
// it stores a list of unique blocks (Palette).
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockStates {
    pub palette: Vec<BlockState>,
    // Indices into the palette. Required if palette length > 1.
    // We use fastnbt::LongArray to ensure it serializes as an NBT Long Array.
    #[serde(default)]
    pub data: Option<fastnbt::LongArray>,
}

// --- Biome Palette ---
#[derive(Debug, Serialize, Deserialize)]
pub struct Biomes {
    pub palette: Vec<String>,
    // Indices into the palette. Required if palette length > 1.
    #[serde(default)]
    pub data: Option<fastnbt::LongArray>,
}

// --- Single Block ---
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockState {
    #[serde(rename = "Name")]
    pub name: String,
    // Properties (like waterlogged, facing) are optional/omitted for MVP.
}