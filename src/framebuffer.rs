// framebuffer.rs

use raylib::prelude::*;

/// Framebuffer CPU con textura GPU persistente (sin recreación por frame).
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pixels: Vec<Color>,                               // buffer CPU: width*height
    texture_gpu: Option<raylib::texture::Texture2D>,  // textura persistente
    background_color: Color,
    current_color: Color,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize) * (height as usize);
        Framebuffer {
            width,
            height,
            pixels: vec![Color::BLACK; n],
            texture_gpu: None,
            background_color: Color::BLACK,
            current_color: Color::WHITE,
        }
    }

    /// Debes crear la Texture2D UNA sola vez (desde un Image temporal) y adjuntarla aquí.
    pub fn attach_texture(&mut self, tex: raylib::texture::Texture2D) {
        self.texture_gpu = Some(tex);
    }

    /// Acceso mutable al buffer para render paralelo.
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [Color] {
        &mut self.pixels
    }

    /// Acceso de solo lectura, por si lo necesitas.
    #[inline]
    pub fn pixels(&self) -> &[Color] {
        &self.pixels
    }

    /// Limpia el buffer CPU sin recrearlo.
    pub fn clear(&mut self) {
        let bg = self.background_color;
        for px in self.pixels.iter_mut() {
            *px = bg;
        }
    }

    /// Escritura de píxel directa (para usos puntuales).
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32) {
        if x >= self.width || y >= self.height { return; }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.pixels[idx] = self.current_color;
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    pub fn set_current_color(&mut self, color: Color) {
        self.current_color = color;
    }

    /// Sube el buffer CPU a la textura persistente y **pinta**.
    /// Acepta un `draw_overlay` para que dibujes el HUD en el **mismo frame** (una sola Begin/End).
    pub fn swap_buffers_with<F>(
        &mut self,
        window: &mut RaylibHandle,
        raylib_thread: &RaylibThread,
        mut draw_overlay: F,
    )
    where
        F: FnMut(&mut RaylibDrawHandle),
    {
        if let Some(tex) = &mut self.texture_gpu {
            let byte_len = self.pixels.len() * std::mem::size_of::<Color>();
            let bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(self.pixels.as_ptr() as *const u8, byte_len)
            };

            // Actualiza TODO el área de la textura (0,0, w, h)
            let rect = Rectangle::new(0.0, 0.0, self.width as f32, self.height as f32);
            tex.update_texture_rec(rect, bytes).expect("update_texture_rec failed");

            // Dibuja frame + overlay en una sola pasada
            let mut d = window.begin_drawing(raylib_thread);
            d.clear_background(Color::BLACK);
            d.draw_texture(tex, 0, 0, Color::WHITE);

            // HUD/overlay del usuario
            draw_overlay(&mut d);
        }
    }
}
