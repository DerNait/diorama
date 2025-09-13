// accel.rs
use raylib::prelude::Vector3;
use crate::ray_intersect::{Intersect, RayIntersect};

#[derive(Clone, Copy)]
struct Aabb { min: Vector3, max: Vector3 }

impl Aabb {
    fn union(a: Aabb, b: Aabb) -> Aabb {
        Aabb {
            min: Vector3::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y), a.min.z.min(b.min.z)),
            max: Vector3::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y), a.max.z.max(b.max.z)),
        }
    }
    fn intersect_ray(&self, ro: Vector3, rd: Vector3) -> Option<(f32, f32)> {
        let inv = Vector3::new(1.0/rd.x, 1.0/rd.y, 1.0/rd.z);

        let mut t1 = (self.min.x - ro.x)*inv.x;
        let mut t2 = (self.max.x - ro.x)*inv.x;
        if t1>t2 { std::mem::swap(&mut t1, &mut t2); }

        let mut ty1 = (self.min.y - ro.y)*inv.y;
        let mut ty2 = (self.max.y - ro.y)*inv.y;
        if ty1>ty2 { std::mem::swap(&mut ty1, &mut ty2); }

        if t1>ty2 || ty1>t2 { return None; }
        if ty1>t1 { t1=ty1; }
        if ty2<t2 { t2=ty2; }

        let mut tz1 = (self.min.z - ro.z)*inv.z;
        let mut tz2 = (self.max.z - ro.z)*inv.z;
        if tz1>tz2 { std::mem::swap(&mut tz1, &mut tz2); }

        if t1>tz2 || tz1>t2 { return None; }
        if tz1>t1 { t1=tz1; }
        if tz2<t2 { t2=tz2; }

        Some((t1, t2))
    }
}

pub struct UniformGridAccel {
    bounds: Aabb,
    dims: [i32; 3],
    cell_size: Vector3,
    cells: Vec<Vec<usize>>, // por celda, índices a objects[]
}

impl UniformGridAccel {
    pub fn build(objects: &[Box<dyn RayIntersect>], desired_cell_size: f32) -> Self {
        // 1) Bounds globales
        let mut bounds = Aabb {
            min: Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
            max: Vector3::new(-f32::INFINITY, -f32::INFINITY, -f32::INFINITY),
        };
        let mut aabbs: Vec<Aabb> = Vec::with_capacity(objects.len());
        for obj in objects.iter() {
            let (mn, mx) = obj.aabb();
            let a = Aabb { min: mn, max: mx };
            aabbs.push(a);
            bounds = Aabb::union(bounds, a);
        }
        let pad = 1e-4;
        bounds.min = bounds.min - Vector3::new(pad, pad, pad);
        bounds.max = bounds.max + Vector3::new(pad, pad, pad);

        // 2) Dims
        let ext = bounds.max - bounds.min;
        let mut nx = (ext.x / desired_cell_size).ceil() as i32;
        let mut ny = (ext.y / desired_cell_size).ceil() as i32;
        let mut nz = (ext.z / desired_cell_size).ceil() as i32;
        nx = nx.max(1); ny = ny.max(1); nz = nz.max(1);

        let dims = [nx, ny, nz];
        let cell_size = Vector3::new(ext.x / nx as f32, ext.y / ny as f32, ext.z / nz as f32);
        let total = (nx as usize) * (ny as usize) * (nz as usize);
        let mut cells: Vec<Vec<usize>> = (0..total).map(|_| Vec::new()).collect();

        // 3) Insertar cada objeto en las celdas que toca
        for (i, a) in aabbs.iter().enumerate() {
            let min_ix = ((a.min.x - bounds.min.x) / cell_size.x).floor() as i32;
            let min_iy = ((a.min.y - bounds.min.y) / cell_size.y).floor() as i32;
            let min_iz = ((a.min.z - bounds.min.z) / cell_size.z).floor() as i32;
            let max_ix = ((a.max.x - bounds.min.x) / cell_size.x).floor() as i32;
            let max_iy = ((a.max.y - bounds.min.y) / cell_size.y).floor() as i32;
            let max_iz = ((a.max.z - bounds.min.z) / cell_size.z).floor() as i32;

            for iz in min_iz.max(0)..=max_iz.min(nz - 1) {
                for iy in min_iy.max(0)..=max_iy.min(ny - 1) {
                    for ix in min_ix.max(0)..=max_ix.min(nx - 1) {
                        let idx = ((iz * ny + iy) * nx + ix) as usize;
                        cells[idx].push(i);
                    }
                }
            }
        }

        UniformGridAccel { bounds, dims, cell_size, cells }
    }

    #[inline] fn cell_index(&self, ix: i32, iy: i32, iz: i32) -> usize {
        ((iz * self.dims[1] + iy) * self.dims[0] + ix) as usize
    }

    /// DDA estilo Amanatides & Woo: tMax son **tiempos absolutos**, tDelta es el incremento por celda.
    pub fn trace(&self, ro: &Vector3, rd: &Vector3, objects: &[Box<dyn RayIntersect>]) -> Intersect {
        let (mut t_enter, t_exit) = match self.bounds.intersect_ray(*ro, *rd) {
            Some(t) => t, None => return Intersect::empty(),
        };
        if t_exit < 0.0 { return Intersect::empty(); }
        if t_enter < 0.0 { t_enter = 0.0; }

        let eps = 1e-4;
        let pos = *ro + *rd * t_enter;

        // celda inicial
        let mut ix = ((pos.x - self.bounds.min.x) / self.cell_size.x).floor() as i32;
        let mut iy = ((pos.y - self.bounds.min.y) / self.cell_size.y).floor() as i32;
        let mut iz = ((pos.z - self.bounds.min.z) / self.cell_size.z).floor() as i32;
        ix = ix.clamp(0, self.dims[0]-1);
        iy = iy.clamp(0, self.dims[1]-1);
        iz = iz.clamp(0, self.dims[2]-1);

        // pasos y tiempos a la siguiente pared (absolutos)
        let step_x = if rd.x > 0.0 { 1 } else if rd.x < 0.0 { -1 } else { 0 };
        let step_y = if rd.y > 0.0 { 1 } else if rd.y < 0.0 { -1 } else { 0 };
        let step_z = if rd.z > 0.0 { 1 } else if rd.z < 0.0 { -1 } else { 0 };

        let next_x = self.bounds.min.x + (ix + (step_x > 0) as i32) as f32 * self.cell_size.x;
        let next_y = self.bounds.min.y + (iy + (step_y > 0) as i32) as f32 * self.cell_size.y;
        let next_z = self.bounds.min.z + (iz + (step_z > 0) as i32) as f32 * self.cell_size.z;

        let mut t_max_x = if step_x != 0 { t_enter + (next_x - pos.x) / rd.x } else { f32::INFINITY };
        let mut t_max_y = if step_y != 0 { t_enter + (next_y - pos.y) / rd.y } else { f32::INFINITY };
        let mut t_max_z = if step_z != 0 { t_enter + (next_z - pos.z) / rd.z } else { f32::INFINITY };

        let t_delta_x = if step_x != 0 { self.cell_size.x / rd.x.abs() } else { f32::INFINITY };
        let t_delta_y = if step_y != 0 { self.cell_size.y / rd.y.abs() } else { f32::INFINITY };
        let t_delta_z = if step_z != 0 { self.cell_size.z / rd.z.abs() } else { f32::INFINITY };

        let mut best = Intersect::empty();
        let mut best_t = f32::INFINITY;

        loop {
            // probar objetos en la celda
            let cell_idx = self.cell_index(ix, iy, iz);
            for &obj_idx in &self.cells[cell_idx] {
                let i = objects[obj_idx].ray_intersect(ro, rd);
                if i.is_intersecting && i.distance >= t_enter - eps && i.distance < best_t {
                    best_t = i.distance;
                    best = i;
                }
            }

            // Si el hit ocurre antes de salir de la celda actual, listo
            let t_cell_exit = t_max_x.min(t_max_y).min(t_max_z);
            if best_t <= t_cell_exit { break; }

            // Avanzar a la siguiente celda en el eje con menor tMax
            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    ix += step_x; if ix < 0 || ix >= self.dims[0] { break; }
                    t_enter = t_max_x; t_max_x += t_delta_x;
                } else {
                    iz += step_z; if iz < 0 || iz >= self.dims[2] { break; }
                    t_enter = t_max_z; t_max_z += t_delta_z;
                }
            } else {
                if t_max_y < t_max_z {
                    iy += step_y; if iy < 0 || iy >= self.dims[1] { break; }
                    t_enter = t_max_y; t_max_y += t_delta_y;
                } else {
                    iz += step_z; if iz < 0 || iz >= self.dims[2] { break; }
                    t_enter = t_max_z; t_max_z += t_delta_z;
                }
            }
            if t_enter > t_exit { break; }
        }

        best
    }

    /// Sombra: true si hay intersección antes de `max_t`
    pub fn occluded(&self, ro: &Vector3, rd: &Vector3, max_t: f32, objects: &[Box<dyn RayIntersect>]) -> bool {
        let (mut t_enter, t_exit) = match self.bounds.intersect_ray(*ro, *rd) {
            Some(t) => t, None => return false,
        };
        if t_exit < 0.0 { return false; }
        if t_enter < 0.0 { t_enter = 0.0; }
        let eps = 1e-4;
        let pos = *ro + *rd * t_enter;

        let mut ix = ((pos.x - self.bounds.min.x) / self.cell_size.x).floor() as i32;
        let mut iy = ((pos.y - self.bounds.min.y) / self.cell_size.y).floor() as i32;
        let mut iz = ((pos.z - self.bounds.min.z) / self.cell_size.z).floor() as i32;
        ix = ix.clamp(0, self.dims[0]-1);
        iy = iy.clamp(0, self.dims[1]-1);
        iz = iz.clamp(0, self.dims[2]-1);

        let step_x = if rd.x > 0.0 { 1 } else if rd.x < 0.0 { -1 } else { 0 };
        let step_y = if rd.y > 0.0 { 1 } else if rd.y < 0.0 { -1 } else { 0 };
        let step_z = if rd.z > 0.0 { 1 } else if rd.z < 0.0 { -1 } else { 0 };

        let next_x = self.bounds.min.x + (ix + (step_x > 0) as i32) as f32 * self.cell_size.x;
        let next_y = self.bounds.min.y + (iy + (step_y > 0) as i32) as f32 * self.cell_size.y;
        let next_z = self.bounds.min.z + (iz + (step_z > 0) as i32) as f32 * self.cell_size.z;

        let mut t_max_x = if step_x != 0 { t_enter + (next_x - pos.x) / rd.x } else { f32::INFINITY };
        let mut t_max_y = if step_y != 0 { t_enter + (next_y - pos.y) / rd.y } else { f32::INFINITY };
        let mut t_max_z = if step_z != 0 { t_enter + (next_z - pos.z) / rd.z } else { f32::INFINITY };

        let t_delta_x = if step_x != 0 { self.cell_size.x / rd.x.abs() } else { f32::INFINITY };
        let t_delta_y = if step_y != 0 { self.cell_size.y / rd.y.abs() } else { f32::INFINITY };
        let t_delta_z = if step_z != 0 { self.cell_size.z / rd.z.abs() } else { f32::INFINITY };

        loop {
            let cell_idx = self.cell_index(ix, iy, iz);
            for &obj_idx in &self.cells[cell_idx] {
                let i = objects[obj_idx].ray_intersect(ro, rd);
                if i.is_intersecting && i.distance > eps && i.distance < max_t {
                    return true;
                }
            }

            let t_cell_exit = t_max_x.min(t_max_y).min(t_max_z);
            if t_cell_exit >= max_t { break; }

            if t_max_x < t_max_y {
                if t_max_x < t_max_z {
                    ix += step_x; if ix < 0 || ix >= self.dims[0] { break; }
                    t_enter = t_max_x; t_max_x += t_delta_x;
                } else {
                    iz += step_z; if iz < 0 || iz >= self.dims[2] { break; }
                    t_enter = t_max_z; t_max_z += t_delta_z;
                }
            } else {
                if t_max_y < t_max_z {
                    iy += step_y; if iy < 0 || iy >= self.dims[1] { break; }
                    t_enter = t_max_y; t_max_y += t_delta_y;
                } else {
                    iz += step_z; if iz < 0 || iz >= self.dims[2] { break; }
                    t_enter = t_max_z; t_max_z += t_delta_z;
                }
            }
            if t_enter > t_exit { break; }
        }
        false
    }
}
