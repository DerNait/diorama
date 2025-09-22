use raylib::prelude::Vector3;
use crate::material::Material;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Intersect {
    pub point: Vector3,
    pub normal: Vector3,
    pub distance: f32,
    pub is_intersecting: bool,
    pub material: Material,
    /// Cobertura 0..1 del texel (1 = opaco, 0 = totalmente transparente).
    /// Se usa para sombreado y para que las sombras ignoren superficies “ventana”.
    pub coverage: f32,
    pub object_index: Option<usize>,
}

impl Intersect {
    pub fn new(point: Vector3, normal: Vector3, distance: f32, material: Material) -> Self {
        Intersect {
            point,
            normal,
            distance,
            is_intersecting: true,
            material,
            coverage: 1.0,
            object_index: None,
        }
    }

    /// Construye un hit con cobertura explícita (para texturas con alpha).
    pub fn with_coverage(
        point: Vector3, normal: Vector3, distance: f32, material: Material, coverage: f32
    ) -> Self {
        Intersect {
            point,
            normal,
            distance,
            is_intersecting: true,
            material,
            coverage: coverage.clamp(0.0, 1.0),
            object_index: None,
        }
    }

    pub fn empty() -> Self {
        Intersect {
            point: Vector3::zero(),
            normal: Vector3::zero(),
            distance: 0.0,
            is_intersecting: false,
            material: Material::black(),
            coverage: 0.0,
            object_index: None,
        }
    }
}

/// Los objetos deben proveer intersección y su AABB para la aceleración.
pub trait RayIntersect: Send + Sync {
    fn ray_intersect(&self, ray_origin: &Vector3, ray_direction: &Vector3) -> Intersect;

    /// AABB en espacio mundo para aceleración (grilla/BVH).
    fn aabb(&self) -> (Vector3, Vector3);
}
