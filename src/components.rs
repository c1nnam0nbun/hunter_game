use bevy::prelude::*;

pub struct Threat;
pub struct Prey;

pub(crate) struct MovementSpeed {
    pub value: f32,
}

impl MovementSpeed {
    pub fn new(speed: f32) -> Self {
        Self { value: speed }
    }
}

pub(crate) struct MainCamera;

pub(crate) struct Materials {
    pub player_material: Handle<ColorMaterial>,
    pub hare_material: Handle<ColorMaterial>,
    pub wolf_material: Handle<ColorMaterial>,
    pub deer_material: Handle<ColorMaterial>,
    pub bullet_material: Handle<ColorMaterial>,
}

pub(crate) struct MousePosition {
    pub value: Vec3,
}

impl MousePosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            value: Vec3::new(x, y, 0.0),
        }
    }
}

impl Default for MousePosition {
    fn default() -> Self {
        Self {
            value: Vec3::default(),
        }
    }
}
