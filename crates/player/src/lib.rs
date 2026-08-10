pub mod player;
pub mod inventory;

pub use player::Player;
pub use player::Input;          // <-- теперь доступен как player::Input
pub use inventory::{Inventory, ItemStack, HOTBAR_SIZE};
