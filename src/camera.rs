use raylib::prelude::*;

pub struct Camera {
    pub eye: Vector3,    
    pub center: Vector3, 
    pub up: Vector3,     
    pub forward: Vector3,
    pub right: Vector3,  
}

#[derive(Clone, Copy)]
pub struct CameraBasis {
    pub eye: Vector3,
    pub forward: Vector3,
    pub right: Vector3,
    pub up: Vector3,
}

impl Camera {
    pub fn new(eye: Vector3, center: Vector3, up: Vector3) -> Self {
        let mut camera = Camera {
            eye,
            center,
            up,
            forward: Vector3::zero(), 
            right: Vector3::zero(),   
        };
        camera.update_basis_vectors();
        camera
    }

    pub fn update_basis_vectors(&mut self) {
        self.forward = (self.center - self.eye).normalized();

        self.right = self.forward.cross(self.up).normalized();

        self.up = self.right.cross(self.forward);
    }

    pub fn orbit(&mut self, yaw: f32, pitch: f32) {

        let relative_pos = self.eye - self.center;

        let radius = relative_pos.length();

        let current_yaw = relative_pos.z.atan2(relative_pos.x);
        let current_pitch = (relative_pos.y / radius).asin();
        
        let new_yaw = current_yaw + yaw;
        let new_pitch = (current_pitch + pitch).clamp(-1.5, 1.5);

        let cos_pitch = new_pitch.cos();
        let new_relative_pos = Vector3::new(
            radius * cos_pitch * new_yaw.cos(),  
            radius * new_pitch.sin(),              
            radius * cos_pitch * new_yaw.sin(),  
        );
        
        self.eye = self.center + new_relative_pos;
        
        self.update_basis_vectors();
    }

    pub fn basis_change(&self, v: &Vector3) -> Vector3 {
        Vector3::new(
            v.x * self.right.x + v.y * self.up.x - v.z * self.forward.x,
            v.x * self.right.y + v.y * self.up.y - v.z * self.forward.y,
            v.x * self.right.z + v.y * self.up.z - v.z * self.forward.z,
        )
    }

    pub fn basis(&self) -> CameraBasis {
        CameraBasis {
            eye: self.eye,
            forward: self.forward,
            right: self.right,
            up: self.up,
        }
    }
}
