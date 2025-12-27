use serde::Serialize;

// --- Main Chunk Structure ---
// Represents the root of the .mca data hierarchy
#[derive(Debug, Serialize)]
pub struct ChunkData {
    // Minecraft 1.21.1 requires DataVersion 3955.
    #[serde(rename = "DataVersion")]
    pub data_version: i32,

    // Chunk coordinates (absolute, not relative to region)
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "zPos")]
    pub z_pos: i32,

    // Lowest Y coordinate. In 1.18+ this is usually -64.
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
    #[serde(rename = "isLightOn")]
    pub is_light_on: i8,

    // Vertical slices of the chunk (16 blocks high each)
    pub sections: Vec<Section>,
}

// --- Section (16x16x16 Cube) ---
#[derive(Debug, Serialize)]
pub struct Section {
    // Vertical index of this section (e.g., -4 for the bottom, up to 19)
    #[serde(rename = "Y")]
    pub y: i8,

    // The blocks inside this section
    pub block_states: BlockStates,

    // The biomes inside this section
    pub biomes: Biomes,
}

// --- Block Palette ---
// Minecraft uses "Paletted Storage". Instead of storing 4096 block IDs,
// it stores a list of unique blocks (Palette).
#[derive(Debug, Serialize)]
pub struct BlockStates {
    pub palette: Vec<BlockState>,
    // Note: 'data' field is omitted. If the palette has only 1 item,
    // Minecraft assumes the whole section is filled with that block.
}

// --- Biome Palette ---
#[derive(Debug, Serialize)]
pub struct Biomes {
    pub palette: Vec<String>,
    // 'data' omitted here as well for single-biome sections.
}

// --- Single Block ---
#[derive(Debug, Serialize)]
pub struct BlockState {
    #[serde(rename = "Name")]
    pub name: String,
    // Properties (like waterlogged, facing) are optional/omitted for MVP.
}