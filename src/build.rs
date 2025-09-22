// build.rs
use raylib::prelude::*;
use crate::material::Material;
use crate::ray_intersect::RayIntersect;
use crate::cube::Cube;
use crate::palette::CubeTemplate;

/// Estado simple de construcción: qué bloque está seleccionado y el "ghost".
pub struct BuildState {
    pub options: Vec<char>,
    pub sel_idx: usize,
    pub ghost_center: Option<Vector3>,
    pub cube_size: Vector3,
    pub ghost_mat: Material,
    pub current_char: char,
}

impl BuildState {
    pub fn new(options: Vec<char>, cube_size: Vector3) -> Self {
        let ghost_mat = Material::new(
            Vector3::new(0.7, 0.85, 1.0), // un celestito
            10.0,
            [0.95, 0.05, 0.0, 0.0],
            0.0,
        );
        let current_char = options.get(0).copied().unwrap_or('X');
        Self {
            options,
            sel_idx: 0,
            ghost_center: None,
            cube_size,
            ghost_mat,
            current_char,
        }
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
    mouse: Vector2,
    width: f32,
    height: f32,
    fov: f32,
    cam_right: Vector3,
    cam_up: Vector3,
    cam_forward: Vector3,
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

/// Redondea un punto a centro de celda de una grilla axis-aligned.
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

/// Centro de la celda ADYACENTE a la cara tocada.
#[inline]
pub fn face_neighbor_center_from_hit(
    hit_point: Vector3,
    hit_normal: Vector3,
    size: Vector3,
    origin: Vector3,
) -> Vector3 {
    // Un pasito hacia el interior del bloque vecino para que el floor caiga seguro en su celda
    let eps = 1e-4;

    // offset hasta el centro del bloque vecino (medio tamaño en el eje de la cara)
    let offset = Vector3::new(
        hit_normal.x.signum() * size.x * 0.5,
        hit_normal.y.signum() * size.y * 0.5,
        hit_normal.z.signum() * size.z * 0.5,
    );

    // pequeño empujón extra para no caer justo en el plano de cara
    let nudge = Vector3::new(
        hit_normal.x.signum() * eps,
        hit_normal.y.signum() * eps,
        hit_normal.z.signum() * eps,
    );

    let p = hit_point + offset + nudge;
    snap_to_grid_center(p, size, origin)
}

/// Dado un punto de impacto y su normal, devuelve el centro del bloque **adyacente**.
pub fn adjacent_cell_center(hit_point: Vector3, normal: Vector3, cube_size: Vector3, grid_origin: Vector3) -> Vector3 {
    let half = cube_size * 0.5;
    let eps  = Vector3::new(1e-3, 1e-3, 1e-3);
    // Empuja solo en el/los ejes marcados por la normal (±1/0)
    let push = Vector3::new(
        normal.x.signum() * (half.x + eps.x),
        normal.y.signum() * (half.y + eps.y),
        normal.z.signum() * (half.z + eps.z),
    );
    let p_out = hit_point + push;
    snap_to_grid_center(p_out, cube_size, grid_origin)
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

/// HUD textual simple.
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
