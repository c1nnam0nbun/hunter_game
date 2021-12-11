use crate::utils::{limit, set_mag};
use bevy::math::Vec3;

pub(crate) struct Physics {
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub wander_theta: f32,
}

pub struct WanderData {
    pub weight: f32,
    pub displace_range: f32,
    pub radius: f32,
    pub max_force: f32,
    pub distance: f32,
}

pub struct FleeData {
    pub weight: f32,
    pub max_flee_time: f32,
}

pub struct PursueData {
    pub weight: f32,
}

pub struct EvadeData {
    pub weight: f32,
}

pub struct EvadeWallsData {
    pub weight: f32,
}

pub struct FlockingData {
    pub perception_radius: f32,
    pub max_force: f32,
}

pub fn seek(position: Vec3, velocity: Vec3, target: Vec3, max_speed: f32) -> Vec3 {
    let desired = set_mag(target - position, max_speed);

    let mut steer = desired - velocity;
    steer = limit(steer, max_speed);

    steer
}

pub fn flee(position: Vec3, velocity: Vec3, target: Vec3, max_speed: f32) -> Vec3 {
    let desired = set_mag(position - target, max_speed);

    let mut steer = desired - velocity;
    steer = limit(steer, max_speed);

    steer
}

pub fn wander(
    position: Vec3,
    velocity: Vec3,
    wander_radius: f32,
    distance: f32,
    wander_theta: f32,
    max_force: f32,
) -> Vec3 {
    let mut wander_point = set_mag(velocity, distance);
    wander_point += position;

    let theta = wander_theta + velocity.y.atan2(velocity.x);
    let x = wander_radius * theta.cos();
    let y = wander_radius * theta.sin();
    wander_point += Vec3::new(x, y, 0.0);

    let mut steer = wander_point - position;
    steer = set_mag(steer, max_force);

    steer
}

pub fn pursue(
    position: Vec3,
    velocity: Vec3,
    target_position: Vec3,
    target_velocity: Vec3,
    max_speed: f32,
) -> Vec3 {
    let distance = target_position - position;
    let t = distance.length() / max_speed;
    let future_position = target_position + target_velocity * t;
    seek(position, velocity, future_position, max_speed)
}

pub fn evade(
    position: Vec3,
    velocity: Vec3,
    target_position: Vec3,
    target_velocity: Vec3,
    max_speed: f32,
) -> Vec3 {
    let distance = target_position - position;
    let t = distance.length() / max_speed;
    let future_position = target_position + target_velocity * t;
    flee(position, velocity, future_position, max_speed)
}
