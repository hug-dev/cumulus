// Code adapted from https://github.com/sburris0/bevy_flycam/
// Big changes were made that might not fit in the original repository, hence the code is forked
// and modified.
//
// Original License:
// =============================
// Copyright 2020 Spencer Burris
//
// Permission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted, provided that the above copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_pointcloud::render::PointCloudRenderMode;

const CURSOR_COLOR: Color = Color::srgb_u8(255, 0, 0);
const CURSOR_SIZE: f32 = 0.0005;
// distance from the viewport to the cursor
const CURSOR_DISTANCE: f32 = 0.15;

const MOVE_FORWARD: KeyCode = KeyCode::KeyW;
const MOVE_BACKWARD: KeyCode = KeyCode::KeyS;
const MOVE_LEFT: KeyCode = KeyCode::KeyA;
const MOVE_RIGHT: KeyCode = KeyCode::KeyD;
const MOVE_ASCEND: KeyCode = KeyCode::Space;
const MOVE_DESCEND: KeyCode = KeyCode::ShiftLeft;
pub const TOGGLE_GRAB_CURSOR: KeyCode = KeyCode::KeyG;
const MOUSE_SENSITIVITY: f32 = 0.00012;
const DEFAULT_SPEED: f32 = 1.;
const MIN_SPEED: f32 = 0.1;
const MAX_SPEED: f32 = 200.0;
const SPEED_INCREMENT: f32 = 0.4;

/// A marker component for the cursor to query it specifically.
#[derive(Component)]
struct Cursor;

/// Used in queries when you want flycams and not other cameras
/// A marker component used in queries when you want flycams and not other cameras
#[derive(Component)]
pub struct FlyCam;

/// Speed of the camera.
#[derive(Component)]
pub struct Speed(pub f32);

/// Handles keyboard input and movement
fn player_move(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &Speed), With<FlyCam>>,
    mut cursor: Query<&mut Transform, (With<Cursor>, Without<FlyCam>)>,
) -> Result {
    if primary_cursor_options.grab_mode == CursorGrabMode::None {
        return Ok(());
    }

    let mut cursor_transform = cursor.single_mut()?;
    let (mut camera, speed) = camera.single_mut()?;

    let mut direction = Vec3::ZERO;

    for key in keys.get_pressed() {
        direction += match *key {
            MOVE_FORWARD => camera.forward().as_vec3(),
            MOVE_BACKWARD => camera.back().as_vec3(),
            MOVE_LEFT => camera.left().as_vec3(),
            MOVE_RIGHT => camera.right().as_vec3(),
            MOVE_ASCEND => Vec3::Y,
            MOVE_DESCEND => Vec3::NEG_Y,
            _ => Vec3::ZERO,
        };
    }

    camera.translation += direction.normalize_or_zero() * time.delta_secs() * speed.0;

    // move the cursor
    let mut new_transform = *camera;
    new_transform.translation += camera.forward() * CURSOR_DISTANCE;
    (*cursor_transform) = new_transform;

    Ok(())
}

/// Handles looking around if cursor is locked
fn player_look(
    primary_window: Query<&mut Window, With<PrimaryWindow>>,
    primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut state: MessageReader<MouseMotion>,
    mut query: Query<&mut Transform, With<FlyCam>>,
    mut cursor: Query<&mut Transform, (With<Cursor>, Without<FlyCam>)>,
) -> Result {
    if primary_cursor_options.grab_mode == CursorGrabMode::None {
        return Ok(());
    }

    let mut cursor_transform = cursor.single_mut()?;

    if let Ok(window) = primary_window.single() {
        for mut transform in query.iter_mut() {
            for ev in state.read() {
                let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

                // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                let window_scale = window.height().min(window.width());
                pitch -= (MOUSE_SENSITIVITY * ev.delta.y * window_scale).to_radians();
                yaw -= (MOUSE_SENSITIVITY * ev.delta.x * window_scale).to_radians();

                pitch = pitch.clamp(-1.54, 1.54);

                // Order is important to prevent unintended roll
                transform.rotation =
                    Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);

                // move the cursor
                let mut new_transform = *transform;
                new_transform.translation += transform.forward() * CURSOR_DISTANCE;
                (*cursor_transform) = new_transform;
            }
        }
    } else {
        warn!("Primary window not found for `player_look`!");
    }

    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut cursor_start = Transform::default();
    cursor_start.translation += cursor_start.forward() * CURSOR_DISTANCE;

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        NoIndirectDrawing,
        Msaa::Off,
        PointCloudRenderMode {
            use_edl: true,
            edl_radius: 2.8,
            edl_strength: 0.4,
            edl_neighbour_count: 4,
        },
        FlyCam,
        Speed(DEFAULT_SPEED),
    ));

    // pointer, just in front of the camera.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(CURSOR_SIZE))),
        MeshMaterial3d(materials.add(CURSOR_COLOR)),
        cursor_start,
        Cursor,
    ));
}

fn cursor_grab(
    keys: Res<ButtonInput<KeyCode>>,
    mut primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(TOGGLE_GRAB_CURSOR) {
        match primary_cursor_options.grab_mode {
            CursorGrabMode::None => {
                primary_cursor_options.grab_mode = CursorGrabMode::Confined;
                primary_cursor_options.visible = false;
            }
            _ => {
                primary_cursor_options.grab_mode = CursorGrabMode::None;
                primary_cursor_options.visible = true;
            }
        }
    }
}

// change the camera speed
fn change_speed(
    mut evr_scroll: MessageReader<MouseWheel>,
    mut speed: Single<&mut Speed, With<FlyCam>>,
) -> bevy::prelude::Result {
    for ev in evr_scroll.read() {
        if ev.y > 0.0 {
            speed.0 += SPEED_INCREMENT;
        } else if ev.y < 0.0 {
            speed.0 -= SPEED_INCREMENT;
        }
    }
    speed.0 = speed.0.max(MIN_SPEED);
    speed.0 = speed.0.min(MAX_SPEED);
    Ok(())
}

pub struct FlyCamPlugin;
impl Plugin for FlyCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, player_move)
            .add_systems(Update, player_look)
            .add_systems(Update, change_speed)
            .add_systems(Update, cursor_grab);
    }
}
