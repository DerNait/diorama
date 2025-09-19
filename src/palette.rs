// palette.rs
use std::collections::HashMap;
use std::sync::Arc;

use raylib::prelude::Vector3;

use crate::material::Material;
use crate::texture::Texture;

/// Estilo de muestreo por cara.
/// - Normal: usa el color de la textura tal cual.
/// - GrayscaleTint: asume textura B/N; multiplica una tinta por la luminancia.
/// - BlackIsTransparent: alpha-test por luminancia (si <= umbral, se considera hueco).
/// - GrayscaleTintBlackTransparent: combina ambas (tinta por luminancia y descarta por umbral).
#[derive(Clone)]
pub enum TexStyle {
    Normal,
    GrayscaleTint { color: Vector3 },
    BlackIsTransparent { threshold: f32 }, // p.ej. 0.05
    GrayscaleTintBlackTransparent { color: Vector3, threshold: f32 },
}

/// Capa de cara: textura + estilo de muestreo.
#[derive(Clone)]
pub struct FaceStyle {
    pub tex: Arc<Texture>,
    pub style: TexStyle,
}

/// Orden de caras (importante):
/// [PosX, NegX, PosY, NegY, PosZ, NegZ]
///  - PosY = tapa (arriba), NegY = base.
///  - PosZ = frente, NegZ = fondo.
#[derive(Clone)]
pub struct CubeTemplate {
    pub material: Material,
    pub face_textures: [Option<FaceStyle>; 6],
}

impl CubeTemplate {
    /// Solo material (sin texturas)
    pub fn material_only(material: Material) -> Self {
        CubeTemplate {
            material,
            face_textures: [None, None, None, None, None, None],
        }
    }

    /// Misma textura en las 6 caras (estilo NORMAL por defecto)
    pub fn with_same_texture(material: Material, tex: Arc<Texture>) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::Normal };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Misma textura en las 6 caras, pero usando TINTA para B/N.
    pub fn with_same_texture_tinted(material: Material, tex: Arc<Texture>, color: Vector3) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::GrayscaleTint { color } };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Misma textura con “negro es transparente” (alpha-test por umbral).
    pub fn with_same_texture_black_transparent(
        material: Material,
        tex: Arc<Texture>,
        threshold: f32,
    ) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::BlackIsTransparent { threshold } };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Misma textura B/N tintada **y** con “negro transparente”.
    pub fn with_same_texture_tinted_black_transparent(
        material: Material,
        tex: Arc<Texture>,
        color: Vector3,
        threshold: f32,
    ) -> Self {
        let fs = FaceStyle {
            tex: tex.clone(),
            style: TexStyle::GrayscaleTintBlackTransparent { color, threshold },
        };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Top / Bottom / Sides (lados iguales), útil para terreno tipo Minecraft (NORMAL).
    pub fn with_top_bottom_sides(
        material: Material,
        top: Arc<Texture>,
        bottom: Arc<Texture>,
        side: Arc<Texture>,
    ) -> Self {
        CubeTemplate {
            face_textures: [
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::Normal }), // PosX
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::Normal }), // NegX
                Some(FaceStyle { tex: top.clone(),    style: TexStyle::Normal }), // PosY
                Some(FaceStyle { tex: bottom.clone(), style: TexStyle::Normal }), // NegY
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::Normal }), // PosZ
                Some(FaceStyle { tex: side,           style: TexStyle::Normal }), // NegZ
            ],
            material,
        }
    }

    /// Variante tintada para B/N (top/bottom/sides).
    pub fn with_top_bottom_sides_tinted(
        material: Material,
        top: Arc<Texture>, top_color: Vector3,
        bottom: Arc<Texture>, bottom_color: Vector3,
        side: Arc<Texture>, side_color: Vector3,
    ) -> Self {
        CubeTemplate {
            face_textures: [
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::GrayscaleTint { color: side_color } }),
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::GrayscaleTint { color: side_color } }),
                Some(FaceStyle { tex: top.clone(),    style: TexStyle::GrayscaleTint { color: top_color } }),
                Some(FaceStyle { tex: bottom.clone(), style: TexStyle::GrayscaleTint { color: bottom_color } }),
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::GrayscaleTint { color: side_color } }),
                Some(FaceStyle { tex: side,           style: TexStyle::GrayscaleTint { color: side_color } }),
            ],
            material,
        }
    }

    /// 6 caras explícitas con estilos (usa `None` para “sin textura” en una cara).
    pub fn with_faces_styled(
        material: Material,
        faces: [Option<(Arc<Texture>, TexStyle)>; 6],
    ) -> Self {
        let map = |opt: Option<(Arc<Texture>, TexStyle)>| {
            opt.map(|(tex, style)| FaceStyle { tex, style })
        };
        CubeTemplate {
            material,
            face_textures: [
                map(faces[0].clone()),
                map(faces[1].clone()),
                map(faces[2].clone()),
                map(faces[3].clone()),
                map(faces[4].clone()),
                map(faces[5].clone()),
            ],
        }
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
