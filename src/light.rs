use raylib::prelude::*;

/// Tipo de luz
#[derive(Clone, Copy, Debug)]
pub enum LightKind {
    Point,      
    Directional,
}

#[derive(Clone, Copy)]
pub struct Light {
    pub kind: LightKind,
    pub position: Vector3,
    pub direction: Vector3,
    pub color: Color,
    pub intensity: f32,
}

impl Light {
    pub fn new(position: Vector3, color: Color, intensity: f32) -> Self {
        Self {
            kind: LightKind::Point,
            position,
            direction: Vector3::new(-1.0, -1.0, -1.0).normalized(),
            color,
            intensity,
        }
    }

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

    pub fn at(&self, point: Vector3) -> (Vector3, f32) {
        match self.kind {
            LightKind::Point => {
                let to = self.position - point;
                let dist = to.length();
                if dist > 0.0 { (to / dist, dist) } else { (Vector3::new(0.0, -1.0, 0.0), 0.0) }
            }
            LightKind::Directional => {
                (-self.direction, f32::INFINITY)
            }
        }
    }

    pub fn translate(&mut self, delta: Vector3) {
        if matches!(self.kind, LightKind::Point) {
            self.position += delta;
        }
    }

    pub fn yaw_pitch(&mut self, yaw: f32, pitch: f32) {
        if !matches!(self.kind, LightKind::Directional) { return; }
        let mut dir = self.direction;
        let r = dir.length();
        if r == 0.0 { dir = Vector3::new(-1.0,-1.0,-1.0).normalized(); }

        let mut cur_yaw   = dir.z.atan2(dir.x);
        let mut cur_pitch = (dir.y).asin().clamp(-0.999, 0.999);

        cur_yaw += yaw;
        cur_pitch = (cur_pitch + pitch).clamp(-1.3, 1.3);

        let cp = cur_pitch.cos();
        let x = cp * cur_yaw.cos();
        let y = cur_pitch.sin();
        let z = cp * cur_yaw.sin();

        self.direction = Vector3::new(x, y, z).normalized();
    }

    pub fn clone_light_readonly(&self) -> Light {
        Light {
            kind: self.kind,
            position: self.position,
            direction: self.direction,
            color: self.color,
            intensity: self.intensity,
        }
    }
}
