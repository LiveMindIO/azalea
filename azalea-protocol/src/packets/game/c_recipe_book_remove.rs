use azalea_buf::AzBuf;
use azalea_protocol_macros::ClientboundGamePacket;

use crate::common::recipe::RecipeDisplayId;

#[derive(AzBuf, ClientboundGamePacket, Clone, Debug, PartialEq)]
pub struct ClientboundRecipeBookRemove {
    pub recipes: Vec<RecipeDisplayId>,
}
