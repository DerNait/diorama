// palette.rs
use std::collections::HashMap;
use std::sync::Arc;

use crate::material::Material;
use crate::texture::Texture;

/// Orden de caras (importante):
/// [PosX, NegX, PosY, NegY, PosZ, NegZ]
///  - PosY = tapa (arriba), NegY = base.
///  - PosZ = frente, NegZ = fondo.
#[derive(Clone)]
pub struct CubeTemplate {
    pub material: Material,
    pub face_textures: [Option<Arc<Texture>>; 6],
}

impl CubeTemplate {
    /// Solo material (sin texturas)
    pub fn material_only(material: Material) -> Self {
        CubeTemplate {
            material,
            face_textures: [None, None, None, None, None, None],
        }
    }

    /// Misma textura en las 6 caras
    pub fn with_same_texture(material: Material, tex: Arc<Texture>) -> Self {
        CubeTemplate {
            face_textures: [
                Some(tex.clone()), Some(tex.clone()), Some(tex.clone()),
                Some(tex.clone()), Some(tex.clone()), Some(tex),
            ],
            material,
        }
    }

    /// Top / Bottom / Sides (lados iguales), útil para terreno tipo Minecraft
    pub fn with_top_bottom_sides(
        material: Material,
        top: Arc<Texture>,
        bottom: Arc<Texture>,
        side: Arc<Texture>,
    ) -> Self {
        CubeTemplate {
            face_textures: [
                Some(side.clone()),  // PosX
                Some(side.clone()),  // NegX
                Some(top.clone()),   // PosY (tapa)
                Some(bottom.clone()),// NegY (base)
                Some(side.clone()),  // PosZ
                Some(side),          // NegZ
            ],
            material,
        }
    }

    /// 6 caras explícitas (usa `None` para “sin textura” en una cara)
    pub fn with_faces(
        material: Material,
        faces: [Option<Arc<Texture>>; 6],
    ) -> Self {
        CubeTemplate { material, face_textures: faces }
    }
}

pub struct Palette {
    map: HashMap<char, CubeTemplate>,
}

impl Palette {
    pub fn new() -> Self {
        Palette { map: HashMap::new() }
    }
    pub fn set(&mut self, ch: char, tpl: CubeTemplate) {
        self.map.insert(ch, tpl);
    }
    pub fn get(&self, ch: char) -> Option<&CubeTemplate> {
        self.map.get(&ch)
    }
}
