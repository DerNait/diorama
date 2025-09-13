use raylib::prelude::*;

/// Textura CPU-side con muestreo por UV.
/// Guarda el buffer de colores para muestrear sin pedir &mut.
pub struct Texture {
    width: i32,
    height: i32,
    pixels: ImageColors, // Box<[Color]> administrado por raylib-rs
}

impl Texture {
    pub fn from_file(path: &str) -> Self {
        let img = Image::load_image(path).expect("No se pudo cargar la textura");
        let w = img.width();
        let h = img.height();
        let pixels = img.get_image_data(); // row-major, origen top-left
        Texture { width: w, height: h, pixels }
    }

    /// ---- MODO REPEAT (wrap) ----
    /// Devuelve color lineal [0,1] por UV, con wrap repetido.
    /// Convención: v=0 es fila superior (top), v=1 inferior (bottom).
    #[inline]
    pub fn sample_repeat(&self, mut u: f32, mut v: f32) -> Vector3 {
        // Wrap a [0,1)
        u = u.fract();
        if u < 0.0 { u += 1.0; }
        v = v.fract();
        if v < 0.0 { v += 1.0; }

        // Centro de texel: (u*W - 0.5, v*H - 0.5)
        let sx = u * self.width as f32  - 0.5;
        let sy = v * self.height as f32 - 0.5;

        let xi = sx.floor().clamp(0.0, self.width  as f32 - 1.0)  as usize;
        let yi = sy.floor().clamp(0.0, self.height as f32 - 1.0)  as usize;
        let idx = yi * self.width as usize + xi;

        let c = self.pixels[idx];
        Vector3::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0)
    }

    /// ---- MODO CLAMP (sin wrap) ----
    /// Clampa los UV a los centros de texel válidos para evitar costuras en 0/1.
    #[inline]
    pub fn sample_clamp(&self, mut u: f32, mut v: f32) -> Vector3 {
        // “Inset” de medio texel en UV-space
        let eps_u = 0.5 / self.width as f32;
        let eps_v = 0.5 / self.height as f32;
        u = u.clamp(eps_u, 1.0 - eps_u);
        v = v.clamp(eps_v, 1.0 - eps_v);

        // Centro de texel
        let sx = u * self.width as f32  - 0.5;
        let sy = v * self.height as f32 - 0.5;

        let xi = sx.floor().clamp(0.0, self.width  as f32 - 1.0)  as usize;
        let yi = sy.floor().clamp(0.0, self.height as f32 - 1.0)  as usize;
        let idx = yi * self.width as usize + xi;

        let c = self.pixels[idx];
        Vector3::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0)
    }

    /// Alias para compatibilidad: por defecto, repeat.
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> Vector3 {
        self.sample_repeat(u, v)
    }
}
