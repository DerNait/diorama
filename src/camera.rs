use raylib::prelude::*;

/// Configuración de la cámara orbital (fácil de tunear).
#[derive(Clone, Copy, Debug)]
pub struct CameraConfig {
    /// Sensibilidad de órbita (radianes por unidad de input).
    pub orbit_sensitivity_yaw: f32,
    pub orbit_sensitivity_pitch: f32,
    /// Sensibilidad de zoom (unidades de distancia por input).
    pub zoom_sensitivity: f32,
    /// Límites del pitch (en radianes). Usualmente (-pi/2 + eps, pi/2 - eps).
    pub min_pitch: f32,
    pub max_pitch: f32,
    /// Límites de distancia (zoom). min>0.
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            orbit_sensitivity_yaw:   1.0,
            orbit_sensitivity_pitch: 1.0,
            zoom_sensitivity:        1.0,
            min_pitch:  -1.45,   // ~ -83°
            max_pitch:   1.45,   // ~  83°
            min_distance: 0.25,
            max_distance: 5000.0,
        }
    }
}

/// Base precomputada para “ray directions”.
#[derive(Clone, Copy)]
pub struct CameraBasis {
    pub eye: Vector3,
    pub forward: Vector3,
    pub right: Vector3,
    pub up: Vector3,
}

/// Cámara orbital: siempre mira al centro.
pub struct Camera {
    /// Punto que orbitamos y observamos.
    pub center: Vector3,
    /// Distancia desde la cámara al centro.
    pub distance: f32,
    /// Ángulos esféricos (en radianes).
    pub yaw: f32,   // rotación alrededor del eje Y (horizontal)
    pub pitch: f32, // elevación
    /// Vectores base (se actualizan con `update_basis_vectors`)
    pub eye: Vector3,
    pub forward: Vector3,
    pub right: Vector3,
    pub up: Vector3,
    /// Config editable
    pub config: CameraConfig,
}

impl Camera {
    pub fn new(eye: Vector3, center: Vector3, up_hint: Vector3) -> Self {
        let offset = eye - center;
        let mut distance = offset.length().max(1e-6);
        let pitch = (offset.y / distance).asin();            // [-pi/2, pi/2]
        let yaw = offset.z.atan2(offset.x);                  // [-pi, pi]

        let mut cam = Self {
            center,
            distance,
            yaw,
            pitch,
            eye: eye,
            forward: Vector3::zero(),
            right: Vector3::zero(),
            up: up_hint,
            config: CameraConfig::default(),
        };

        cam.clamp_angles_and_distance();
        cam.update_eye_from_spherical();
        cam.update_basis_vectors();
        cam
    }

    pub fn from_spherical(center: Vector3, distance: f32, yaw: f32, pitch: f32) -> Self {
        let mut cam = Self {
            center,
            distance,
            yaw,
            pitch,
            eye: Vector3::zero(),
            forward: Vector3::zero(),
            right: Vector3::zero(),
            up: Vector3::new(0.0, 1.0, 0.0),
            config: CameraConfig::default(),
        };
        cam.clamp_angles_and_distance();
        cam.update_eye_from_spherical();
        cam.update_basis_vectors();
        cam
    }

    #[inline]
    pub fn set_config(&mut self, cfg: CameraConfig) {
        self.config = cfg;
        self.clamp_angles_and_distance();
        self.update_eye_from_spherical();
        self.update_basis_vectors();
    }

    #[inline]
    pub fn set_center(&mut self, new_center: Vector3) {
        self.center = new_center;
        self.update_eye_from_spherical();
        self.update_basis_vectors();
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw   += delta_yaw  * self.config.orbit_sensitivity_yaw;
        self.pitch += delta_pitch * self.config.orbit_sensitivity_pitch;
        if self.yaw > std::f32::consts::PI { self.yaw -= 2.0*std::f32::consts::PI; }
        if self.yaw < -std::f32::consts::PI { self.yaw += 2.0*std::f32::consts::PI; }

        self.clamp_angles_and_distance();
        self.update_eye_from_spherical();
        self.update_basis_vectors();
    }

    pub fn zoom(&mut self, amount: f32) {
        self.distance += amount * self.config.zoom_sensitivity;
        self.clamp_angles_and_distance();
        self.update_eye_from_spherical();
        self.update_basis_vectors();
    }

    pub fn zoom_exp(&mut self, amount: f32) {
        let factor = (1.0 + 0.2 * amount).max(0.05);
        self.distance *= factor;
        self.clamp_angles_and_distance();
        self.update_eye_from_spherical();
        self.update_basis_vectors();
    }

    #[inline]
    fn update_eye_from_spherical(&mut self) {
        let cp = self.pitch.cos();
        let x = self.distance * cp * self.yaw.cos();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * cp * self.yaw.sin();
        self.eye = self.center + Vector3::new(x, y, z);
    }

    #[inline]
    fn clamp_angles_and_distance(&mut self) {
        self.pitch = self.pitch.clamp(self.config.min_pitch, self.config.max_pitch);
        self.distance = self.distance.clamp(self.config.min_distance, self.config.max_distance);
    }

    pub fn update_basis_vectors(&mut self) {
        self.forward = (self.center - self.eye).normalized();
        let world_up = Vector3::new(0.0, 1.0, 0.0);
        let mut right = self.forward.cross(world_up);
        if right.length() < 1e-6 {
            let alt_up = Vector3::new(0.0, 0.0, 1.0);
            right = self.forward.cross(alt_up);
        }
        self.right = right.normalized();
        self.up = self.right.cross(self.forward).normalized();
    }

    #[inline]
    pub fn basis_change(&self, v: &Vector3) -> Vector3 {
        Vector3::new(
            v.x * self.right.x + v.y * self.up.x - v.z * self.forward.x,
            v.x * self.right.y + v.y * self.up.y - v.z * self.forward.y,
            v.x * self.right.z + v.y * self.up.z - v.z * self.forward.z,
        )
    }

    #[inline]
    pub fn basis(&self) -> CameraBasis {
        CameraBasis {
            eye: self.eye,
            forward: self.forward,
            right: self.right,
            up: self.up,
        }
    }
}
