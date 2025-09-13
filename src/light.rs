use raylib::prelude::*;

/// Tipo de luz
#[derive(Clone, Copy, Debug)]
pub enum LightKind {
    Point,        // usa position
    Directional,  // usa direction (vector unitario, sentido = hacia donde viaja la luz)
}

pub struct Light {
    pub kind: LightKind,
    pub position: Vector3,  // usado si kind = Point
    pub direction: Vector3, // usado si kind = Directional (unitario)
    pub color: Color,
    pub intensity: f32,
}

impl Light {
    /// Luz puntual (compat con tu código anterior)
    pub fn new(position: Vector3, color: Color, intensity: f32) -> Self {
        Self {
            kind: LightKind::Point,
            position,
            direction: Vector3::new(-1.0, -1.0, -1.0).normalized(),
            color,
            intensity,
        }
    }

    /// Luz direccional (tipo Sol). `dir` es hacia dónde viaja la luz.
    pub fn directional(dir: Vector3, color: Color, intensity: f32) -> Self {
        let d = if dir.length() > 0.0 { dir.normalized() } else { Vector3::new(-1.0,-1.0,-1.0).normalized() };
        Self {
            kind: LightKind::Directional,
            position: Vector3::zero(),
            direction: d,
            color,
            intensity,
        }
    }

    /// Devuelve (light_dir, light_distance) desde un punto de la escena:
    /// - light_dir: vector unitario desde el punto hacia la fuente de luz
    /// - light_distance: distancia hasta la luz (∞ si es direccional)
    pub fn at(&self, point: Vector3) -> (Vector3, f32) {
        match self.kind {
            LightKind::Point => {
                let to = self.position - point;
                let dist = to.length();
                if dist > 0.0 { (to / dist, dist) } else { (Vector3::new(0.0, -1.0, 0.0), 0.0) }
            }
            LightKind::Directional => {
                // direction = hacia dónde viaja la luz, así que desde el punto hacia la fuente es -direction
                (-self.direction, f32::INFINITY)
            }
        }
    }

    /// Mueve la luz puntual
    pub fn translate(&mut self, delta: Vector3) {
        if matches!(self.kind, LightKind::Point) {
            self.position += delta;
        }
    }

    /// Rotación yaw/pitch para luz direccional (en ejes globales Y/X)
    pub fn yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        if !matches!(self.kind, LightKind::Directional) { return; }
        // ángulos actuales a partir del vector direction
        let mut dir = self.direction;
        let r = dir.length();
        if r == 0.0 { dir = Vector3::new(-1.0,-1.0,-1.0).normalized(); }

        let mut cur_yaw   = dir.z.atan2(dir.x);          // [-pi, pi]
        let mut cur_pitch = (dir.y).asin().clamp(-0.999, 0.999); // [-~pi/2, ~pi/2]

        cur_yaw += yaw;
        cur_pitch = (cur_pitch + pitch).clamp(-1.3, 1.3);

        let cp = cur_pitch.cos();
        let x = cp * cur_yaw.cos();
        let y = cur_pitch.sin();
        let z = cp * cur_yaw.sin();

        self.direction = Vector3::new(x, y, z).normalized();
    }
}
