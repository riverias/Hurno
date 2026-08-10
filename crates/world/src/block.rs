/// Block identifier — stored as u8 in chunks (palette)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum BlockId {
    #[default]
    Air        = 0,
    Stone      = 1,
    Grass      = 2,
    Dirt       = 3,
    Cobblestone= 4,
    Planks     = 5,
    Sapling    = 6,
    Bedrock    = 7,
    Water      = 8,
    Lava       = 10,
    Sand       = 12,
    Gravel     = 13,
    GoldOre    = 14,
    IronOre    = 15,
    CoalOre    = 16,
    Log        = 17,
    Leaves     = 18,
    Sponge     = 19,
    Glass      = 20,
}

impl BlockId {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1  => Self::Stone,
            2  => Self::Grass,
            3  => Self::Dirt,
            4  => Self::Cobblestone,
            5  => Self::Planks,
            6  => Self::Sapling,
            7  => Self::Bedrock,
            8  => Self::Water,
            10 => Self::Lava,
            12 => Self::Sand,
            13 => Self::Gravel,
            14 => Self::GoldOre,
            15 => Self::IronOre,
            16 => Self::CoalOre,
            17 => Self::Log,
            18 => Self::Leaves,
            19 => Self::Sponge,
            20 => Self::Glass,
            _  => Self::Air,
        }
    }
}

/// Six faces of a cube
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockFace { Top, Bottom, North, South, East, West }

/// Per-block static definition
#[derive(Debug, Clone, Copy)]
pub struct BlockDef {
    pub id:          BlockId,
    pub name:        &'static str,
    pub solid:       bool,
    pub transparent: bool,
    pub emits_light: u8,
    /// Atlas tile index: (top, bottom, side)
    pub tex:         (u8, u8, u8),
}

impl BlockDef {
    pub fn tex_for(&self, face: BlockFace) -> u8 {
        match face {
            BlockFace::Top    => self.tex.0,
            BlockFace::Bottom => self.tex.1,
            _                 => self.tex.2,
        }
    }
}

pub const BLOCK_DEFS: &[BlockDef] = &[
    BlockDef { id: BlockId::Air,         name: "air",         solid: false, transparent: true,  emits_light: 0,  tex: (0,  0,  0)  },
    BlockDef { id: BlockId::Stone,       name: "stone",       solid: true,  transparent: false, emits_light: 0,  tex: (1,  1,  1)  },
    BlockDef { id: BlockId::Grass,       name: "grass",       solid: true,  transparent: false, emits_light: 0,  tex: (0,  2,  3)  },
    BlockDef { id: BlockId::Dirt,        name: "dirt",        solid: true,  transparent: false, emits_light: 0,  tex: (2,  2,  2)  },
    BlockDef { id: BlockId::Cobblestone, name: "cobblestone", solid: true,  transparent: false, emits_light: 0,  tex: (16, 16, 16) },
    BlockDef { id: BlockId::Planks,      name: "planks",      solid: true,  transparent: false, emits_light: 0,  tex: (4,  4,  4)  },
    BlockDef { id: BlockId::Bedrock,     name: "bedrock",     solid: true,  transparent: false, emits_light: 0,  tex: (17, 17, 17) },
    BlockDef { id: BlockId::Water,       name: "water",       solid: false, transparent: true,  emits_light: 0,  tex: (205,205,205)},
    BlockDef { id: BlockId::Lava,        name: "lava",        solid: false, transparent: false, emits_light: 15, tex: (237,237,237)},
    BlockDef { id: BlockId::Sand,        name: "sand",        solid: true,  transparent: false, emits_light: 0,  tex: (18, 18, 18) },
    BlockDef { id: BlockId::Gravel,      name: "gravel",      solid: true,  transparent: false, emits_light: 0,  tex: (19, 19, 19) },
    BlockDef { id: BlockId::GoldOre,     name: "gold_ore",    solid: true,  transparent: false, emits_light: 0,  tex: (32, 32, 32) },
    BlockDef { id: BlockId::IronOre,     name: "iron_ore",    solid: true,  transparent: false, emits_light: 0,  tex: (33, 33, 33) },
    BlockDef { id: BlockId::CoalOre,     name: "coal_ore",    solid: true,  transparent: false, emits_light: 0,  tex: (34, 34, 34) },
    BlockDef { id: BlockId::Log,         name: "log",         solid: true,  transparent: false, emits_light: 0,  tex: (21, 21, 20) },
    BlockDef { id: BlockId::Leaves,      name: "leaves",      solid: true,  transparent: true,  emits_light: 0,  tex: (22, 22, 22) },
    BlockDef { id: BlockId::Glass,       name: "glass",       solid: true,  transparent: true,  emits_light: 0,  tex: (49, 49, 49) },
];

pub fn block_def(id: BlockId) -> &'static BlockDef {
    BLOCK_DEFS.iter().find(|b| b.id == id).unwrap_or(&BLOCK_DEFS[0])
}
