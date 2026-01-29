use crate::flycam::FlyCam;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            is_active: false,
            ..default()
        },
    ));
}

pub struct ImageCamPlugin;
impl Plugin for ImageCamPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, toggle_image);
    }
}

fn toggle_image(
    keys: Res<ButtonInput<KeyCode>>,
    mut fly_camera: Query<&mut Camera, With<FlyCam>>,
    mut image_camera: Query<&mut Camera, Without<FlyCam>>,
    mut primary_cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) -> Result {
    if keys.just_pressed(KeyCode::Tab) {
        let mut fly_camera = fly_camera.single_mut()?;
        let mut image_camera = image_camera.single_mut()?;

        fly_camera.is_active = !fly_camera.is_active;
        image_camera.is_active = !image_camera.is_active;

        if image_camera.is_active {
            primary_cursor_options.grab_mode = CursorGrabMode::None;
            primary_cursor_options.visible = true;
        }
    }

    Ok(())
}
