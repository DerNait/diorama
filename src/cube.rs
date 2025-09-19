use std::sync::Arc;
use raylib::prelude::Vector3;

use crate::material::Material;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::texture::Texture;
use crate::palette::{FaceStyle, TexStyle};

#[derive(Clone, Copy)]
pub enum Face { PosX, NegX, PosY, NegY, PosZ, NegZ }
impl Face {
    #[inline] fn idx(self) -> usize {
        match self { Face::PosX=>0, Face::NegX=>1, Face::PosY=>2, Face::NegY=>3, Face::PosZ=>4, Face::NegZ=>5 }
    }
}

/// AABB con texturas por cara (opcionales, compartidas por Arc)
pub struct Cube {
    pub min: Vector3,
    pub max: Vector3,
    pub material: Material,
    face_textures: [Option<FaceStyle>; 6],
}

impl Cube {
    pub fn from_center_size(center: Vector3, size: Vector3, material: Material) -> Self {
        let half = size * 0.5;
        Cube {
            min: center - half,
            max: center + half,
            material,
            face_textures: [None, None, None, None, None, None],
        }
    }

    pub fn new(min: Vector3, max: Vector3, material: Material) -> Self {
        Cube { min, max, material, face_textures: [None, None, None, None, None, None] }
    }

    /// Compatibilidad: setea textura en estilo NORMAL
    pub fn set_face_texture(&mut self, face: Face, tex: Arc<Texture>) {
        self.face_textures[face.idx()] = Some(FaceStyle { tex, style: TexStyle::Normal });
    }

    /// NUEVO: setear textura con estilo
    pub fn set_face_texture_styled(&mut self, face: Face, tex: Arc<Texture>, style: TexStyle) {
        self.face_textures[face.idx()] = Some(FaceStyle { tex, style });
    }

    pub fn set_face_textures_from_template(&mut self, tpl: &[Option<FaceStyle>; 6]) {
        self.face_textures = [
            tpl[0].clone(), tpl[1].clone(), tpl[2].clone(),
            tpl[3].clone(), tpl[4].clone(), tpl[5].clone(),
        ];
    }
}

#[inline]
fn luminance(rgb: Vector3) -> f32 {
    // Coeficientes Rec.709 aproximados
    (rgb.x * 0.2126 + rgb.y * 0.7152 + rgb.z * 0.0722).clamp(0.0, 1.0)
}

/// Aplica el estilo de muestreo. Devuelve:
/// - Some(color) si el texel es visible
/// - None si el texel debe considerarse transparente (alpha-test)
fn sample_with_style(tex: &Texture, u: f32, v: f32, style: &TexStyle) -> Option<Vector3> {
    let base = tex.sample_clamp(u, v); // 0..1
    match style {
        TexStyle::Normal => Some(base),

        TexStyle::GrayscaleTint { color } => {
            let a = luminance(base);
            Some(Vector3::new(color.x * a, color.y * a, color.z * a))
        }

        TexStyle::BlackIsTransparent { threshold } => {
            let a = luminance(base);
            if a <= *threshold { None } else { Some(base) }
        }

        TexStyle::GrayscaleTintBlackTransparent { color, threshold } => {
            let a = luminance(base);
            if a <= *threshold { None } else {
                Some(Vector3::new(color.x * a, color.y * a, color.z * a))
            }
        }
    }
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ro: &Vector3, rd: &Vector3) -> Intersect {
        // Slabs con tracking del eje de entrada (robusto en aristas)
        let inv = Vector3::new(1.0 / rd.x, 1.0 / rd.y, 1.0 / rd.z);

        let (tx1, tx2) = ((self.min.x - ro.x) * inv.x, (self.max.x - ro.x) * inv.x);
        let (ty1, ty2) = ((self.min.y - ro.y) * inv.y, (self.max.y - ro.y) * inv.y);
        let (tz1, tz2) = ((self.min.z - ro.z) * inv.z, (self.max.z - ro.z) * inv.z);

        let tmin_x = tx1.min(tx2);
        let tmax_x = tx1.max(tx2);
        let tmin_y = ty1.min(ty2);
        let tmax_y = ty1.max(ty2);
        let tmin_z = tz1.min(tz2);
        let tmax_z = tz1.max(tz2);

        let t_enter = tmin_x.max(tmin_y).max(tmin_z);
        let t_exit  = tmax_x.min(tmax_y).min(tmax_z);

        if t_exit < 0.0 || t_enter > t_exit {
            return Intersect::empty();
        }

        // NOTA: si hubiera alpha-test y el primer texel cae “transparente”,
        // no hacemos un segundo “raycast” interno — para cubos voxel está ok.
        let t_hit = if t_enter > 0.0 { t_enter } else { t_exit };
        if !t_hit.is_finite() { return Intersect::empty(); }

        let p = *ro + *rd * t_hit;

        // Eje de la cara golpeada
        let face = if t_enter == tmin_x || (tmin_x > tmin_y && tmin_x > tmin_z) {
            if rd.x > 0.0 { Face::NegX } else { Face::PosX }
        } else if t_enter == tmin_y || (tmin_y > tmin_z) {
            if rd.y > 0.0 { Face::NegY } else { Face::PosY }
        } else {
            if rd.z > 0.0 { Face::NegZ } else { Face::PosZ }
        };

        let normal = match face {
            Face::PosX => Vector3::new( 1.0, 0.0, 0.0),
            Face::NegX => Vector3::new(-1.0, 0.0, 0.0),
            Face::PosY => Vector3::new( 0.0, 1.0, 0.0),
            Face::NegY => Vector3::new( 0.0,-1.0, 0.0),
            Face::PosZ => Vector3::new( 0.0, 0.0, 1.0),
            Face::NegZ => Vector3::new( 0.0, 0.0,-1.0),
        };

        // UV por cara
        let size = self.max - self.min;
        let (mut u, mut v) = match face {
            Face::PosX => ( (p.z - self.min.z) / size.z, (self.max.y - p.y) / size.y ),
            Face::NegX => ( (self.max.z - p.z) / size.z, (self.max.y - p.y) / size.y ),
            Face::PosY => ( (p.x - self.min.x) / size.x, (p.z - self.min.z) / size.z ),
            Face::NegY => ( (p.x - self.min.x) / size.x, (self.max.z - p.z) / size.z ),
            Face::PosZ => ( (p.x - self.min.x) / size.x, (self.max.y - p.y) / size.y ),
            Face::NegZ => ( (self.max.x - p.x) / size.x, (self.max.y - p.y) / size.y ),
        };

        // Clampeo sutil para evitar costuras
        let tiny = 1e-6f32;
        u = u.clamp(0.0 + tiny, 1.0 - tiny);
        v = v.clamp(0.0 + tiny, 1.0 - tiny);

        // Material final
        let final_material = if let Some(face_layer) = &self.face_textures[face.idx()] {
            // Aplica estilo (tint y/o alpha-test)
            match sample_with_style(&face_layer.tex, u, v, &face_layer.style) {
                Some(tex_color) => {
                    Material { diffuse: tex_color, ..self.material }
                }
                None => {
                    // “Hueco” por alpha-test: no impacta este cubo
                    return Intersect::empty();
                }
            }
        } else {
            self.material
        };

        Intersect::new(p, normal, t_hit, final_material)
    }

    fn aabb(&self) -> (Vector3, Vector3) {
        (self.min, self.max)
    }
}
