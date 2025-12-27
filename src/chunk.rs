use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ChunkRoot {
    #[serde(rename = "DataVersion")]
    pub data_version: i32,
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "yPos")]
    pub y_pos: i32,
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "LastUpdate")]
    pub last_update: i64,
    #[serde(rename = "sections")]
    pub sections: Vec<Section>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Section {
    #[serde(rename = "Y")] // Y - big letter in NBT 
    pub y: i8,
    pub block_states: Option<BlockStates>,
    pub biomes: Option<Biomes>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockStates {
    pub palette: Vec<BlockState>,
    // no data if only air 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<fastnbt::LongArray>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockState {
    #[serde(rename = "Name")] // Minecraft need this
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Biomes {
    pub palette: Vec<String>,
    pub data: Option<fastnbt::LongArray>,
}