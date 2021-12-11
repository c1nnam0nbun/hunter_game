use std::{f32::consts::PI};

use bevy::{
    math::{Quat, Vec3, Vec2},
    prelude::{
        AppBuilder, Commands, IntoSystem, ParallelSystemDescriptorCoercion, Plugin, Query, Res,
        ResMut, SpriteBundle, Transform, With, Without, Entity,
    }, sprite::collide_aabb::collide, core::Time,
};
use rand::Rng;

use crate::{
    components::{Materials, MovementSpeed, Prey, Threat},
    hare::Hare,
    steering::{
        evade, flee, wander, EvadeData, EvadeWallsData, FleeData, FlockingData, Physics, WanderData,
    },
    utils::{dist, limit, line_line_intersection, set_mag},
    wolf::{Wolf, WolfData, WolfBehavior},
    FieldSize, Walls, TIME_STEP, player::{Bullet, BulletData},
};

pub(crate) struct DeerData {
    pub transform: Transform,
    pub movement_speed: f32,
    pub width: f32,
    pub height: f32,
    pub max_number: u32,
    pub group_number: u32,
}

pub struct DeerSteeringData {
    pub wander: WanderData,
    pub evade_walls: EvadeWallsData,
    pub flee: FleeData,
    pub evade: EvadeData,
    pub separation: FlockingData,
    pub alignment: FlockingData,
    pub cohesion: FlockingData,
}

struct Behavior {
    force: Vec3,
}

struct Deer;

struct GroupID {
    value: u32,
}

struct DeerGroup {
    id: u32,
    count: u32,
}

struct DeerGroups {
    groups: Vec<DeerGroup>,
}

pub struct DeerPlugin;

impl Plugin for DeerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(DeerGroups { groups: Vec::new() })
            .add_system(deer_spawn.system().label("deer_spawn"))
            .add_system(
                deer_wander
                    .system()
                    .label("deer_wander")
                    .after("deer_spawn")
                    .before("deer_move"),
            )
            .add_system(
                deer_flee
                    .system()
                    .label("deer_flee")
                    .before("deer_move")
                    .after("deer_spawn"),
            )
            .add_system(
                deer_alignment
                    .system()
                    .label("deer_alignment")
                    .before("deer_move"),
            )
            .add_system(
                deer_cohesion
                    .system()
                    .label("deer_cohesion")
                    .before("deer_move"),
            )
            .add_system(
                deer_separation
                    .system()
                    .label("deer_separation")
                    .before("deer_move"),
            )
            .add_system(
                deer_evade_walls
                    .system()
                    .label("deer_evade_walls")
                    .before("deer_move"),
            )
            .add_system(deer_evade.system().label("deer_evade").before("deer_move"))
            .add_system(deer_move.system().label("deer_move").after("deer_spawn"))
            .add_system(deer_die.system().label("deer_die"));
    }
}

fn deer_spawn(
    mut commands: Commands,
    materials: Res<Materials>,
    mut deer_groups: ResMut<DeerGroups>,
    filed_size: Res<FieldSize>,
    settings: Res<DeerData>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        let mut rng = rand::thread_rng();
        let w_span = filed_size.width / 2.0 - 60.0;
        let h_span = filed_size.height / 2.0 - 60.0;

        let x = rng.gen_range(-w_span..w_span) as f32;
        let y = rng.gen_range(-h_span..h_span) as f32;

        let deer_count = rng.gen_range(3..settings.max_number);
        let id = rng.gen();
        let group = DeerGroup {
            count: deer_count,
            id,
        };

        deer_groups.groups.push(group);

        for _ in 0..deer_count {
            let x_offset = rng.gen_range(-30.0..30.0) as f32;
            let y_offset = rng.gen_range(-30.0..30.0) as f32;

            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.deer_material.clone(),
                    transform: Transform {
                        translation: Vec3::new(x + x_offset, y + y_offset, 0.0),
                        scale: settings.transform.scale,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Deer)
                .insert(Prey)
                .insert(MovementSpeed::new(settings.movement_speed))
                .insert(Physics {
                    velocity: Vec3::new(0.0, -2.0, 0.0),
                    acceleration: Vec3::default(),
                    wander_theta: PI / 2.0,
                })
                .insert(Behavior { force: Vec3::ZERO })
                .insert(GroupID { value: id });
        }
    }
}

fn deer_move(
    mut query: Query<(&mut Transform, &mut Physics, &mut Behavior, &MovementSpeed), With<Deer>>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
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

fn deer_alignment(
    mut query_mut: Query<
        (
            &Transform,
            &Physics,
            &mut Behavior,
            &GroupID,
            &MovementSpeed,
        ),
        With<Deer>,
    >,
    query_im: Query<(&Transform, &Physics, &GroupID), With<Deer>>,
    behavior_data: Res<DeerSteeringData>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for (transform, physics, mut behavior, id, speed) in query_mut.iter_mut() {
        let perception_radius: f32 = behavior_data.alignment.perception_radius;
        let mut steer = Vec3::default();
        let mut total = 0.0;

        for (other_transform, other_physics, other_id) in query_im.iter() {
            if id.value == other_id.value && other_transform != transform {
                if dist(transform.translation, other_transform.translation) < perception_radius {
                    steer += other_physics.velocity;
                    total += 1.0;
                }
            }
        }

        if total > 0.0 {
            steer /= total;
            steer = set_mag(steer, speed.value);
            steer -= physics.velocity;
            steer = limit(steer, behavior_data.alignment.max_force);
            behavior.force += steer;
        }
    }
}

fn deer_cohesion(
    mut query_mut: Query<
        (
            &Transform,
            &Physics,
            &mut Behavior,
            &GroupID,
            &MovementSpeed,
        ),
        With<Deer>,
    >,
    query_im: Query<(&Transform, &GroupID), With<Deer>>,
    behavior_data: Res<DeerSteeringData>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for (transform, physics, mut behavior, id, speed) in query_mut.iter_mut() {
        let perception_radius: f32 = behavior_data.cohesion.perception_radius;
        let mut steer = Vec3::default();
        let mut total = 0.0;

        for (other_transform, other_id) in query_im.iter() {
            if id.value == other_id.value && other_transform != transform {
                if dist(transform.translation, other_transform.translation) < perception_radius {
                    steer += other_transform.translation;
                    total += 1.0;
                }
            }
        }

        if total > 0.0 {
            steer /= total;
            steer -= transform.translation;
            steer = set_mag(steer, speed.value);
            steer -= physics.velocity;
            steer = limit(steer, behavior_data.cohesion.max_force);
            behavior.force += steer;
        }
    }
}

fn deer_separation(
    mut query_mut: Query<
        (
            &Transform,
            &Physics,
            &mut Behavior,
            &GroupID,
            &MovementSpeed,
        ),
        With<Deer>,
    >,
    query_im: Query<(&Transform, &GroupID), With<Deer>>,
    behavior_data: Res<DeerSteeringData>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for (transform, physics, mut behavior, id, speed) in query_mut.iter_mut() {
        let perception_radius: f32 = behavior_data.separation.perception_radius;
        let mut steer = Vec3::default();
        let mut total = 0.0;

        for (other_transform, other_id) in query_im.iter() {
            if id.value == other_id.value && other_transform != transform {
                let d = dist(transform.translation, other_transform.translation);
                if d < perception_radius {
                    let mut diff = transform.translation - other_transform.translation;
                    diff /= d * d;
                    steer += diff;
                    total += 1.0;
                }
            }
        }

        if total > 0.0 {
            steer /= total;
            steer = set_mag(steer, speed.value);
            steer -= physics.velocity;
            steer = limit(steer, behavior_data.separation.max_force);
            behavior.force += steer;
        }
    }
}

fn deer_wander(
    mut query: Query<(&Transform, &mut Physics, &mut Behavior, &GroupID), With<Deer>>,

    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
    behavior_data: Res<DeerSteeringData>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for group in deer_groups.groups.iter() {
        let mut rng = rand::thread_rng();
        let displace_range: f32 = behavior_data.wander.displace_range;
        let mut displacements = vec![0.0; group.count as usize];

        for i in 0..group.count {
            displacements[i as usize] = rng.gen_range(-displace_range..displace_range);
        }

        let mut count: usize = 0;

        for (transform, mut physics, mut behavior, id) in query.iter_mut() {
            if group.id != id.value {
                continue;
            }

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
}

fn deer_evade_walls(
    mut deer_query: Query<(&Transform, &Physics, &MovementSpeed, &mut Behavior), With<Deer>>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
    behavior_data: Res<DeerSteeringData>,
    walls: Res<Walls>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for (transform, physics, speed, mut behavior) in deer_query.iter_mut() {
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

fn deer_flee(
    mut deer_query: Query<(&Transform, &Physics, &MovementSpeed, &mut Behavior), With<Deer>>,
    threat_query: Query<&Transform, (With<Threat>, Without<Hare>)>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
    behavior_data: Res<DeerSteeringData>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for (deer_transform, physics, speed, mut behavior) in deer_query.iter_mut() {
        for threat_transform in threat_query.iter() {
            if deer_transform == threat_transform {
                continue;
            }

            let ds = dist(deer_transform.translation, threat_transform.translation);
            if ds < 100.0 {
                let force = flee(
                    deer_transform.translation,
                    physics.velocity,
                    threat_transform.translation,
                    speed.value * TIME_STEP,
                );
                behavior.force += force * behavior_data.flee.weight;
            }
        }
    }
}

fn deer_evade(
    mut deer_query: Query<(&Transform, &Physics, &MovementSpeed, &mut Behavior), With<Deer>>,
    wolf_query: Query<(&Transform, &Physics), With<Wolf>>,
    settings: Res<DeerData>,
    deer_groups: Res<DeerGroups>,
    behavior_data: Res<DeerSteeringData>,
) {
    if deer_groups.groups.len() < settings.group_number.try_into().unwrap() {
        return;
    }

    for (deer_transform, physics, speed, mut behavior) in deer_query.iter_mut() {
        for (wolf_transform, prey_physics) in wolf_query.iter() {
            let ds = dist(deer_transform.translation, wolf_transform.translation);

            let force = evade(
                deer_transform.translation,
                physics.velocity,
                wolf_transform.translation,
                prey_physics.velocity,
                speed.value * TIME_STEP,
            );

            behavior.force += if ds > 180.0 {
                Vec3::ZERO
            } else {
                force * behavior_data.evade.weight
            };
        }
    }
}

fn deer_die(
    mut commands: Commands,
    deer_query: Query<(Entity, &Transform), With<Deer>>,
    mut wolf_query: Query<(&Transform, &mut WolfBehavior), With<Wolf>>,
    bullet_query: Query<(Entity, &Transform), With<Bullet>>,
    deer_data: Res<DeerData>,
    wolf_data: Res<WolfData>,
    bullet_data: Res<BulletData>,
    time: Res<Time>
) {
    for (deer, deer_transform) in deer_query.iter() {
        for (wolf_transform, mut behavior) in wolf_query.iter_mut() {
            if collide(
                deer_transform.translation,
                Vec2::new(deer_data.width, deer_data.height),
                wolf_transform.translation,
                Vec2::new(wolf_data.width, wolf_data.height),
            )
            .is_some()
            {
                commands.entity(deer).despawn();
                behavior.hunger_time = time.seconds_since_startup() as f32;
                break;
            }
        }

        for (bullet, bullet_transform) in bullet_query.iter() {
            if collide(
                deer_transform.translation,
                Vec2::new(deer_data.width, deer_data.height),
                bullet_transform.translation,
                Vec2::new(bullet_data.width, bullet_data.height),
            )
            .is_some()
            {
                commands.entity(deer).despawn();
                commands.entity(bullet).despawn();
            }
        }
    }
}
