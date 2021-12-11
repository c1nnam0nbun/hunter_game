use crate::{
    components::{Prey, Threat},
    steering::Physics,
    wolf::{Wolf, WolfBehavior, WolfData},
    FieldSize,
};
use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
};
use std::f32::consts::PI;

use crate::{
    components::{Materials, MousePosition, MovementSpeed},
    TIME_STEP,
};

pub(crate) struct Player;
pub(crate) struct Bullet;

pub(crate) struct PlayerData {
    pub transform: Transform,
    pub movement_speed: f32,
    pub width: f32,
    pub height: f32,
}

pub struct BulletData {
    pub width: f32,
    pub height: f32,
    pub movement_speed: f32,
    pub max_duration: f32,
}

pub struct BulletDuration {
    pub shot_at: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_stage(
            "game_setup_player",
            SystemStage::single(player_spawn.system().label("player_spawn")),
        )
        .add_system(player_move.system().label("player_movement"))
        .add_system(player_rotate.system().label("player_rotation"))
        .add_system(
            player_check_intersection
                .system()
                .label("player_intersection"),
        )
        .add_system(player_shoot.system().label("player_shoot"))
        .add_system(
            bullet_fly
                .system()
                .label("bullet_fly")
                .after("player_shoot"),
        )
        .add_system(
            player_die
                .system()
                .label("player_die")
                .after("player_spawn"),
        );
    }
}

fn player_spawn(mut commands: Commands, materials: Res<Materials>, settings: Res<PlayerData>) {
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.player_material.clone(),
            transform: settings.transform,
            ..Default::default()
        })
        .insert(Player)
        .insert(Threat)
        .insert(Prey)
        .insert(MovementSpeed::new(settings.movement_speed))
        .insert(Physics {
            velocity: Vec3::new(0.0, -2.0, 0.0),
            acceleration: Vec3::default(),
            wander_theta: 0.0,
        });
}

fn player_move(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&MovementSpeed, &mut Transform, &mut Physics), With<Player>>,
) {
    if let Ok((speed, mut transform, mut physics)) = query.single_mut() {
        let mut dir = Vec3::default();

        if keyboard_input.pressed(KeyCode::A) {
            dir.x = -1.0;
        }
        if keyboard_input.pressed(KeyCode::D) {
            dir.x = 1.0;
        }
        if keyboard_input.pressed(KeyCode::W) {
            dir.y = 1.0;
        }
        if keyboard_input.pressed(KeyCode::S) {
            dir.y = -1.0;
        }

        dir.normalize();
        physics.velocity = dir * speed.value * TIME_STEP;
        transform.translation += physics.velocity;
    }
}

fn player_rotate(
    mouse_position: Res<MousePosition>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    if let Ok(mut transform) = query.single_mut() {
        let dir: Vec3 = transform.translation - mouse_position.value;
        let angle = dir.y.atan2(dir.x.clone()) + PI / 2.0;

        transform.rotation = Quat::from_rotation_z(angle);
    }
}

fn player_check_intersection(
    mut query_player: Query<&mut Transform, With<Player>>,
    data: Res<PlayerData>,
    filed_size: Res<FieldSize>,
) {
    if let Ok(mut player_transform) = query_player.single_mut() {
        if let Some(collision) = collide(
            player_transform.translation,
            Vec2::new(data.width, data.height),
            Vec3::default(),
            Vec2::new(filed_size.width, filed_size.height),
        ) {
            match collision {
                Collision::Top => player_transform.translation.y -= 1.0,
                Collision::Right => player_transform.translation.x -= 1.0,
                Collision::Bottom => player_transform.translation.y += 1.0,
                Collision::Left => player_transform.translation.x += 1.0,
            }
        }
    }
}

fn player_shoot(
    mut commands: Commands,
    query: Query<&Transform, With<Player>>,
    mouse: Res<Input<MouseButton>>,
    materials: Res<Materials>,
    bullet_data: Res<BulletData>,
    time: Res<Time>,
) {
    if let Ok(transform) = query.single() {
        if mouse.just_released(MouseButton::Left) {
            commands
                .spawn_bundle(SpriteBundle {
                    material: materials.bullet_material.clone(),
                    transform: Transform {
                        translation: transform.translation,
                        rotation: transform.rotation,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(Bullet)
                .insert(Physics {
                    velocity: transform.local_y() * bullet_data.movement_speed * TIME_STEP,
                    acceleration: Vec3::default(),
                    wander_theta: 0.0,
                })
                .insert(BulletDuration {
                    shot_at: time.seconds_since_startup() as f32,
                });
        }
    }
}

fn bullet_fly(
    mut commands: Commands,
    mut query: Query<(&mut Transform, &Physics, &BulletDuration, Entity), With<Bullet>>,
    bullet_data: Res<BulletData>,
    time: Res<Time>,
) {
    for (mut transform, physics, duration, bullet) in query.iter_mut() {
        let now = time.seconds_since_startup();
        println!("{}, {}", now, (duration.shot_at + bullet_data.max_duration));
        if now < (duration.shot_at + bullet_data.max_duration).into() {
            transform.translation += physics.velocity;
        } else {
            commands.entity(bullet).despawn();
        }
    }
}

fn player_die(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut wolf_query: Query<(&Transform, &mut WolfBehavior), With<Wolf>>,
    player_data: Res<PlayerData>,
    wolf_data: Res<WolfData>,
    time: Res<Time>,
) {
    if let Ok((player, player_transform)) = player_query.single() {
        for (wolf_transform, mut behavior) in wolf_query.iter_mut() {
            if collide(
                player_transform.translation,
                Vec2::new(player_data.width, player_data.height),
                wolf_transform.translation,
                Vec2::new(wolf_data.width, wolf_data.height),
            )
            .is_some()
            {
                commands.entity(player).despawn();
                behavior.hunger_time = time.seconds_since_startup() as f32;
                break;
            }
        }
    }
}
