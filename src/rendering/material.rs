use crate::assets::{Asset, MaterialId, ShaderId, TextureId};

pub struct Material {
    pub id: MaterialId,
    pub vertex_shader_id: ShaderId,
    pub fragment_shader_id: ShaderId,
}

impl Asset for Material {
    fn id(&self) -> MaterialId {
        self.id
    }
}
