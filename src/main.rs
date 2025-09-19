use raylib::prelude::*;
use std::f32::consts::PI;
use std::sync::Arc;

mod framebuffer;
mod ray_intersect;
mod sphere;
mod camera;
mod light;
mod material;
mod cube;
mod texture;
mod scene;
mod palette;
mod accel;

use framebuffer::Framebuffer;
use ray_intersect::{Intersect, RayIntersect};
use camera::Camera;
use light::{Light, LightKind};
use material::{Material, vector3_to_color};
use palette::{Palette, CubeTemplate};
use accel::UniformGridAccel;

use crate::texture::Texture;

const ORIGIN_BIAS: f32 = 1e-3;

#[inline]
fn lerp(a: Vector3, b: Vector3, t: f32) -> Vector3 { a * (1.0 - t) + b * t }

#[inline]
fn smooth5(t: f32) -> f32 {
    // smoothstep quintico (suave y sin bandas)
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn procedural_sky(dir: Vector3) -> Vector3 {
    let d = dir.normalized();
    // t=0 cerca del horizonte, t=1 en el cénit
    let t = ((d.y) * 0.5 + 0.5).clamp(0.0, 1.0);

    // Paleta "Nocturne Violet" (morado-negro)
    let horizon = Vector3::new(0.08, 0.04, 0.12); // más claro en horizonte
    let mid     = Vector3::new(0.03, 0.015, 0.06);
    let top     = Vector3::new(0.015, 0.010, 0.030); // casi negro con tinte violeta

    // Gradiente en dos tramos con curvas suaves
    let c = if t < 0.6 {
        let k = smooth5(t / 0.6);            // horizonte -> medio
        lerp(horizon, mid, k)
    } else {
        let k = smooth5((t - 0.6) / 0.4);    // medio -> cénit
        lerp(mid, top, k)
    };

    // Leve brillo/magia de horizonte (muy sutil y oscuro, no “blanco”)
    let h = (1.0 - t).clamp(0.0, 1.0);           // 1 cerca del horizonte
    let glow = h.powf(5.0);                      // curva agresiva para que solo afecte abajo
    let glow_col = Vector3::new(0.20, 0.05, 0.15); // magenta oscuro
    let c = c + glow_col * (0.08 * glow);        // intensidad pequeña

    // Pequeña “niebla” atmosférica violeta en bajo ángulo (evita sky totalmente plano)
    let haze = (1.0 - t).powf(2.0) * 0.03;
    let c = c + Vector3::new(haze * 0.6, haze * 0.3, haze);

    // clamp final a [0,1]
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
    light: &Light,
    objects: &[Box<dyn RayIntersect>],
    accel: &UniformGridAccel,
) -> f32 {
    let (light_dir, light_distance) = light.at(intersect.point);
    let shadow_ray_origin = offset_origin(intersect, &light_dir);
    if accel.occluded(&shadow_ray_origin, &light_dir, light_distance, objects) { 1.0 } else { 0.0 }
}

pub fn cast_ray(
    ray_origin: &Vector3,
    ray_direction: &Vector3,
    objects: &[Box<dyn RayIntersect>],
    accel: &UniformGridAccel,
    light: &Light,
    depth: u32,
) -> Vector3 {
    if depth > 3 { return procedural_sky(*ray_direction); }

    let mut intersect = accel.trace(ray_origin, ray_direction, objects);

    if !intersect.is_intersecting {
        return procedural_sky(*ray_direction);
    }

    let (light_dir, _light_distance) = light.at(intersect.point);
    let view_dir  = (*ray_origin - intersect.point).normalized();
    let reflect_dir = reflect(&-light_dir, &intersect.normal).normalized();

    let shadow_intensity = cast_shadow(&intersect, light, objects, accel);
    let light_intensity = light.intensity * (1.0 - shadow_intensity);

    // Half-Lambert (k=0.3) + ambient 0.15
    let diffuse_intensity = ((intersect.normal.dot(light_dir) + 0.3) / 1.3).clamp(0.0, 1.0) * light_intensity;
    let diffuse = intersect.material.diffuse * diffuse_intensity;

    let specular_intensity = view_dir.dot(reflect_dir).max(0.0).powf(intersect.material.specular) * light_intensity;
    let light_color_v3 = Vector3::new(
        light.color.r as f32 / 255.0,
        light.color.g as f32 / 255.0,
        light.color.b as f32 / 255.0
    );
    let specular = light_color_v3 * specular_intensity;

    let albedo = intersect.material.albedo;
    let phong_color = (diffuse + intersect.material.diffuse * 0.15) * albedo[0] + specular * albedo[1];

    // Reflections
    let reflectivity = intersect.material.albedo[2];
    let reflect_color = if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
        let reflect_origin = offset_origin(&intersect, &reflect_dir);
        cast_ray(&reflect_origin, &reflect_dir, objects, accel, light, depth + 1)
    } else { Vector3::zero() };

    // Refractions
    let transparency = intersect.material.albedo[3];
    let refract_color = if transparency > 0.0 {
        if let Some(refract_dir) = refract(ray_direction, &intersect.normal, intersect.material.refractive_index) {
            let refract_origin = offset_origin(&intersect, &refract_dir);
            cast_ray(&refract_origin, &refract_dir, objects, accel, light, depth + 1)
        } else {
            let reflect_dir = reflect(ray_direction, &intersect.normal).normalized();
            let reflect_origin = offset_origin(&intersect, &reflect_dir);
            cast_ray(&reflect_origin, &reflect_dir, objects, accel, light, depth + 1)
        }
    } else { Vector3::zero() };

    phong_color * (1.0 - reflectivity - transparency) + reflect_color * reflectivity + refract_color * transparency
}

pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    accel: &UniformGridAccel,
    camera: &Camera,
    light: &Light,
) {
    let w = framebuffer.width as usize;
    let h = framebuffer.height as usize;

    // Snapshot de la base de cámara para lectura en hilos
    let cam = camera.basis();

    let width_f = framebuffer.width as f32;
    let height_f = framebuffer.height as f32;
    let aspect_ratio = width_f / height_f;
    let fov = PI / 3.0;
    let perspective_scale = (fov * 0.5).tan();

    // ¿Cuántos hilos usar?
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let rows_per = (h + threads - 1) / threads;

    // Buffer final (de destino) accesible tras juntar resultados
    let pixels = framebuffer.pixels_mut();

    // Scoped threads: pueden tomar prestado &accel y &objects sin 'static
    std::thread::scope(|scope| {
        // Guardamos los join handles y cada bloque local
        let mut joins = Vec::with_capacity(threads);
        // Contenedor para resultados por hilo (y_start, bloque)
        let mut results: Vec<(usize, Vec<Color>)> = Vec::with_capacity(threads);

        for t in 0..threads {
            let y_start = t * rows_per;
            if y_start >= h { break; }
            let y_end = ((t + 1) * rows_per).min(h);

            // Capturas por copia (baratas)
            let light_c = *light; // Light es Copy
            let aspect_ratio_c = aspect_ratio;
            let perspective_scale_c = perspective_scale;
            let width_f_c = width_f;
            let height_f_c = height_f;
            let cam_c = cam;
            let span_w = w;

            // Reservamos un buffer local por hilo
            // Nota: lo crearemos *dentro* del hilo para no mover ownership raro aquí.
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
                        // base change manual evitando método sobre &self
                        let ray_dir = Vector3::new(
                            v_cam.x * cam_c.right.x + v_cam.y * cam_c.up.x - v_cam.z * cam_c.forward.x,
                            v_cam.x * cam_c.right.y + v_cam.y * cam_c.up.y - v_cam.z * cam_c.forward.y,
                            v_cam.x * cam_c.right.z + v_cam.y * cam_c.up.z - v_cam.z * cam_c.forward.z,
                        );

                        // Usamos &accel y &objects prestados del scope
                        let rgb = cast_ray(&cam_c.eye, &ray_dir, objects, accel, &light_c, 0);
                        local[row_off * span_w + x] = vector3_to_color(rgb);
                    }
                }

                (y_start, local)
            });

            joins.push(handle);
        }

        // Recogemos resultados (join implícito al final del scope, pero queremos orden)
        for j in joins {
            let (y_start, local) = j.join().expect("Hilo de render falló");
            results.push((y_start, local));
        }

        // Copiamos cada bloque local a su sitio en el framebuffer
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

fn main() {
    let window_width = 1300;
    let window_height = 900;
 
    let (mut window, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Raytracer Example")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    let mut framebuffer = Framebuffer::new(window_width as u32, window_height as u32);

    let mut tmp_img = Image::gen_image_color(window_width, window_height, Color::BLACK);
    let texture = window
        .load_texture_from_image(&thread, &tmp_img)
        .expect("No se pudo crear la textura persistente");
    framebuffer.attach_texture(texture);

    // ======= PALETA =======
    let stone = Material::new(
        Vector3::new(0.55, 0.55, 0.55),
        25.0,
        [0.85, 0.15, 0.0, 0.0],
        0.0,
    );
    let grass_mat = Material::new(
        Vector3::new(1.0, 1.0, 1.0),
        20.0,
        [0.9, 0.1, 0.0, 0.0],
        0.0,
    );
    let crate_mat = Material::new(
        Vector3::new(1.0, 1.0, 1.0),
        30.0,
        [0.8, 0.2, 0.0, 0.0],
        0.0,
    );

    use std::sync::Arc;
    let grass_top   = Arc::new(Texture::from_file("assets/snow_grass/posy.png"));
    let grass_side  = Arc::new(Texture::from_file("assets/snow_grass/posx.png"));
    let grass_bottom= Arc::new(Texture::from_file("assets/snow_grass/negy.png"));
    let dirt_tex    = Arc::new(Texture::from_file("assets/dirt/dirt.png"));

    let log_top     = Arc::new(Texture::from_file("assets/spruce_log/spruce_log_top.png"));
    let log_bottom  = Arc::new(Texture::from_file("assets/spruce_log/spruce_log_top.png"));
    let log_side    = Arc::new(Texture::from_file("assets/spruce_log/spruce_log.png"));

    let planks = Arc::new(Texture::from_file("assets/spruce_planks/spruce_planks.png"));
    
    let glass = Arc::new(Texture::from_file("assets/glass/glass.png"));

    let leaves = Arc::new(Texture::from_file("assets/spruce_leaves/spruce_leaves.png"));

    let ice = Arc::new(Texture::from_file("assets/ice/ice.png"));

    let mut palette = Palette::new();
    //palette.set('G', CubeTemplate::material_only(stone));
    palette.set('X', CubeTemplate::with_top_bottom_sides(grass_mat, grass_top, grass_bottom, grass_side));
    palette.set('D', CubeTemplate::with_same_texture(crate_mat,  dirt_tex));
    palette.set('L', CubeTemplate::with_top_bottom_sides(crate_mat,  log_top, log_bottom, log_side));
    palette.set('P', CubeTemplate::with_same_texture(crate_mat,  planks));
    palette.set('G', CubeTemplate::with_same_texture(crate_mat,  glass));
    palette.set('l', CubeTemplate::with_same_texture(crate_mat,  leaves));
    palette.set('H', CubeTemplate::with_same_texture(crate_mat,  ice));

    // ===== CARGA ESCENA ASCII SIN GAPS =====
    let cube_size = Vector3::new(1.0, 1.0, 1.0);
    let mut params = scene::default_params(cube_size);
    params.gap = Vector3::new(0.0, 0.0, 0.0);
    params.origin = Vector3::new(0.0, 0.0, 0.0);
    params.y0 = -0.5;
    params.y_step = 1.0;

    let default_mat = stone;

    let objects: Vec<Box<dyn RayIntersect>> =
        scene::load_ascii_layers_with_palette("assets/scene", &params, &palette, default_mat)
            .expect("Error leyendo assets/scene");

    // ===== Aceleración por grilla =====
    let accel = UniformGridAccel::build(&objects, cube_size.x.max(0.01));

    // Cámara
    let mut camera = Camera::new(
        Vector3::new(0.0, 1.5, 5.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );

    // --- NUEVO: configura la cámara orbital de forma centralizada
    camera.set_config(camera::CameraConfig {
        orbit_sensitivity_yaw:   1.0,   // súbele si quieres girar más rápido
        orbit_sensitivity_pitch: 1.0,
        zoom_sensitivity:        0.5,   // controla “lo que avanza” cada paso de zoom
        min_pitch:  -1.45, // -83°
        max_pitch:   1.45, //  83°
        min_distance: 0.5,  // mínimo acercamiento
        max_distance: 2000.0, // MUY grande pero manejable
    });
    let rotation_speed = PI / 100.0; // puedes mantener esta constante para inputs discretos

    // ===== Luz (mut) =====
    // Cambia a Point si prefieres:
    // let mut light = Light::new(Vector3::new(1.0, -1.0, 5.0), Color::WHITE, 1.5);
    let mut light = Light::directional(Vector3::new(-1.0, -1.0, 0.3), Color::new(255,255,255,255), 1.2);

    // Controles
    let dir_rot_speed = PI / 300.0;
    let move_speed = 0.15;

    while !window.window_should_close() {
        // Cámara orbit
        if window.is_key_down(KeyboardKey::KEY_LEFT)  { camera.orbit( rotation_speed, 0.0); }
        if window.is_key_down(KeyboardKey::KEY_RIGHT) { camera.orbit(-rotation_speed, 0.0); }
        if window.is_key_down(KeyboardKey::KEY_DOWN)    { camera.orbit(0.0, -rotation_speed); }
        if window.is_key_down(KeyboardKey::KEY_UP)  { camera.orbit(0.0,  rotation_speed); }

        if window.is_key_down(KeyboardKey::KEY_PAGE_UP)   { camera.zoom(-0.5); }
        if window.is_key_down(KeyboardKey::KEY_PAGE_DOWN) { camera.zoom( 0.5); }

        // Toggle tipo de luz
        if window.is_key_pressed(KeyboardKey::KEY_ONE) { light.kind = LightKind::Point; }
        if window.is_key_pressed(KeyboardKey::KEY_TWO) { light.kind = LightKind::Directional; }

        // Rotar dirección (direccional): J/L (yaw), I/K (pitch)
        if matches!(light.kind, LightKind::Directional) {
            if window.is_key_down(KeyboardKey::KEY_J) { light.yaw_pitch( dir_rot_speed, 0.0); }
            if window.is_key_down(KeyboardKey::KEY_L) { light.yaw_pitch(-dir_rot_speed, 0.0); }
            if window.is_key_down(KeyboardKey::KEY_I) { light.yaw_pitch(0.0,  dir_rot_speed); }
            if window.is_key_down(KeyboardKey::KEY_K) { light.yaw_pitch(0.0, -dir_rot_speed); }
        }

        // Mover posición (puntual): WASD + R/F
        if matches!(light.kind, LightKind::Point) {
            if window.is_key_down(KeyboardKey::KEY_W) { light.translate(Vector3::new( 0.0, 0.0, -move_speed)); }
            if window.is_key_down(KeyboardKey::KEY_S) { light.translate(Vector3::new( 0.0, 0.0,  move_speed)); }
            if window.is_key_down(KeyboardKey::KEY_A) { light.translate(Vector3::new(-move_speed, 0.0, 0.0)); }
            if window.is_key_down(KeyboardKey::KEY_D) { light.translate(Vector3::new( move_speed, 0.0, 0.0)); }
            if window.is_key_down(KeyboardKey::KEY_R) { light.translate(Vector3::new( 0.0,  move_speed, 0.0)); }
            if window.is_key_down(KeyboardKey::KEY_F) { light.translate(Vector3::new( 0.0, -move_speed, 0.0)); }
        }

        framebuffer.clear();
        render(&mut framebuffer, &objects, &accel, &camera, &light);
        framebuffer.swap_buffers(&mut window, &thread);
    }
}
