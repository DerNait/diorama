// palette.rs
use std::collections::HashMap;
use std::sync::Arc;

use raylib::prelude::Vector3;

use crate::material::Material;
use crate::texture::Texture;

/// Estilo de muestreo por cara.
/// - Normal: usa el color de la textura.
/// - GrayscaleTint: asume B/N; tinta por luminancia.
/// - BlackIsTransparent: cutout por luminancia (<= umbral → hueco).
/// - GrayscaleTintBlackTransparent: tinta + cutout por luminancia.
/// - ImageAlphaCutout: cutout por canal alpha del PNG (<= umbral → hueco).
/// - GrayscaleTintImageAlphaCutout: tinta + cutout por alpha.
/// - ImageAlphaWindow: usa alpha como **coverage** (0..1), NO corta el rayo; ideal ventana.
/// - GrayscaleTintImageAlphaWindow: igual que arriba con tinta para B/N.
#[derive(Clone)]
pub enum TexStyle {
    Normal,
    GrayscaleTint { color: Vector3 },
    BlackIsTransparent { threshold: f32 },
    GrayscaleTintBlackTransparent { color: Vector3, threshold: f32 },
    ImageAlphaCutout { threshold: f32 },
    GrayscaleTintImageAlphaCutout { color: Vector3, threshold: f32 },
    ImageAlphaWindow { threshold: f32 },
    GrayscaleTintImageAlphaWindow { color: Vector3, threshold: f32 },
}

/// Capa de cara: textura + estilo de muestreo.
#[derive(Clone)]
pub struct FaceStyle {
    pub tex: Arc<Texture>,
    pub style: TexStyle,
}

/// Orden de caras (importante):
/// [PosX, NegX, PosY, NegY, PosZ, NegZ]
#[derive(Clone)]
pub struct CubeTemplate {
    pub material: Material,
    pub face_textures: [Option<FaceStyle>; 6],
}

impl CubeTemplate {
    pub fn material_only(material: Material) -> Self {
        CubeTemplate {
            material,
            face_textures: [None, None, None, None, None, None],
        }
    }

    pub fn with_same_texture(material: Material, tex: Arc<Texture>) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::Normal };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    pub fn with_same_texture_tinted(material: Material, tex: Arc<Texture>, color: Vector3) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::GrayscaleTint { color } };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    pub fn with_same_texture_black_transparent(
        material: Material, tex: Arc<Texture>, threshold: f32,
    ) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::BlackIsTransparent { threshold } };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    pub fn with_same_texture_tinted_black_transparent(
        material: Material, tex: Arc<Texture>, color: Vector3, threshold: f32,
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

    /// Cutout por alpha de imagen (zonas transparentes desaparecen).
    pub fn with_same_texture_image_alpha(
        material: Material, tex: Arc<Texture>, threshold: f32,
    ) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::ImageAlphaCutout { threshold } };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Tinta + cutout por alpha de imagen.
    pub fn with_same_texture_tinted_image_alpha(
        material: Material, tex: Arc<Texture>, color: Vector3, threshold: f32,
    ) -> Self {
        let fs = FaceStyle {
            tex: tex.clone(),
            style: TexStyle::GrayscaleTintImageAlphaCutout { color, threshold },
        };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Ventana: usa alpha como coverage (0..1), NO corta el rayo (sigue reflejando).
    pub fn with_same_texture_image_alpha_window(
        material: Material, tex: Arc<Texture>, threshold: f32,
    ) -> Self {
        let fs = FaceStyle { tex: tex.clone(), style: TexStyle::ImageAlphaWindow { threshold } };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    /// Ventana tintada (B/N) + coverage por alpha.
    pub fn with_same_texture_tinted_image_alpha_window(
        material: Material, tex: Arc<Texture>, color: Vector3, threshold: f32,
    ) -> Self {
        let fs = FaceStyle {
            tex: tex.clone(),
            style: TexStyle::GrayscaleTintImageAlphaWindow { color, threshold },
        };
        CubeTemplate {
            face_textures: [Some(fs.clone()), Some(fs.clone()), Some(fs.clone()),
                            Some(fs.clone()), Some(fs.clone()), Some(fs)],
            material,
        }
    }

    pub fn with_top_bottom_sides(
        material: Material,
        top: Arc<Texture>,
        bottom: Arc<Texture>,
        side: Arc<Texture>,
    ) -> Self {
        CubeTemplate {
            face_textures: [
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::Normal }),
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::Normal }),
                Some(FaceStyle { tex: top.clone(),    style: TexStyle::Normal }),
                Some(FaceStyle { tex: bottom.clone(), style: TexStyle::Normal }),
                Some(FaceStyle { tex: side.clone(),   style: TexStyle::Normal }),
                Some(FaceStyle { tex: side,           style: TexStyle::Normal }),
            ],
            material,
        }
    }

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
