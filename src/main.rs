mod components;
mod deer;
mod hare;
mod player;
mod steering;
mod utils;
mod wolf;

use crate::hare::HarePlugin;
use bevy::prelude::*;
use deer::{DeerData, DeerPlugin, DeerSteeringData};
use hare::{HareData, HareSteeringData};
use player::BulletData;
use serde_json::{from_str, Value};
use std::fs;
use steering::{EvadeData, EvadeWallsData, FleeData, FlockingData, PursueData, WanderData};
use wolf::{WolfData, WolfPlugin, WolfSteeringData};

use crate::components::{MainCamera, Materials, MousePosition};
use crate::player::{PlayerData, PlayerPlugin};

const TIME_STEP: f32 = 1.0 / 60.0;

struct FieldSize {
    width: f32,
    height: f32,
}

struct WallData {
    point_a: Vec3,
    point_b: Vec3,
}

struct Walls {
    value: Vec<WallData>,
}

fn main() {
    App::build()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(WindowDescriptor {
            title: "Hunter Game".to_string(),
            width: 1280.0,
            height: 720.0,
            ..Default::default()
        })
        .insert_resource(MousePosition::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system_to_stage(CoreStage::PreUpdate, cursor_screen_to_world.system())
        .add_plugin(PlayerPlugin)
        .add_plugin(HarePlugin)
        .add_plugin(WolfPlugin)
        .add_plugin(DeerPlugin)
        .run();
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window: Res<WindowDescriptor>,
    asset_server: Res<AssetServer>,
) {
    let contents =
        fs::read_to_string("assets/settings.json").expect("Something went wrong reading the file");

    let settings: Value = from_str(contents.as_str()).unwrap();

    let player_transform_data = &settings["player"]["transform"];
    let mut player_transform = Transform::default();
    if !player_transform_data.is_null() {
        player_transform = get_transform(player_transform_data);
    }

    commands.insert_resource(PlayerData {
        transform: player_transform,
        movement_speed: settings["player"]["movement_speed"].as_f64().unwrap() as f32,
        width: 60.0 * player_transform.scale.x,
        height: 60.0 * player_transform.scale.y,
    });

    let hare_transform_data = &settings["hare"]["transform"];
    let mut hare_transform = Transform::default();
    if !hare_transform_data.is_null() {
        hare_transform = get_transform(hare_transform_data);
    }

    commands.insert_resource(HareData {
        transform: hare_transform,
        movement_speed: settings["hare"]["movement_speed"].as_f64().unwrap() as f32,
        width: 60.0 * hare_transform.scale.x,
        height: 60.0 * hare_transform.scale.y,
        max_number: settings["hare"]["max_number"].as_u64().unwrap() as u32,
    });

    let wolf_transform_data = &settings["wolf"]["transform"];
    let mut wolf_transform = Transform::default();
    if !wolf_transform_data.is_null() {
        wolf_transform = get_transform(wolf_transform_data);
    }

    commands.insert_resource(WolfData {
        transform: wolf_transform,
        movement_speed: settings["wolf"]["movement_speed"].as_f64().unwrap() as f32,
        width: 60.0 * wolf_transform.scale.x,
        height: 60.0 * wolf_transform.scale.y,
        max_number: settings["wolf"]["max_number"].as_u64().unwrap() as u32,
    });

    let deer_transform_data = &settings["deer"]["transform"];
    let mut deer_transform = Transform::default();
    if !deer_transform_data.is_null() {
        deer_transform = get_transform(deer_transform_data);
    }

    commands.insert_resource(DeerData {
        transform: deer_transform,
        movement_speed: settings["deer"]["movement_speed"].as_f64().unwrap() as f32,
        width: 60.0 * deer_transform.scale.x,
        height: 60.0 * deer_transform.scale.y,
        max_number: settings["deer"]["max_number"].as_u64().unwrap() as u32,
        group_number: settings["deer"]["group_number"].as_u64().unwrap() as u32,
    });

    commands.insert_resource(BulletData {
        width: 24.0,
        height: 24.0,
        movement_speed: settings["bullet"]["movement_speed"].as_f64().unwrap() as f32,
        max_duration: settings["deer"]["max_duration"].as_f64().unwrap() as f32
    });

    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);

    commands.insert_resource(Materials {
        player_material: materials.add(ColorMaterial {
            color: Color::rgb(
                settings["player"]["material"]["color"]["r"]
                    .as_f64()
                    .unwrap() as f32,
                settings["player"]["material"]["color"]["g"]
                    .as_f64()
                    .unwrap() as f32,
                settings["player"]["material"]["color"]["b"]
                    .as_f64()
                    .unwrap() as f32,
            ),
            texture: asset_server
                .load(settings["player"]["material"]["texture"].as_str().unwrap())
                .into(),
        }),
        hare_material: materials.add(ColorMaterial {
            texture: asset_server
                .load(settings["hare"]["material"]["texture"].as_str().unwrap())
                .into(),
            ..Default::default()
        }),
        wolf_material: materials.add(ColorMaterial {
            color: Color::rgb(
                settings["wolf"]["material"]["color"]["r"].as_f64().unwrap() as f32,
                settings["wolf"]["material"]["color"]["g"].as_f64().unwrap() as f32,
                settings["wolf"]["material"]["color"]["b"].as_f64().unwrap() as f32,
            ),
            texture: asset_server
                .load(settings["wolf"]["material"]["texture"].as_str().unwrap())
                .into(),
        }),
        deer_material: materials.add(ColorMaterial {
            color: Color::rgb(
                settings["deer"]["material"]["color"]["r"].as_f64().unwrap() as f32,
                settings["deer"]["material"]["color"]["g"].as_f64().unwrap() as f32,
                settings["deer"]["material"]["color"]["b"].as_f64().unwrap() as f32,
            ),
            texture: asset_server
                .load(settings["deer"]["material"]["texture"].as_str().unwrap())
                .into(),
        }),
        bullet_material: materials.add(ColorMaterial {
            texture: asset_server
                .load(settings["bullet"]["material"]["texture"].as_str().unwrap())
                .into(),
            ..Default::default()
        }),
    });

    commands.insert_resource(HareSteeringData {
        wander: WanderData {
            weight: settings["hare"]["steering"]["wander"]["weight"]
                .as_f64()
                .unwrap() as f32,
            displace_range: settings["hare"]["steering"]["wander"]["displace_range"]
                .as_f64()
                .unwrap() as f32,
            radius: settings["hare"]["steering"]["wander"]["radius"]
                .as_f64()
                .unwrap() as f32,
            max_force: settings["hare"]["steering"]["wander"]["max_force"]
                .as_f64()
                .unwrap() as f32,
            distance: settings["hare"]["steering"]["wander"]["distance"]
                .as_f64()
                .unwrap() as f32,
        },
        flee: steering::FleeData {
            weight: settings["hare"]["steering"]["flee"]["weight"]
                .as_f64()
                .unwrap() as f32,
            max_flee_time: settings["hare"]["steering"]["flee"]["max_flee_time"]
                .as_f64()
                .unwrap() as f32,
        },
        evade_walls: EvadeWallsData {
            weight: settings["hare"]["steering"]["evade_walls"]["weight"]
                .as_f64()
                .unwrap() as f32,
        },
    });

    commands.insert_resource(WolfSteeringData {
        wander: WanderData {
            weight: settings["wolf"]["steering"]["wander"]["weight"]
                .as_f64()
                .unwrap() as f32,
            displace_range: settings["wolf"]["steering"]["wander"]["displace_range"]
                .as_f64()
                .unwrap() as f32,
            radius: settings["wolf"]["steering"]["wander"]["radius"]
                .as_f64()
                .unwrap() as f32,
            max_force: settings["wolf"]["steering"]["wander"]["max_force"]
                .as_f64()
                .unwrap() as f32,
            distance: settings["wolf"]["steering"]["wander"]["distance"]
                .as_f64()
                .unwrap() as f32,
        },
        evade_walls: EvadeWallsData {
            weight: settings["wolf"]["steering"]["evade_walls"]["weight"]
                .as_f64()
                .unwrap() as f32,
        },
        pursue: PursueData {
            weight: settings["wolf"]["steering"]["pursue"]["weight"]
                .as_f64()
                .unwrap() as f32,
        },
    });

    commands.insert_resource(DeerSteeringData {
        wander: WanderData {
            weight: settings["deer"]["steering"]["wander"]["weight"]
                .as_f64()
                .unwrap() as f32,
            displace_range: settings["deer"]["steering"]["wander"]["displace_range"]
                .as_f64()
                .unwrap() as f32,
            radius: settings["deer"]["steering"]["wander"]["radius"]
                .as_f64()
                .unwrap() as f32,
            max_force: settings["deer"]["steering"]["wander"]["max_force"]
                .as_f64()
                .unwrap() as f32,
            distance: settings["deer"]["steering"]["wander"]["distance"]
                .as_f64()
                .unwrap() as f32,
        },
        evade_walls: EvadeWallsData {
            weight: settings["deer"]["steering"]["evade_walls"]["weight"]
                .as_f64()
                .unwrap() as f32,
        },
        flee: FleeData {
            weight: settings["deer"]["steering"]["flee"]["weight"]
                .as_f64()
                .unwrap() as f32,
            max_flee_time: 0.0,
        },
        evade: EvadeData {
            weight: settings["deer"]["steering"]["evade"]["weight"]
                .as_f64()
                .unwrap() as f32,
        },
        separation: FlockingData {
            perception_radius: settings["deer"]["steering"]["separation"]["perception_radius"]
                .as_f64()
                .unwrap() as f32,
            max_force: settings["deer"]["steering"]["separation"]["max_force"]
                .as_f64()
                .unwrap() as f32,
        },
        alignment: FlockingData {
            perception_radius: settings["deer"]["steering"]["alignment"]["perception_radius"]
                .as_f64()
                .unwrap() as f32,
            max_force: settings["deer"]["steering"]["alignment"]["max_force"]
                .as_f64()
                .unwrap() as f32,
        },
        cohesion: FlockingData {
            perception_radius: settings["deer"]["steering"]["cohesion"]["perception_radius"]
                .as_f64()
                .unwrap() as f32,
            max_force: settings["deer"]["steering"]["cohesion"]["max_force"]
                .as_f64()
                .unwrap() as f32,
        },
    });

    let width = window.width - 80.0;
    let height = window.height - 20.0;

    commands.spawn_bundle(SpriteBundle {
        material: materials.add(Color::rgba(0.1, 0.7, 0.2, 1.0).into()),
        sprite: Sprite::new(Vec2::new(width, height)),
        ..Default::default()
    });

    commands.insert_resource(FieldSize { width, height });

    commands.insert_resource(Walls {
        value: vec![
            WallData {
                point_a: Vec3::new(width / 2.0, height / 2.0, 0.0),
                point_b: Vec3::new(width / 2.0, -(height / 2.0), 0.0),
            },
            WallData {
                point_a: Vec3::new(-(width / 2.0), -(height / 2.0), 0.0),
                point_b: Vec3::new(width / 2.0, -(height / 2.0), 0.0),
            },
            WallData {
                point_a: Vec3::new(-(width / 2.0), height / 2.0, 0.0),
                point_b: Vec3::new(-(width / 2.0), -(height / 2.0), 0.0),
            },
            WallData {
                point_a: Vec3::new(-(width / 2.0), height / 2.0, 0.0),
                point_b: Vec3::new(width / 2.0, height / 2.0, 0.0),
            },
        ],
    });
}

fn cursor_screen_to_world(
    mut commands: Commands,
    windows: Res<Windows>,
    query: Query<&Transform, With<MainCamera>>,
) {
    let window = windows.get_primary().unwrap();

    if let Some(pos) = window.cursor_position() {
        let size = Vec2::new(window.width(), window.height());
        let p = pos - size / 2.0;
        let camera_transform = query.single().unwrap();
        let pos_wld = camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);
        commands.insert_resource(MousePosition::new(pos_wld.x.clone(), pos_wld.y.clone()));
    }
}

fn get_transform(transform_data: &Value) -> Transform {
    let mut transform = Transform::default();

    let translation = &transform_data["translation"];
    if !translation.is_null() {
        transform.translation = Vec3::new(
            translation["x"].as_f64().unwrap() as f32,
            translation["y"].as_f64().unwrap() as f32,
            translation["z"].as_f64().unwrap() as f32,
        )
    }

    let rotation = &transform_data["rotation"];
    if !rotation.is_null() {
        transform.rotation = Quat::from_rotation_ypr(
            rotation["y"].as_f64().unwrap() as f32,
            rotation["x"].as_f64().unwrap() as f32,
            rotation["z"].as_f64().unwrap() as f32,
        )
    }

    let scale = &transform_data["scale"];
    if !scale.is_null() {
        transform.scale = Vec3::new(
            scale["x"].as_f64().unwrap() as f32,
            scale["y"].as_f64().unwrap() as f32,
            scale["z"].as_f64().unwrap() as f32,
        )
    }

    transform
}
