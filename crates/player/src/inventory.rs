use world::BlockId;

pub const HOTBAR_SIZE: usize = 9;
pub const INV_ROWS:   usize = 3;
pub const INV_COLS:   usize = 9;
pub const INV_SIZE:   usize = INV_ROWS * INV_COLS;

#[derive(Debug, Clone, Copy, Default)]
pub struct ItemStack {
    pub id:    BlockId,
    pub count: u8,
}

#[derive(Debug, Clone)]
pub struct Inventory {
    pub hotbar:   [Option<ItemStack>; HOTBAR_SIZE],
    pub slots:    [Option<ItemStack>; INV_SIZE],
    pub selected: usize,
    pub open:     bool,
}

impl Inventory {
    pub fn classic_starter() -> Self {
        let mut inv = Self {
            hotbar:   [None; HOTBAR_SIZE],
            slots:    [None; INV_SIZE],
            selected: 0,
            open:     false,
        };
        let blocks = [
            BlockId::Grass,
            BlockId::Stone,
            BlockId::Planks,
            BlockId::Cobblestone,
            BlockId::Log,
            BlockId::Leaves,
            BlockId::Sand,
            BlockId::Glass,
            BlockId::Dirt,
        ];
        for (i, &id) in blocks.iter().enumerate() {
            inv.hotbar[i] = Some(ItemStack { id, count: 64 });
        }
        inv
    }

    pub fn selected_block(&self) -> Option<BlockId> {
        self.hotbar[self.selected].map(|s| s.id)
    }

    pub fn scroll(&mut self, delta: i32) {
        let n = HOTBAR_SIZE as i32;
        self.selected = ((self.selected as i32 + delta).rem_euclid(n)) as usize;
    }

    pub fn consume_selected(&mut self) {
        if let Some(slot) = &mut self.hotbar[self.selected] {
            if slot.count > 1 { slot.count -= 1; }
            // In creative/classic we don't deplete, but here for survival:
            // else { self.hotbar[self.selected] = None; }
        }
    }
}
