use std::f32::consts::PI;

use bevy::{
    core::Time,
    math::{Quat, Vec3},
    prelude::{
        AppBuilder, Commands, Entity, IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query,
        Res, ResMut, SpriteBundle, Transform, With,
    },
};
use rand::Rng;

use crate::{
    components::{Materials, MovementSpeed, Prey, Threat},
    steering::{flee, pursue, wander, EvadeWallsData, Physics, PursueData, WanderData},
    utils::{dist, limit, line_line_intersection},
    FieldSize, Walls, TIME_STEP,
};

pub(crate) struct WolfData {
    pub transform: Transform,
    pub movement_speed: f32,
    pub width: f32,
    pub height: f32,
    pub max_number: u32,
}

pub struct WolfSteeringData {
    pub wander: WanderData,
    pub evade_walls: EvadeWallsData,
    pub pursue: PursueData,
}

pub struct WolfBehavior {
    force: Vec3,
    pub hunger_time: f32,
    max_hunger_time: f32,
}

pub struct WolfPlugin;

impl Plugin for WolfPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ActiveWolves { count: 0 })
            .add_system(wolf_spawn.system().label("wolf_spawn"))
            .add_system(
                wolf_wander
                    .system()
                    .label("wolf_wander")
                    .after("wolf_spawn")
                    .before("wolf_move"),
            )
            .add_system(
                wolf_evade_walls
                    .system()
                    .label("wolf_evade_walls")
                    .before("wolf_move"),
            )
            .add_system(
                wolf_pursue
                    .system()
                    .label("wolf_pursue")
                    .before("wolf_move"),
            )
            .add_system(wolf_move.system().label("wolf_move").after("wolf_spawn"))
            .add_system(
                wolf_starve
                    .system()
                    .label("wolf_starve")
                    .after("wolf_spawn"),
            );
    }
}

struct ActiveWolves {
    count: u32,
}

pub struct Wolf;

fn wolf_spawn(
    mut commands: Commands,
    materials: Res<Materials>,
    mut active_wolves: ResMut<ActiveWolves>,
    filed_size: Res<FieldSize>,
    settings: Res<WolfData>,
) {
    if active_wolves.count < settings.max_number {
        let mut rng = rand::thread_rng();
        let w_span = filed_size.width / 2.0 - 30.0;
        let h_span = filed_size.height / 2.0 - 30.0;
        let x = rng.gen_range(-w_span..w_span) as f32;
        let y = rng.gen_range(-h_span..h_span) as f32;

        commands
            .spawn_bundle(SpriteBundle {
                material: materials.wolf_material.clone(),
                transform: Transform {
                    translation: Vec3::new(x, y, 0.0),
                    scale: settings.transform.scale,
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Wolf)
            .insert(Threat)
            .insert(MovementSpeed::new(settings.movement_speed))
            .insert(Physics {
                velocity: Vec3::new(0.0, -2.0, 0.0),
                acceleration: Vec3::default(),
                wander_theta: PI / 2.0,
            })
            .insert(WolfBehavior {
                force: Vec3::ZERO,
                hunger_time: 0.0,
                max_hunger_time: 5.0,
            });

        active_wolves.count += 1;
    }
}

fn wolf_move(
    mut query: Query<
        (
            &mut Transform,
            &mut Physics,
            &mut WolfBehavior,
            &MovementSpeed,
        ),
        With<Wolf>,
    >,
    active_wolves: Res<ActiveWolves>,
    settings: Res<WolfData>,
) {
    if active_wolves.count < settings.max_number {
        return;
    }

    for (mut transform, mut physics, mut behavior, speed) in query.iter_mut() {
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

fn wolf_wander(
    mut query: Query<(&Transform, &mut Physics, &mut WolfBehavior), With<Wolf>>,
    active_wolves: Res<ActiveWolves>,
    settings: Res<WolfData>,
    behavior_data: Res<WolfSteeringData>,
) {
    if active_wolves.count < settings.max_number {
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

fn wolf_evade_walls(
    mut wolf_query: Query<(&Transform, &Physics, &MovementSpeed, &mut WolfBehavior), With<Wolf>>,
    active_wolves: Res<ActiveWolves>,
    settings: Res<WolfData>,
    behavior_data: Res<WolfSteeringData>,
    walls: Res<Walls>,
) {
    if active_wolves.count < settings.max_number {
        return;
    }

    for (transform, physics, speed, mut behavior) in wolf_query.iter_mut() {
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

fn wolf_pursue(
    mut wolf_query: Query<(&Transform, &Physics, &MovementSpeed, &mut WolfBehavior), With<Wolf>>,
    prey_query: Query<(&Transform, &Physics), With<Prey>>,
    active_wolves: Res<ActiveWolves>,
    settings: Res<WolfData>,
    behavior_data: Res<WolfSteeringData>,
) {
    if active_wolves.count < settings.max_number {
        return;
    }

    for (wolf_transform, physics, speed, mut behavior) in wolf_query.iter_mut() {
        for (prey_transform, prey_physics) in prey_query.iter() {
            let ds = dist(wolf_transform.translation, prey_transform.translation);

            let force = pursue(
                wolf_transform.translation,
                physics.velocity,
                prey_transform.translation,
                prey_physics.velocity,
                speed.value * TIME_STEP,
            );

            behavior.force += if ds > 100.0 {
                Vec3::ZERO
            } else {
                force * behavior_data.pursue.weight
            };
        }
    }
}

fn wolf_starve(
    mut commands: Commands,
    mut query: Query<(Entity, &mut WolfBehavior), With<Wolf>>,
    time: Res<Time>,
    active_wolves: Res<ActiveWolves>,
    settings: Res<WolfData>,
) {
    if active_wolves.count < settings.max_number {
        return;
    }    

    for (wolf, mut behavior) in query.iter_mut() {
        let now = time.seconds_since_startup();

    if behavior.hunger_time == 0.0 {
        behavior.hunger_time = now as f32;
    }
        if now > (behavior.hunger_time + behavior.max_hunger_time).into() {
            commands.entity(wolf).despawn();
        }
    }
}
