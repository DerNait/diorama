// build.rs
use raylib::prelude::*;
use crate::material::Material;
use crate::ray_intersect::RayIntersect;
use crate::cube::Cube;
use crate::palette::CubeTemplate;

/// Sprites del HUD (hotbar estilo Minecraft).
pub struct HudSprites {
    pub hotbar: Texture2D,        // 182x22
    pub selection: Texture2D,     // 24x23
    /// Íconos en el mismo orden que `options` (uno por bloque).
    pub icons: Vec<Texture2D>,
}

/// Config visual del HUD.
#[derive(Clone, Copy)]
pub struct HudConfig {
    /// Escala global (1.0 = 100%)
    pub scale: f32,
    /// Margen inferior en pixels de pantalla (no escala)
    pub bottom_margin: i32,
    /// Padding interno por lado para iconos (2px por lado de MC → 2.0)
    pub icon_padding_px: f32,
}
impl Default for HudConfig {
    fn default() -> Self {
        Self { scale: 1.0, bottom_margin: 8, icon_padding_px: 2.0 }
    }
}

/// Estado simple de construcción.
pub struct BuildState {
    pub options: Vec<char>,
    pub sel_idx: usize,
    pub ghost_center: Option<Vector3>,
    pub cube_size: Vector3,
    pub ghost_mat: Material,
    pub current_char: char,
    
    // NUEVO: sprites del HUD
    pub hud: Option<HudSprites>,
    pub hud_cfg: HudConfig, 
}

impl BuildState {
    pub fn new(options: Vec<char>, cube_size: Vector3) -> Self {
        let ghost_mat = Material::new(Vector3::new(0.7, 0.85, 1.0), 10.0, [0.95, 0.05, 0.0, 0.0], 0.0);
        let current_char = options.get(0).copied().unwrap_or('X');
        Self {
            options,
            sel_idx: 0,
            ghost_center: None,
            cube_size,
            ghost_mat,
            current_char,
            hud: None, // ← por defecto sin sprites
            hud_cfg: HudConfig::default(),
        }
    }

    /// Creador con sprites del hotbar (íconos deben ir en el mismo orden que `options`).
    pub fn new_with_sprites_and_cfg(
        options: Vec<char>, cube_size: Vector3,
        hotbar: Texture2D, selection: Texture2D, icons: Vec<Texture2D>,
        hud_cfg: HudConfig,
    ) -> Self {
        let mut s = Self::new(options, cube_size);
        s.hud = Some(HudSprites { hotbar, selection, icons });
        s.hud_cfg = hud_cfg;
        s
    }

    #[inline]
    pub fn current_block_char(&self) -> char {
        self.options.get(self.sel_idx).copied().unwrap_or('X')
    }

    pub fn next(&mut self) {
        if !self.options.is_empty() {
            self.sel_idx = (self.sel_idx + 1) % self.options.len();
            self.current_char = self.current_block_char();
        }
    }

    pub fn prev(&mut self) {
        if !self.options.is_empty() {
            if self.sel_idx == 0 { self.sel_idx = self.options.len() - 1; }
            else { self.sel_idx -= 1; }
            self.current_char = self.current_block_char();
        }
    }
}

/// Construye un rayo desde el mouse en espacio cámara → mundo.
pub fn mouse_ray_dir(
    mouse: Vector2, width: f32, height: f32, fov: f32,
    cam_right: Vector3, cam_up: Vector3, cam_forward: Vector3,
) -> Vector3 {
    let aspect = width / height;
    let scale = (fov * 0.5).tan();
    let sx_ndc = (2.0 * (mouse.x)) / width - 1.0;
    let sy_ndc = 1.0 - (2.0 * (mouse.y)) / height;
    let sx = sx_ndc * aspect * scale;
    let sy = sy_ndc * scale;
    let v_cam = Vector3::new(sx, sy, -1.0).normalized();
    Vector3::new(
        v_cam.x * cam_right.x + v_cam.y * cam_up.x - v_cam.z * cam_forward.x,
        v_cam.x * cam_right.y + v_cam.y * cam_up.y - v_cam.z * cam_forward.y,
        v_cam.x * cam_right.z + v_cam.y * cam_up.z - v_cam.z * cam_forward.z,
    ).normalized()
}

#[inline]
fn snap_to_grid_center(p: Vector3, size: Vector3, origin: Vector3) -> Vector3 {
    let rel = p - origin;
    let gx = (rel.x / size.x).floor();
    let gy = (rel.y / size.y).floor();
    let gz = (rel.z / size.z).floor();
    Vector3::new(
        origin.x + (gx + 0.5) * size.x,
        origin.y + (gy + 0.5) * size.y,
        origin.z + (gz + 0.5) * size.z,
    )
}

#[inline]
pub fn face_neighbor_center_from_hit(
    hit_point: Vector3, hit_normal: Vector3, size: Vector3, origin: Vector3,
) -> Vector3 {
    let eps = 1e-4;
    let offset = Vector3::new(
        hit_normal.x.signum() * size.x * 0.5,
        hit_normal.y.signum() * size.y * 0.5,
        hit_normal.z.signum() * size.z * 0.5,
    );
    let nudge = Vector3::new(
        hit_normal.x.signum() * eps,
        hit_normal.y.signum() * eps,
        hit_normal.z.signum() * eps,
    );
    snap_to_grid_center(hit_point + offset + nudge, size, origin)
}

pub fn adjacent_cell_center(hit_point: Vector3, normal: Vector3, cube_size: Vector3, grid_origin: Vector3) -> Vector3 {
    let half = cube_size * 0.5;
    let eps  = Vector3::new(1e-3, 1e-3, 1e-3);
    let push = Vector3::new(
        normal.x.signum() * (half.x + eps.x),
        normal.y.signum() * (half.y + eps.y),
        normal.z.signum() * (half.z + eps.z),
    );
    snap_to_grid_center(hit_point + push, cube_size, grid_origin)
}

pub fn make_block_from_palette(center: Vector3, cube_size: Vector3, tpl: &CubeTemplate) -> Box<dyn RayIntersect> {
    let mut cube = Cube::from_center_size(center, cube_size, tpl.material);
    cube.set_face_textures_from_template(&tpl.face_textures);
    Box::new(cube)
}

pub fn make_ghost(center: Vector3, cube_size: Vector3, ghost_mat: Material) -> Box<dyn RayIntersect> {
    let cube = Cube::from_center_size(center, cube_size, ghost_mat);
    Box::new(cube)
}

pub fn find_object_index_by_center(objects: &[Box<dyn RayIntersect>], center: Vector3) -> Option<usize> {
    for (i, obj) in objects.iter().enumerate() {
        let (mn, mx) = obj.aabb();
        if center.x >= mn.x - 1e-4 && center.x <= mx.x + 1e-4 &&
           center.y >= mn.y - 1e-4 && center.y <= mx.y + 1e-4 &&
           center.z >= mn.z - 1e-4 && center.z <= mx.z + 1e-4 {
            return Some(i);
        }
    }
    None
}

/// ———————————————————————————————————————————————————————————————
/// HUD con sprites estilo Minecraft
/// ———————————————————————————————————————————————————————————————
/// Dibuja hotbar centrado abajo, íconos con margen de 2px y la selección encima.
pub fn draw_hud_hotbar(d: &mut RaylibDrawHandle, state: &BuildState, screen_w: i32, screen_h: i32) {
    let hud = match &state.hud { Some(h) => h, None => { draw_hud_text(d, state); return; } };

    // 1) Parámetros de escala
    let s  = state.hud_cfg.scale.clamp(0.5, 3.0);
    let bm = state.hud_cfg.bottom_margin;
    let pad = state.hud_cfg.icon_padding_px * s; // padding por lado, escalado

    // 2) Tamaños escalados del hotbar
    let hb_w = hud.hotbar.width() as f32 * s;
    let hb_h = hud.hotbar.height() as f32 * s;

    // 3) Posición centrada abajo
    let hb_x = (screen_w as f32 - hb_w) * 0.5;
    let hb_y = (screen_h as f32 - hb_h) - bm as f32;

    // 4) Dibujar hotbar escalado
    let hb_src = Rectangle { x:0.0, y:0.0, width:hud.hotbar.width() as f32, height:hud.hotbar.height() as f32 };
    let hb_dst = Rectangle { x:hb_x, y:hb_y, width:hb_w, height:hb_h };
    d.draw_texture_pro(&hud.hotbar, hb_src, hb_dst, Vector2::zero(), 0.0, Color::WHITE);

    // 5) Geometría de slots a partir del ancho ESCALADO
    let slots = 9usize;
    let pitch = hb_w / slots as f32;                  // distancia entre centros
    let cx0   = hb_x + pitch * 0.5;
    let cy    = hb_y + hb_h * 0.5;

    // 6) Iconos: margen de 2px por lado escalado → restamos 4*pad en total
    let icon_size = (pitch.min(hb_h) - 2.0 * pad * 2.0).max(1.0);

    // 7) Dibujar iconos
    let count = state.options.len().min(slots);
    for i in 0..count {
        if i >= hud.icons.len() { break; }
        let icon = &hud.icons[i];

        let center_x = cx0 + i as f32 * pitch;
        let src = Rectangle { x:0.0, y:0.0, width:icon.width() as f32, height:icon.height() as f32 };
        let dst = Rectangle {
            x:center_x - icon_size * 0.5,
            y:cy - icon_size * 0.5,
            width:icon_size,
            height:icon_size,
        };
        d.draw_texture_pro(icon, src, dst, Vector2::zero(), 0.0, Color::WHITE);
    }

    // 8) Selección, también escalada
    let sel_w = hud.selection.width() as f32 * s;
    let sel_h = hud.selection.height() as f32 * s;
    let sel_cx = cx0 + state.sel_idx.min(slots - 1) as f32 * pitch;
    let sel_cy = cy;

    let sel_src = Rectangle { x:0.0, y:0.0, width:hud.selection.width() as f32, height:hud.selection.height() as f32 };
    let sel_dst = Rectangle { x:sel_cx - sel_w * 0.5, y:sel_cy - sel_h * 0.5, width:sel_w, height:sel_h };
    d.draw_texture_pro(&hud.selection, sel_src, sel_dst, Vector2::zero(), 0.0, Color::WHITE);
}

/// HUD textual (fallback) — lo dejamos por si quieres ver info de depuración.
pub fn draw_hud_text(d: &mut RaylibDrawHandle, state: &BuildState) {
    let x = 12;
    let mut y = 12;
    d.draw_text("BUILDER", x, y, 16, Color::YELLOW);
    y += 22;
    d.draw_text("Bloque [Q/E]:", x, y, 14, Color::LIGHTGRAY);
    y += 18;

    for (idx, ch) in state.options.iter().enumerate() {
        let line = format!("{} {}", if idx == state.sel_idx { "➤" } else { "  " }, ch);
        let col = if idx == state.sel_idx { Color::WHITE } else { Color::GRAY };
        d.draw_text(&line, x, y, 18, col);
        y += 20;
    }

    y += 8;
    d.draw_text("Click izq: colocar", x, y, 14, Color::LIGHTGRAY); y += 16;
    d.draw_text("Click der: quitar",  x, y, 14, Color::LIGHTGRAY); y += 16;
    d.draw_text("J/L/I/K: rotar luz",  x, y, 14, Color::LIGHTGRAY);
}
