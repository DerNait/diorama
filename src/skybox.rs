use std::sync::Arc;
use raylib::prelude::Vector3;

use crate::texture::Texture;

/// Orden y nombres de archivo requeridos en la carpeta:
/// posx.png (Right), negx.png (Left), posy.png (Top), negy.png (Bottom), posz.png (Front), negz.png (Back)
///
/// Convención de cubemap (OpenGL style) para las proyecciones y signos:
/// +X: u = (-Rz/|Rx|+1)/2, v = (-Ry/|Rx|+1)/2
/// -X: u = ( Rz/|Rx|+1)/2, v = (-Ry/|Rx|+1)/2
/// +Y: u = ( Rx/|Ry|+1)/2, v = ( Rz/|Ry|+1)/2
/// -Y: u = ( Rx/|Ry|+1)/2, v = (-Rz/|Ry|+1)/2
/// +Z: u = ( Rx/|Rz|+1)/2, v = (-Ry/|Rz|+1)/2
/// -Z: u = (-Rx/|Rz|+1)/2, v = (-Ry/|Rz|+1)/2
///
/// Nota: nuestras texturas se muestrean con origen **arriba-izquierda** (top-left),
/// por lo que invertimos v: v = 1 - v_raw, para que no aparezca verticalmente volteado.
pub struct Skybox {
    posx: Arc<Texture>,
    negx: Arc<Texture>,
    posy: Arc<Texture>,
    negy: Arc<Texture>,
    posz: Arc<Texture>,
    negz: Arc<Texture>,
}

impl Skybox {
    /// Carga un skybox desde una carpeta con archivos:
    /// posx.png, negx.png, posy.png, negy.png, posz.png, negz.png
    pub fn from_folder(folder: &str) -> Self {
        let join = |name: &str| -> String { format!("{}/{}", folder, name) };
        let posx = Arc::new(Texture::from_file(&join("posx.png")));
        let negx = Arc::new(Texture::from_file(&join("negx.png")));
        let posy = Arc::new(Texture::from_file(&join("posy.png")));
        let negy = Arc::new(Texture::from_file(&join("negy.png")));
        let posz = Arc::new(Texture::from_file(&join("posz.png")));
        let negz = Arc::new(Texture::from_file(&join("negz.png")));
        Skybox { posx, negx, posy, negy, posz, negz }
    }

    /// Devuelve el color RGB [0..1] para un rayo (dirección en mundo).
    pub fn sample(&self, dir: Vector3) -> Vector3 {
        let r = dir.normalized();
        let ax = r.x.abs();
        let ay = r.y.abs();
        let az = r.z.abs();

        // Elige cara dominante
        if ax >= ay && ax >= az {
            // Cara X
            let (tex, sc, tc, ma) = if r.x > 0.0 {
                (&self.posx, -r.z, -r.y, ax) // +X
            } else {
                (&self.negx,  r.z, -r.y, ax) // -X
            };
            let u = (sc / ma + 1.0) * 0.5;
            let v_raw = (tc / ma + 1.0) * 0.5;
            let u = (sc / ma + 1.0) * 0.5;
            let v = v_raw; // invertir v por origen top-left
            let u = 1.0 - u;
            tex.sample_clamp(u, v)
        } else if ay >= ax && ay >= az {
            // Cara Y
            let (tex, sc, tc, ma) = if r.y > 0.0 {
                (&self.posy,  r.x,  r.z, ay) // +Y (top)
            } else {
                (&self.negy,  r.x, -r.z, ay) // -Y (bottom)
            };
            let u = (sc / ma + 1.0) * 0.5;
            let v_raw = (tc / ma + 1.0) * 0.5;
            let v = 1.0 - v_raw;
            tex.sample_clamp(u, v)
        } else {
            // Cara Z
            let (tex, sc, tc, ma) = if r.z > 0.0 {
                (&self.posz,  r.x, -r.y, az) // +Z (front)
            } else {
                (&self.negz, -r.x, -r.y, az) // -Z (back)
            };
            let u = (sc / ma + 1.0) * 0.5;
            let v_raw = (tc / ma + 1.0) * 0.5;
            let v = v_raw;
            let u = 1.0 - u;
            tex.sample_clamp(u, v)
        }
    }
}
