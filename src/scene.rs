// scene.rs
use std::{fs, io};

use raylib::prelude::Vector3;

use crate::cube::Cube;
use crate::material::Material;
use crate::palette::{CubeTemplate, Palette};
use crate::ray_intersect::RayIntersect;
use crate::slab::{Slab, SlabHalf, Face as SlabFace};

/// Parámetros para construir la escena a partir de ASCII layers.
pub struct SceneParams {
    pub cube_size: Vector3,
    pub gap: Vector3,
    pub origin: Vector3,
    pub y0: f32,
    pub y_step: f32,
    pub any_non_whitespace_is_solid: bool,
    pub solid_chars: Vec<char>,
}

pub fn load_ascii_layers_with_palette(
    dir: &str,
    params: &SceneParams,
    palette: &Palette,
    default_material: Material,
) -> io::Result<Vec<Box<dyn RayIntersect>>> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.is_file() && p.extension().map(|ext| ext == "txt").unwrap_or(false)
        })
        .collect();

    entries.sort_by_key(|e| e.path());

    let mut objects: Vec<Box<dyn RayIntersect>> = Vec::new();

    for (layer_idx, entry) in entries.into_iter().enumerate() {
        let path = entry.path();
        let text = fs::read_to_string(&path)?;

        let mut lines: Vec<String> = text
            .lines()
            .map(|s| s.trim_end_matches(&['\r', '\n'][..]).to_string())
            .collect();

        while matches!(lines.first(), Some(s) if s.trim().is_empty()) { lines.remove(0); }
        while matches!(lines.last(), Some(s) if s.trim().is_empty()) { lines.pop(); }
        if lines.is_empty() { continue; }

        let rows = lines.len();
        let cols = lines.iter().map(|s| s.chars().count()).max().unwrap_or(0);

        // pasos entre centros: SIN GAPS si gap = 0
        let step_x = params.cube_size.x + params.gap.x;
        let step_z = params.cube_size.z + params.gap.z;

        let half_w = (cols as f32 - 1.0) * 0.5;
        let half_h = (rows as f32 - 1.0) * 0.5;

        let y_center = params.y0 + layer_idx as f32 * params.y_step;

        for (r, line) in lines.iter().enumerate() {
            let mut chars = line.chars().collect::<Vec<char>>();
            if chars.len() < cols { chars.resize(cols, ' '); }

            for c in 0..cols {
                let ch = chars[c];

                // sólido si está en paleta o según flags
                let has_tpl = palette.get(ch).is_some();
                let is_slab = ch == '_' || ch == '-';
                let solid = if params.any_non_whitespace_is_solid {
                    !ch.is_whitespace()
                } else {
                    params.solid_chars.contains(&ch) || has_tpl || is_slab
                };
                if !solid { continue; }

                let x = (c as f32 - half_w) * step_x;
                let z = (r as f32 - half_h) * step_z;
                let center = params.origin + Vector3::new(x, y_center, z);

                if is_slab {
                    // Crear SLAB (mitad baja '_' o mitad alta '-')
                    let half = if ch == '_' { SlabHalf::Bottom } else { SlabHalf::Top };
                    let mut slab = Slab::from_block_center_size(center, params.cube_size, half, default_material);

                    if let Some(tpl) = palette.get(ch) {
                        slab.material = tpl.material;
                        slab.set_face_textures_from_template(&tpl.face_textures);
                    }
                    objects.push(Box::new(slab));
                } else {
                    // Cubo estándar
                    let mut cube = Cube::from_center_size(center, params.cube_size, default_material);
                    if let Some(tpl) = palette.get(ch) {
                        cube.material = tpl.material;
                        cube.set_face_textures_from_template(&tpl.face_textures);
                    }
                    objects.push(Box::new(cube));
                }
            }
        }
    }

    Ok(objects)
}

pub fn default_params(cube_size: Vector3) -> SceneParams {
    SceneParams {
        cube_size,
        gap: Vector3::new(0.0, 0.0, 0.0), // << sin espacios
        origin: Vector3::zero(),
        y0: -cube_size.y * 0.5,
        y_step: cube_size.y,
        any_non_whitespace_is_solid: false,      // << usamos paleta por carácter
        solid_chars: vec!['X', '_', '-'],        // << incluye slabs por defecto
    }
}
