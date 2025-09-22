use raylib::prelude::*;
use std::f32::consts::PI;

mod framebuffer;
mod ray_intersect;
mod sphere;
mod camera;
mod light;
mod material;
mod cube;
mod slab;
mod texture;
mod scene;
mod palette;
mod accel;
mod build;
mod skybox; // ← NUEVO

use framebuffer::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use camera::Camera;
use light::LightKind;
use material::{Material, vector3_to_color};
use palette::{Palette, CubeTemplate};
use accel::UniformGridAccel;

use crate::texture::Texture;
use crate::build::*;
use crate::skybox::Skybox; // ← NUEVO

const ORIGIN_BIAS: f32 = 1e-3;

#[inline]
fn lerp(a: Vector3, b: Vector3, t: f32) -> Vector3 { a * (1.0 - t) + b * t }

#[inline]
fn smooth5(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// FONDO fallback (si no hay skybox cargado)
fn procedural_sky(dir: Vector3) -> Vector3 {
    let d = dir.normalized();
    let t = ((d.y) * 0.5 + 0.5).clamp(0.0, 1.0);

    let horizon = Vector3::new(0.08, 0.04, 0.12);
    let mid     = Vector3::new(0.03, 0.015, 0.06);
    let top     = Vector3::new(0.015, 0.010, 0.030);

    let c = if t < 0.6 {
        let k = smooth5(t / 0.6);
        lerp(horizon, mid, k)
    } else {
        let k = smooth5((t - 0.6) / 0.4);
        lerp(mid, top, k)
    };

    let h = (1.0 - t).clamp(0.0, 1.0);
    let glow = h.powf(5.0);
    let glow_col = Vector3::new(0.20, 0.05, 0.15);
    let c = c + glow_col * (0.08 * glow);

    let haze = (1.0 - t).powf(2.0) * 0.03;
    let c = c + Vector3::new(haze * 0.6, haze * 0.3, haze);

    Vector3::new(c.x.clamp(0.0, 1.0), c.y.clamp(0.0, 1.0), c.z.clamp(0.0, 1.0))
}

fn offset_origin(intersect: &Intersect, direction: &Vector3) -> Vector3 {
    let offset = intersect.normal * ORIGIN_BIAS;
    if direction.dot(intersect.normal) < 0.0 { intersect.point - offset } else { intersect.point + offset }
}

fn reflect(incident: &Vector3, normal: &Vector3) -> Vector3 {
    *incident - *normal * 2.0 * incident.dot(*normal)
}

fn refract(incident: &Vector3, normal: &Vector3, refractive_index: f32) -> Option<Vector3> {
    let mut cosi = incident.dot(*normal).max(-1.0).min(1.0);
    let mut etai = 1.0;
    let mut etat = refractive_index;
    let mut n = *normal;

    if cosi > 0.0 {
        std::mem::swap(&mut etai, &mut etat);
        n = -n;
    } else {
        cosi = -cosi;
    }

    let eta = etai / etat;
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
    if k < 0.0 { None } else { Some(*incident * eta + n * (eta * cosi - k.sqrt())) }
}

fn cast_shadow(
    intersect: &Intersect,
    light: &light::Light,
    objects: &[Box<dyn RayIntersect>],
    accel: &UniformGridAccel,
) -> f32 {
    let (light_dir, light_distance) = light.at(intersect.point);
    let shadow_ray_origin = offset_origin(intersect, &light_dir);
    if accel.occluded(&shadow_ray_origin, &light_dir, light_distance, objects) { 1.0 } else { 0.0 }
}

// ==== PREVIEW sin objeto “ghost” ====
#[derive(Clone, Copy)]
struct Preview { hovered_idx: usize }

#[inline]
fn sample_background(ray_direction: &Vector3, skybox: Option<&Skybox>) -> Vector3 {
    if let Some(sb) = skybox {
        sb.sample(*ray_direction)
    } else {
        procedural_sky(*ray_direction)
    }
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[Box<dyn RayIntersect>],
    accel: &UniformGridAccel,
    light: &light::Light,
    depth: u32,
    preview: Option<Preview>,     // ← mantiene preview
    skybox: Option<&Skybox>,      // ← NUEVO
) -> Vector3 {
    if depth > 3 {
        return sample_background(ray_direction, skybox);
    }

    let mut intersect = accel.trace(ray_origin, ray_direction, objects);

    // Override del material en el objeto hovered para “preview”
    if let Some(pv) = preview {
        if intersect.is_intersecting && intersect.object_index == Some(pv.hovered_idx) {
            let preview_mat = Material::new(
                Vector3::new(0.9, 0.3, 0.3),
                8.0,
                [1.0, 0.0, 0.0, 0.0],
                1.0
            );
            intersect.material = preview_mat;
            intersect.coverage = 1.0;
        }
    }

    if !intersect.is_intersecting {
        return sample_background(ray_direction, skybox);
    }

    let (light_dir, _light_distance) = light.at(intersect.point);
    let view_dir   = (*ray_origin - intersect.point).normalized();
    let refl_light = reflect(&-light_dir, &intersect.normal).normalized();

    let shadow_intensity = cast_shadow(&intersect, light, objects, accel);
    let light_intensity  = light.intensity * (1.0 - shadow_intensity);

    let light_color_v3 = Vector3::new(
        light.color.r as f32 / 255.0,
        light.color.g as f32 / 255.0,
        light.color.b as f32 / 255.0,
    );

    let diffuse_intensity = ((intersect.normal.dot(light_dir) + 0.3) / 1.3)
        .clamp(0.0, 1.0) * light_intensity;
    let diffuse  = intersect.material.diffuse * diffuse_intensity;

    let specular_intensity = view_dir
        .dot(refl_light)
        .max(0.0)
        .powf(intersect.material.specular) * light_intensity;
    let specular = light_color_v3 * specular_intensity;

    let coverage = intersect.coverage;
    let albedo   = intersect.material.albedo;

    let phong_color =
        (diffuse + intersect.material.diffuse * 0.15) * (albedo[0] * coverage) +
        specular * (albedo[1] * coverage);

    let reflectivity = albedo[2];

    let mut transparency = (1.0 - coverage) + albedo[3] * coverage;
    transparency = transparency.clamp(0.0, 1.0);

    let reflect_color = if reflectivity > 0.0 {
        let rdir = reflect(ray_direction, &intersect.normal).normalized();
        let ro   = offset_origin(&intersect, &rdir);
        cast_ray(&ro, &rdir, objects, accel, light, depth + 1, preview, skybox)
    } else {
        Vector3::zero()
    };

    let refract_color = if transparency > 0.0 {
        if let Some(tdir) = refract(ray_direction, &intersect.normal, intersect.material.refractive_index) {
            let ro = offset_origin(&intersect, &tdir);
            cast_ray(&ro, &tdir, objects, accel, light, depth + 1, preview, skybox)
        } else {
            let rdir = reflect(ray_direction, &intersect.normal).normalized();
            let ro   = offset_origin(&intersect, &rdir);
            cast_ray(&ro, &rdir, objects, accel, light, depth + 1, preview, skybox)
        }
    } else {
        Vector3::zero()
    };

    // Glint especular “mirror-light”
    let mut glint = Vector3::zero();
    let mirror_dir    = reflect(ray_direction, &intersect.normal).normalized();
    let mirror_origin = offset_origin(&intersect, &mirror_dir);

    let hardness_point = 800.0;
    let hardness_dir   = 800.0;
    let gain           = 1.0;
    let refl_bias      = (reflectivity + 0.05).min(1.0);

    match light.kind {
        LightKind::Point => {
            let to_l = light.position - mirror_origin;
            let dist = to_l.length();
            if dist > 0.0 {
                let ldir  = to_l / dist;
                let align = mirror_dir.dot(ldir).max(0.0);
                if align > 0.0 && !accel.occluded(&mirror_origin, &ldir, dist, objects) {
                    let falloff = 1.0 / (1.0 + dist * dist);
                    let s = gain * light.intensity * falloff * align.powf(hardness_point) * refl_bias;
                    glint = light_color_v3 * s;
                }
            }
        }
        LightKind::Directional => {
            let ldir  = -light.direction;
            let align = mirror_dir.dot(ldir).max(0.0);
            if align > 0.0 && !accel.occluded(&mirror_origin, &ldir, f32::INFINITY, objects) {
                let s = gain * light.intensity * align.powf(hardness_dir) * refl_bias;
                glint = light_color_v3 * s;
            }
        }
    }

    let k_phong = (1.0 - reflectivity - transparency).max(0.0);
    phong_color * k_phong + reflect_color * reflectivity + refract_color * transparency + glint
}

pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    accel: &UniformGridAccel,
    camera: &Camera,
    light: &light::Light,
    preview: Option<Preview>,
    skybox: Option<&Skybox>,  // ← NUEVO
) {
    let w = framebuffer.width as usize;
    let h = framebuffer.height as usize;

    let cam = camera.basis();

    let width_f = framebuffer.width as f32;
    let height_f = framebuffer.height as f32;
    let aspect_ratio = width_f / height_f;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let rows_per = (h + threads - 1) / threads;

    let pixels = framebuffer.pixels_mut();

    std::thread::scope(|scope| {
        let mut joins = Vec::with_capacity(threads);
        let mut results: Vec<(usize, Vec<Color>)> = Vec::with_capacity(threads);

        for t in 0..threads {
            let y_start = t * rows_per;
            if y_start >= h { break; }
            let y_end = ((t + 1) * rows_per).min(h);

            let light_c = *light;
            let aspect_ratio_c = aspect_ratio;
            let perspective_scale_c = perspective_scale;
            let width_f_c = width_f;
            let height_f_c = height_f;
            let cam_c = cam;
            let span_w = w;
            let preview_c = preview;
            // Pasamos puntero a skybox (Option) por copia ligera
            let skybox_c = skybox;

            let handle = scope.spawn(move || {
                let span_h = y_end - y_start;
                let mut local = vec![Color::BLACK; span_h * span_w];

                for (row_off, y) in (y_start..y_end).enumerate() {
                    let fy = y as f32;
                    for x in 0..span_w {
                        let fx = x as f32;

                        let mut sx = (2.0 * fx) / width_f_c - 1.0;
                        let mut sy = -(2.0 * fy) / height_f_c + 1.0;

                        sx = sx * aspect_ratio_c * perspective_scale_c;
                        sy = sy * perspective_scale_c;

                        let v_cam = Vector3::new(sx, sy, -1.0).normalized();
                        let ray_dir = Vector3::new(
                            v_cam.x * cam_c.right.x + v_cam.y * cam_c.up.x - v_cam.z * cam_c.forward.x,
                            v_cam.x * cam_c.right.y + v_cam.y * cam_c.up.y - v_cam.z * cam_c.forward.y,
                            v_cam.x * cam_c.right.z + v_cam.y * cam_c.up.z - v_cam.z * cam_c.forward.z,
                        );

                        let rgb = cast_ray(&cam_c.eye, &ray_dir, objects, accel, &light_c, 0, preview_c, skybox_c);
                        local[row_off * span_w + x] = vector3_to_color(rgb);
                    }
                }

                (y_start, local)
            });

            joins.push(handle);
        }

        for j in joins {
            let (y_start, local) = j.join().expect("Hilo de render falló");
            results.push((y_start, local));
        }

        for (y_start, local) in results {
            let span_h = local.len() / w;
            for row_off in 0..span_h {
                let dst_start = (y_start + row_off) * w;
                let src_start = row_off * w;
                pixels[dst_start..dst_start + w]
                    .copy_from_slice(&local[src_start..src_start + w]);
            }
        }
    });
}

#[inline]
fn neighbor_cell_center_from_face_hit(
    hit_point: Vector3,
    hit_normal: Vector3,
    size: Vector3,
    origin: Vector3,
) -> Vector3 {
    let eps = 1e-4;
    let p_inside = hit_point - hit_normal * eps;

    let rel = p_inside - origin;
    let mut ix = (rel.x / size.x).floor() as i32;
    let mut iy = (rel.y / size.y).floor() as i32;
    let mut iz = (rel.z / size.z).floor() as i32;

    if hit_normal.x > 0.0 { ix += 1; } else if hit_normal.x < 0.0 { ix -= 1; }
    if hit_normal.y > 0.0 { iy += 1; } else if hit_normal.y < 0.0 { iy -= 1; }
    if hit_normal.z > 0.0 { iz += 1; } else if hit_normal.z < 0.0 { iz -= 1; }

    Vector3::new(
        origin.x + (ix as f32 + 0.5) * size.x,
        origin.y + (iy as f32 + 0.5) * size.y,
        origin.z + (iz as f32 + 0.5) * size.z,
    )
}

fn main() {
    let window_width = 1300;
    let window_height = 900;

    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raytracer Builder")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    let mut tmp_img = Image::gen_image_color(window_width, window_height, Color::BLACK);
    let texture = window
        .load_texture_from_image(&thread, &tmp_img)
        .expect("No se pudo crear la textura persistente");
    framebuffer.attach_texture(texture);

    // ======= PALETA (MATERIALES) =======
    let stone = Material::new(Vector3::new(0.55, 0.55, 0.55), 20.0, [0.90, 0.10, 0.0, 0.0], 0.0);
    let grass_mat = Material::new(Vector3::new(1.0, 1.0, 1.0), 10.0, [0.95, 0.05, 0.0, 0.0], 0.0);
    let dirt_mat  = Material::new(Vector3::new(1.0, 1.0, 1.0), 8.0,  [0.98, 0.02, 0.0, 0.0], 0.0);
    let log_mat   = Material::new(Vector3::new(1.0, 1.0, 1.0), 15.0, [0.92, 0.08, 0.0, 0.0], 0.0);
    let planks_mat= Material::new(Vector3::new(1.0, 1.0, 1.0), 12.0, [0.90, 0.10, 0.0, 0.0], 0.0);
    let glass_mat = Material::new(Vector3::new(1.0, 1.0, 1.0),120.0,[0.80, 0.15, 0.06, 0.0], 1.5);
    let leaves_mat= Material::new(Vector3::new(1.0, 1.0, 1.0), 35.0, [0.92, 0.08, 0.0, 0.0], 0.0);
    let ice_mat   = Material::new(Vector3::new(1.0, 1.0, 1.0), 10.0, [0.80, 0.10, 0.20, 0.05], 1.31);

    use std::sync::Arc;
    let grass_top    = Arc::new(Texture::from_file("assets/snow_grass/posy.png"));
    let grass_side   = Arc::new(Texture::from_file("assets/snow_grass/posx.png"));
    let grass_bottom = Arc::new(Texture::from_file("assets/snow_grass/negy.png"));
    let dirt_tex     = Arc::new(Texture::from_file("assets/dirt/dirt.png"));

    let log_top     = Arc::new(Texture::from_file("assets/spruce_log/spruce_log_top.png"));
    let log_bottom  = Arc::new(Texture::from_file("assets/spruce_log/spruce_log_top.png"));
    let log_side    = Arc::new(Texture::from_file("assets/spruce_log/spruce_log.png"));

    let planks = Arc::new(Texture::from_file("assets/spruce_planks/spruce_planks.png"));
    let uslab_planks = Arc::new(Texture::from_file("assets/spruce_planks/spruce_planks.png"));
    let lslab_planks = Arc::new(Texture::from_file("assets/spruce_planks/spruce_planks.png"));

    let glass = Arc::new(Texture::from_file("assets/glass/glass.png"));
    let glass_tpl = CubeTemplate::with_same_texture_image_alpha_window(glass_mat, glass.clone(), 0.05);

    let leaves = Arc::new(Texture::from_file("assets/spruce_leaves/spruce_leaves.png"));
    let leaves_tpl = CubeTemplate::with_same_texture_tinted_black_transparent(
        leaves_mat, leaves.clone(), Vector3::new(0.2, 0.6, 0.25), 0.05,
    );

    let ice = Arc::new(Texture::from_file("assets/ice/ice.png"));

    let mut palette = Palette::new();
    palette.set('X', CubeTemplate::with_top_bottom_sides(grass_mat, grass_top, grass_bottom, grass_side));
    palette.set('D', CubeTemplate::with_same_texture(dirt_mat,  dirt_tex));
    palette.set('L', CubeTemplate::with_top_bottom_sides(log_mat,  log_top, log_bottom, log_side));
    palette.set('P', CubeTemplate::with_same_texture(planks_mat,  planks));
    palette.set('G', glass_tpl);
    palette.set('l', leaves_tpl);
    palette.set('H', CubeTemplate::with_same_texture(ice_mat,  ice));
    palette.set('-', CubeTemplate::with_same_texture(planks_mat,  uslab_planks));
    palette.set('_', CubeTemplate::with_same_texture(planks_mat,  lslab_planks));

    // ===== CARGA ESCENA ASCII =====
    let cube_size = Vector3::new(1.0, 1.0, 1.0);
    let mut params = scene::default_params(cube_size);
    params.gap = Vector3::new(0.0, 0.0, 0.0);
    params.origin = Vector3::new(0.0, 0.0, 0.0);
    params.y0 = -0.5;
    params.y_step = 1.0;

    let default_mat = stone;

    // Escena dinámica (mutable)
    let mut objects: Vec<Box<dyn RayIntersect>> =
        scene::load_ascii_layers_with_palette("assets/scene", &params, &palette, default_mat)
            .expect("Error leyendo assets/scene");

    // ===== Aceleración (inicial) =====
    let mut accel = UniformGridAccel::build(&objects, cube_size.x.max(0.01));

    // ===== Cámara =====
    let mut camera = Camera::new(
        Vector3::new(20.0, 10.0, 20.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    camera.set_config(camera::CameraConfig {
        orbit_sensitivity_yaw:   1.0,
        orbit_sensitivity_pitch: 1.0,
        zoom_sensitivity:        0.5,
        min_pitch:  -1.45,
        max_pitch:   1.45,
        min_distance: 0.5,
        max_distance: 2000.0,
    });
    let rotation_speed = PI / 100.0;

    // ===== Luz =====
    let mut light = light::Light::directional(Vector3::new(-1.0, -1.0, 0.3), Color::new(255,255,255,255), 1.2);
    let dir_rot_speed = PI / 300.0;
    let move_speed = 0.15;

    // ===== Skyboxes =====
    // Estructura de carpetas/archivos requerida (ejemplo):
    // assets/skyboxes/sky1/{posx.png,negx.png,posy.png,negy.png,posz.png,negz.png}
    // assets/skyboxes/sky2/{posx.png,negx.png,posy.png,negy.png,posz.png,negz.png}
    let sky1 = Skybox::from_folder("assets/skyboxes/sky1");
    let sky2 = Skybox::from_folder("assets/skyboxes/sky2");
    let skyboxes = vec![sky1, sky2];
    let mut current_skybox: usize = 0; // 0 = sky1, 1 = sky2

    // ===== Builder HUD/estado =====
    let options = vec!['X', 'D', 'L', 'P', 'G', 'l', 'H'];

    let hotbar_tex = window
        .load_texture(&thread, "assets/ui/hotbar.png")
        .expect("No se pudo cargar assets/ui/hotbar.png");
    let hotbar_sel_tex = window
        .load_texture(&thread, "assets/ui/hotbar_selection.png")
        .expect("No se pudo cargar assets/ui/hotbar_selection.png");

    let icon_paths = vec![
        "assets/snow_grass/posy.png",
        "assets/dirt/dirt.png",
        "assets/spruce_log/spruce_log_top.png",
        "assets/spruce_planks/spruce_planks.png",
        "assets/glass/glass.png",
        "assets/spruce_leaves/spruce_leaves.png",
        "assets/ice/ice.png",
    ];
    let mut icons: Vec<Texture2D> = Vec::with_capacity(icon_paths.len());
    for p in icon_paths {
        icons.push(window.load_texture(&thread, p).expect(&format!("No se pudo cargar {}", p)));
    }

    let hud_cfg = build::HudConfig { scale: 2.6, bottom_margin: 10, icon_padding_px: 1.0 };

    let mut builder = BuildState::new_with_sprites_and_cfg(
        options,
        cube_size,
        hotbar_tex,
        hotbar_sel_tex,
        icons,
        hud_cfg
    );
    let grid_origin = params.origin;

    while !window.window_should_close() {
        // ====== INPUT Cámara ======
        if window.is_key_down(KeyboardKey::KEY_LEFT)  { camera.orbit( rotation_speed, 0.0); }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) { camera.orbit(-rotation_speed, 0.0); }
        if window.is_key_down(KeyboardKey::KEY_DOWN)  { camera.orbit(0.0, -rotation_speed); }
        if window.is_key_down(KeyboardKey::KEY_UP)    { camera.orbit(0.0,  rotation_speed); }
        if window.is_key_down(KeyboardKey::KEY_PAGE_UP)   { camera.zoom(-0.5); }
        if window.is_key_down(KeyboardKey::KEY_PAGE_DOWN) { camera.zoom( 0.5); }

        if window.is_key_pressed(KeyboardKey::KEY_ONE) { light.kind = LightKind::Point; }
        if window.is_key_pressed(KeyboardKey::KEY_TWO) { light.kind = LightKind::Directional; }

        // Cambiar skybox con 3/4
        if window.is_key_pressed(KeyboardKey::KEY_THREE) { current_skybox = 0; }
        if window.is_key_pressed(KeyboardKey::KEY_FOUR)  { current_skybox = 1; }

        if matches!(light.kind, LightKind::Directional) {
            if window.is_key_down(KeyboardKey::KEY_J) { light.yaw_pitch( dir_rot_speed, 0.0); }
            if window.is_key_down(KeyboardKey::KEY_L) { light.yaw_pitch(-dir_rot_speed, 0.0); }
            if window.is_key_down(KeyboardKey::KEY_I) { light.yaw_pitch(0.0,  dir_rot_speed); }
            if window.is_key_down(KeyboardKey::KEY_K) { light.yaw_pitch(0.0, -dir_rot_speed); }
        }
        if matches!(light.kind, LightKind::Point) {
            if window.is_key_down(KeyboardKey::KEY_W) { light.translate(Vector3::new( 0.0, 0.0, -move_speed)); }
            if window.is_key_down(KeyboardKey::KEY_S) { light.translate(Vector3::new( 0.0, 0.0,  move_speed)); }
            if window.is_key_down(KeyboardKey::KEY_A) { light.translate(Vector3::new(-move_speed, 0.0, 0.0)); }
            if window.is_key_down(KeyboardKey::KEY_D) { light.translate(Vector3::new( move_speed, 0.0, 0.0)); }
            if window.is_key_down(KeyboardKey::KEY_R) { light.translate(Vector3::new( 0.0,  move_speed, 0.0)); }
            if window.is_key_down(KeyboardKey::KEY_F) { light.translate(Vector3::new( 0.0, -move_speed, 0.0)); }
        }

        // ====== INPUT Builder ======
        if window.is_key_pressed(KeyboardKey::KEY_Q) { builder.prev(); }
        if window.is_key_pressed(KeyboardKey::KEY_E) { builder.next(); }

        // ====== PICK / PREVIEW ======
        let mouse = window.get_mouse_position();
        let basis = camera.basis();
        let fov = PI / 3.0;
        let ray_dir = mouse_ray_dir(
            mouse,
            window_width as f32,
            window_height as f32,
            fov,
            basis.right, basis.up, basis.forward,
        );
        let ray_origin = basis.eye;

        let hit = accel.trace(&ray_origin, &ray_dir, &objects);

        let mut preview: Option<Preview> = None;
        if hit.is_intersecting {
            if let Some(idx) = hit.object_index {
                preview = Some(Preview { hovered_idx: idx });
            }
        }

        // Colocación/Eliminación
        if hit.is_intersecting {
            let target_center = neighbor_cell_center_from_face_hit(
                hit.point, hit.normal, builder.cube_size, grid_origin
            );

            if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                if let Some(tpl) = palette.get(builder.current_block_char()) {
                    let block = make_block_from_palette(target_center, builder.cube_size, tpl);
                    objects.push(block);
                    accel = UniformGridAccel::build(&objects, cube_size.x.max(0.01));
                }
            }

            if window.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                if let Some(idx) = hit.object_index {
                    if idx < objects.len() {
                        objects.swap_remove(idx);
                        accel = UniformGridAccel::build(&objects, cube_size.x.max(0.01));
                    }
                }
            }
        }

        // ===== Render =====
        framebuffer.clear();
        let sky_ref = Some(&skyboxes[current_skybox]);
        render(&mut framebuffer, &objects, &accel, &camera, &light, preview, sky_ref);

        framebuffer.swap_buffers_with(&mut window, &thread, |d| {
            draw_hud_hotbar(d, &builder, window_width, window_height);

            // Tip de control (opcional)
            d.draw_text("Light [1:Point, 2:Dir]   Skybox [3:Sky1, 4:Sky2]", 12, window_height - 40, 14, Color::LIGHTGRAY);
        });
    }
}
