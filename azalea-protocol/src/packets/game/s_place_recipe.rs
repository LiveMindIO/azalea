use azalea_buf::AzBuf;
use azalea_protocol_macros::ServerboundGamePacket;

use crate::common::recipe::RecipeDisplayId;

#[derive(AzBuf, Clone, Debug, PartialEq, ServerboundGamePacket)]
pub struct ServerboundPlaceRecipe {
    #[var]
    pub container_id: i32,
    pub recipe: RecipeDisplayId,
    pub shift_down: bool,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn recipe_display_id_is_a_varint() {
        let packet = ServerboundPlaceRecipe {
            container_id: 2,
            recipe: RecipeDisplayId(300),
            shift_down: false,
        };
        let mut bytes = Vec::new();
        packet.azalea_write(&mut bytes).unwrap();
        assert_eq!(bytes, vec![2, 0xac, 2, 0]);
        assert_eq!(
            ServerboundPlaceRecipe::azalea_read(&mut Cursor::new(&bytes)).unwrap(),
            packet
        );
    }
}
