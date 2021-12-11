use crate::{
    components::{Fatal, MovementSpeed, Prey, Threat},
    steering::{flee, wander, EvadeWallsData, FleeData, Physics, WanderData},
    utils::{dist, limit, line_line_intersection},
    wolf::{Wolf, WolfBehavior, WolfData},
    FieldSize, Materials, Walls, TIME_STEP, player::{Bullet, BulletData},
};
use bevy::{
    app::{AppBuilder, Plugin},
    core::FixedTimestep,
    prelude::*,
    sprite::collide_aabb::collide,
};

use rand::Rng;
use std::f32::consts::PI;

pub(crate) struct Hare;

struct ActiveHares {
    count: u32,
}

impl Default for ActiveHares {
    fn default() -> Self {
        Self { count: 0 }
    }
}

pub(crate) struct HareData {
    pub transform: Transform,
    pub movement_speed: f32,
    pub width: f32,
    pub height: f32,
    pub max_number: u32,
}

pub struct HareSteeringData {
    pub wander: WanderData,
    pub flee: FleeData,
    pub evade_walls: EvadeWallsData,
}

struct HareBehavior {
    force: Vec3,
    flee_time: f32,
}

pub struct HarePlugin;

impl Plugin for HarePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(hare_spawn.system().label("hare_spawn"))
            .insert_resource(ActiveHares::default())
            .add_system(
                hare_flee
                    .system()
                    .label("hare_flee")
                    .before("hare_movement")
                    .after("hare_spawn"),
            )
            .add_system(hare_move.system().label("hare_movement"))
            .add_system(
                hare_wander
                    .system()
                    .label("hare_wander")
                    .after("hare_flee")
                    .before("hare_movement")
                    .after("hare_spawn"),
            )
            .add_system(
                hare_evade_walls
                    .system()
                    .label("hare_avoid_walls")
                    .before("hare_flee")
                    .before("hare_movement")
                    .after("hare_spawn"),
            )
            .add_system(hare_die.system().label("hare_die"));
    }
}

fn hare_spawn(
    mut commands: Commands,
    materials: Res<Materials>,
    mut active_hares: ResMut<ActiveHares>,
    filed_size: Res<FieldSize>,
    settings: Res<HareData>,
) {
    if active_hares.count < settings.max_number {
        let mut rng = rand::thread_rng();
        let w_span = filed_size.width / 2.0 - 30.0;
        let h_span = filed_size.height / 2.0 - 30.0;
        let x = rng.gen_range(-w_span..w_span) as f32;
        let y = rng.gen_range(-h_span..h_span) as f32;

        commands
            .spawn_bundle(SpriteBundle {
                material: materials.hare_material.clone(),
                transform: Transform {
                    translation: Vec3::new(x, y, 0.0),
                    scale: settings.transform.scale,
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Hare)
            .insert(Threat)
            .insert(Prey)
            .insert(MovementSpeed::new(settings.movement_speed))
            .insert(Physics {
                velocity: Vec3::new(0.0, -2.0, 0.0),
                acceleration: Vec3::default(),
                wander_theta: PI / 2.0,
            })
            .insert(HareBehavior {
                force: Vec3::ZERO,
                flee_time: 0.0,
            });

        active_hares.count += 1;
    }
}

fn hare_move(
    mut query: Query<
        (
            &mut Transform,
            &mut Physics,
            &mut HareBehavior,
            &mut MovementSpeed,
        ),
        With<Hare>,
    >,
    active_hares: Res<ActiveHares>,
    settings: Res<HareData>,
) {
    if active_hares.count < settings.max_number {
        return;
    }

    for (mut transform, mut physics, mut behavior, mut speed) in query.iter_mut() {
        physics.acceleration += behavior.force;

        let acc_clone = physics.acceleration.clone();
        physics.velocity += acc_clone;
        physics.velocity = limit(physics.velocity, speed.value * TIME_STEP);
        transform.translation += physics.velocity;
        physics.acceleration *= 0.0;
        behavior.force *= 0.0;

        let angle = physics.velocity.y.atan2(physics.velocity.x) - PI / 2.0;

        transform.rotation = Quat::from_rotation_z(angle);
    }
}

fn hare_wander(
    mut query: Query<(&Transform, &mut Physics, &mut HareBehavior), With<Hare>>,
    active_hares: Res<ActiveHares>,
    settings: Res<HareData>,
    behavior_data: Res<HareSteeringData>,
) {
    if active_hares.count < settings.max_number {
        return;
    }

    let mut rng = rand::thread_rng();
    let displace_range: f32 = behavior_data.wander.displace_range;
    let mut displacements = vec![0.0; settings.max_number as usize];

    for i in 0..settings.max_number {
        displacements[i as usize] = rng.gen_range(-displace_range..displace_range);
    }

    let mut count: usize = 0;

    for (transform, mut physics, mut behavior) in query.iter_mut() {
        let force = wander(
            transform.translation,
            physics.velocity,
            behavior_data.wander.radius,
            behavior_data.wander.distance,
            physics.wander_theta,
            behavior_data.wander.max_force,
        );
        behavior.force += force * behavior_data.wander.weight;
        physics.wander_theta += displacements[count];
        count += 1;
    }
}

fn hare_flee(
    mut hare_query: Query<
        (&Transform, &Physics, &mut MovementSpeed, &mut HareBehavior),
        With<Hare>,
    >,
    threat_query: Query<&Transform, With<Threat>>,
    active_hares: Res<ActiveHares>,
    settings: Res<HareData>,
    behavior_data: Res<HareSteeringData>,
    time: Res<Time>,
) {
    if active_hares.count < settings.max_number {
        return;
    }

    for (hare_transform, physics, mut speed, mut behavior) in hare_query.iter_mut() {
        for threat_transform in threat_query.iter() {
            if hare_transform == threat_transform {
                continue;
            }

            let now = time.seconds_since_startup();

            if now >= (behavior.flee_time + behavior_data.flee.max_flee_time).into() {
                behavior.flee_time = 0.0;
                speed.value = settings.movement_speed;
            }

            let ds = dist(hare_transform.translation, threat_transform.translation);
            if ds < 100.0 {
                speed.value = settings.movement_speed + 50.0;
                let force = flee(
                    hare_transform.translation,
                    physics.velocity,
                    threat_transform.translation,
                    speed.value * TIME_STEP,
                );

                behavior.flee_time = now as f32;
                behavior.force += force * behavior_data.flee.weight;
            }
        }
    }
}

fn hare_evade_walls(
    mut hare_query: Query<(&Transform, &Physics, &MovementSpeed, &mut HareBehavior), With<Hare>>,
    active_hares: Res<ActiveHares>,
    settings: Res<HareData>,
    behavior_data: Res<HareSteeringData>,
    walls: Res<Walls>,
) {
    if active_hares.count < settings.max_number {
        return;
    }

    for (transform, physics, speed, mut behavior) in hare_query.iter_mut() {
        for wall in walls.value.iter() {
            if let Ok(int) = line_line_intersection(
                wall.point_a,
                wall.point_b,
                transform.translation,
                transform.translation + physics.velocity,
            ) {
                if dist(transform.translation, int) > 40.0 {
                    continue;
                }

                let force = flee(
                    transform.translation,
                    physics.velocity,
                    int,
                    speed.value * TIME_STEP,
                );
                behavior.force += force * behavior_data.evade_walls.weight;
            }
        }
    }
}

fn hare_die(
    mut commands: Commands,
    hare_query: Query<(Entity, &Transform), With<Hare>>,
    mut wolf_query: Query<(&Transform, &mut WolfBehavior), With<Wolf>>,
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
    hare_data: Res<HareData>,
    wolf_data: Res<WolfData>,
    bullet_data: Res<BulletData>,
    time: Res<Time>,
) {
    for (hare, hare_transform) in hare_query.iter() {
        for (wolf_transform, mut behavior) in wolf_query.iter_mut() {
            if collide(
                hare_transform.translation,
                Vec2::new(hare_data.width, hare_data.height),
                wolf_transform.translation,
                Vec2::new(wolf_data.width, wolf_data.height),
            )
            .is_some()
            {
                commands.entity(hare).despawn();
                behavior.hunger_time = time.seconds_since_startup() as f32;
                break;
            }
        }

        for (bullet, bullet_transform) in bullet_query.iter() {
            if collide(
                hare_transform.translation,
                Vec2::new(hare_data.width, hare_data.height),
                bullet_transform.translation,
                Vec2::new(bullet_data.width, bullet_data.height),
            )
            .is_some()
            {
                commands.entity(hare).despawn();
                commands.entity(bullet).despawn();
            }
        }
    }
}
