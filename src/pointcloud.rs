use crate::flycam::FlyCam;
use anyhow::{Result, anyhow, bail};
use bevy::color::Color;
use bevy::log::info;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_pointcloud::PointCloudPlugin;
use bevy_pointcloud::point_cloud::PointCloud as BevyPointCloud;
use bevy_pointcloud::point_cloud::PointCloud3d;
use bevy_pointcloud::point_cloud::PointCloudData as BevyPointCloudData;
use bevy_pointcloud::point_cloud_material::{PointCloudMaterial, PointCloudMaterial3d};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicBool, AtomicUsize};

const POINT_SIZE: f32 = 50.0;
const HIGHLIGHT_POINT_COLOR: Color = Color::srgba_u8(255, 0, 0, 230);
const HIGHLIGHT_POINT_SIZE: f32 = 0.025;
const HIGHLIGHT_POINT_MAX_DIST: f32 = 0.2;

#[derive(Debug)]
pub struct PointCloud {
    // Stored row-first in an organized point cloud.
    // Row 0 is the bottom one and column 0 is the left-most one.
    points: Vec<Point>,
    field_names: Vec<String>,
    // Number of columns in an organized point cloud, otherwise number of points.
    width: usize,
    // Number of rows in an organized point cloud, otherwise 1.
    height: usize,
}

impl PointCloud {
    pub fn new(
        points: Vec<Point>,
        field_names: Vec<String>,
        width: usize,
        height: usize,
    ) -> Result<Self> {
        let len = points.len();

        if field_names[0] != "x" {
            bail!("fields[0] should be x");
        }
        if field_names[1] != "y" {
            bail!("fields[1] should be y");
        }
        if field_names[2] != "z" {
            bail!("fields[2] should be z");
        }

        // Various checks with width and height.
        if width * height != len {
            bail!("width and height do not match number of points ({width}*{height} != {len})");
        }

        if height == 0 {
            bail!("height can not be 0");
        }

        Ok(Self {
            points,
            field_names,
            width,
            height,
        })
    }

    pub fn field_color_at(&self, row: usize, column: usize, index: usize) -> Color {
        self.points[row * self.width + column].get_field_color(index)
    }

    pub fn fields_number(&self) -> usize {
        self.field_names.len()
    }

    pub fn get_field_name(&self, index: usize) -> &str {
        &self.field_names[index]
    }

    pub fn is_organized(&self) -> bool {
        self.height != 1
    }

    pub fn to_bevy_pointcloud(&self) -> BevyPointCloud {
        // Use the last field as color.
        let field_color_index = self.field_names.len() - 1;

        BevyPointCloud {
            points: self
                .points
                .iter()
                .map(|p| {
                    let color = p.get_field_color(field_color_index).to_srgba();
                    BevyPointCloudData {
                        position: Vec3::new(p.x(), p.y(), p.z()),
                        // The size in the point cloud material will be used.
                        point_size: 1.0,
                        color: [color.red, color.green, color.blue, color.alpha],
                    }
                })
                .collect(),
        }
    }

    // Add a new field to the point cloud, setting all points to the same value and color.
    pub fn add_field(&mut self, field: String, value: f32, color: Color) {
        self.field_names.push(field);
        for point in &mut self.points {
            point.fields.push((value, color));
        }
    }

    // only keep x, y and z columns
    pub fn truncate_xyz(&mut self) {
        self.field_names.truncate(3);

        for point in &mut self.points {
            point.fields.truncate(3);
        }
    }

    pub fn from_bytes(file_extension: &str, bytes: &[u8]) -> Result<Self> {
        match file_extension {
            "pcd" => PointCloud::from_pcd_bytes(bytes),
            "ply" => PointCloud::from_ply_bytes(bytes),
            "csv" => PointCloud::from_csv_bytes(bytes),
            // Do nothing in that case.
            e => {
                anyhow::bail!("unsupported point cloud data type: {e}");
            }
        }
    }

    pub fn from_file(filename: &str) -> Result<Self> {
        let extension = if let Some(ext) = Path::new(filename).extension().and_then(OsStr::to_str) {
            Ok(ext)
        } else {
            Err(anyhow::anyhow!(
                "the file given ({filename}) does not have any extension"
            ))
        }?;

        PointCloud::from_bytes(extension, &fs::read(filename)?)
    }

    pub fn from_pcd_bytes(bytes: &[u8]) -> Result<Self> {
        crate::pcd::init_pcd(bytes)
    }

    pub fn from_ply_bytes(bytes: &[u8]) -> Result<Self> {
        crate::ply::init_ply(bytes)
    }

    pub fn from_csv_bytes(bytes: &[u8]) -> Result<Self> {
        crate::csv::init_csv(bytes)
    }

    // Merges all the point clouds together.
    // If they have the same fields: merge as intended and add an "filename" field.
    // If not, only keep the X, Y and Z data with the new "filename" field.
    pub fn merge(mut point_clouds: Vec<Self>) -> Result<Self> {
        if point_clouds.is_empty() {
            bail!("no point clouds to merge");
        }
        let len = point_clouds.len() as f32;
        let mut last = point_clouds.pop().unwrap();
        let mut same_fields = true;

        if len == 1.0 {
            return Ok(last);
        }

        for point_cloud in &point_clouds {
            if point_cloud.field_names != last.field_names {
                same_fields = false;
                break;
            }
        }

        if last.field_names.contains(&"filename".to_string()) {
            bail!("the point clouds already have a filename field");
        }
        if !same_fields {
            last.truncate_xyz();
        }
        last.add_field(
            "filename".to_string(),
            len - 1.0,
            crate::utils::turbo_color(len - 1.0, 0.0, len - 1.0),
        );
        for (i, point_cloud) in point_clouds.iter_mut().enumerate() {
            if !same_fields {
                point_cloud.truncate_xyz();
            }
            point_cloud.add_field(
                "filename".to_string(),
                i as f32,
                crate::utils::turbo_color(i as f32, 0.0, len - 1.0),
            );
            last.points.append(&mut point_cloud.points);
        }

        Ok(last)
    }
}

#[derive(Debug, Component)]
pub struct Point {
    fields: Vec<(f32, Color)>,
}

impl Point {
    pub fn new(fields: Vec<(f32, Color)>) -> Result<Self> {
        Ok(Self { fields })
    }

    pub fn get_field(&self, index: usize) -> f32 {
        self.fields[index].0
    }

    pub fn x(&self) -> f32 {
        self.get_field(0)
    }

    pub fn y(&self) -> f32 {
        self.get_field(1)
    }

    pub fn z(&self) -> f32 {
        self.get_field(2)
    }

    pub fn get_field_color(&self, index: usize) -> Color {
        self.fields[index].1
    }
}

// Bevy plugin to load the point cloud we got from UI.
pub struct PointCloudLoaderPlugin;

#[derive(Resource)]
// Point cloud to load or currently loaded.
pub struct CurrentPointCloud {
    // If it has already been loaded or not.
    pub is_new: Arc<AtomicBool>,
    pub point_cloud: Arc<RwLock<PointCloud>>,
    pub color_field: Arc<AtomicUsize>,
    // Names of the point clouds used.
    pub names: Arc<RwLock<Vec<String>>>,
}

impl Plugin for PointCloudLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(FixedUpdate, load_pointcloud);
        app.add_systems(Update, update_color);
        app.add_systems(Update, update_size);
        app.add_systems(Update, find_closest);
        app.add_plugins(PointCloudPlugin);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Sphere covering the highlighted point.
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(HIGHLIGHT_POINT_SIZE))),
        MeshMaterial3d(materials.add(HIGHLIGHT_POINT_COLOR)),
        Transform::default(),
        Highlight,
        Point { fields: vec![] },
        Visibility::Hidden,
    ));
}

fn load_pointcloud(
    mut commands: Commands,
    mut point_cloud_materials: ResMut<Assets<PointCloudMaterial>>,
    mut point_clouds: ResMut<Assets<BevyPointCloud>>,
    current_point_cloud: Res<CurrentPointCloud>,
    query: Query<Entity, With<PointCloud3d>>,
    highlight: Single<&mut Visibility, With<Highlight>>,
    mut camera: Single<&mut Transform, With<FlyCam>>,
    window: Single<&Window>,
    mut images: ResMut<Assets<Image>>,
) -> bevy::prelude::Result {
    if current_point_cloud.is_new.load(Ordering::Relaxed) {
        info!("loading new pointcloud...");
        let mut highlight = highlight.into_inner();
        let point_cloud = current_point_cloud
            .point_cloud
            .read()
            .expect("lock poisoned");

        // remove the existing point cloud and spawn the new one
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }

        #[cfg(target_arch = "wasm32")]
        let point_cloud_material = PointCloudMaterial {
            point_size: POINT_SIZE,
            ..default()
        };
        #[cfg(not(target_arch = "wasm32"))]
        let point_cloud_material = PointCloudMaterial {
            point_size: POINT_SIZE,
        };
        let material = point_cloud_materials.add(point_cloud_material);
        commands.spawn((
            PointCloud3d(point_clouds.add(point_cloud.to_bevy_pointcloud())),
            PointCloudMaterial3d(material.clone()),
        ));

        if point_cloud.is_organized() {
            // Also load the 2d image on the 2D camera
            // Use the last field as color.
            let field_color_index = point_cloud.field_names.len() - 1;
            let window_width = window.width();
            let window_height = window.height();

            let mut new_image = Image::new_target_texture(
                point_cloud.width as u32,
                point_cloud.height as u32,
                TextureFormat::Rgba32Float,
            );

            for row in 0..point_cloud.height {
                for column in 0..point_cloud.width {
                    new_image.set_color_at(
                        column as u32,
                        row as u32,
                        point_cloud.field_color_at(
                            point_cloud.height - 1 - row,
                            column,
                            field_color_index,
                        ),
                    )?;
                }
            }

            commands.spawn((Sprite {
                image: images.add(new_image),
                custom_size: Some(Vec2::new(window_width, window_height)),
                ..default()
            },));
        }

        current_point_cloud.is_new.store(false, Ordering::Relaxed);
        *highlight = Visibility::Hidden;
        camera.translation = Default::default();
        camera.rotation = Default::default();

        info!("pointcloud loaded!");
    }

    Ok(())
}

// Update the color of each point depending on arrow press.
fn update_color(
    current_point_cloud: Res<CurrentPointCloud>,
    point_cloud_handle: Single<&mut PointCloud3d>,
    mut point_clouds: ResMut<Assets<BevyPointCloud>>,
    // We assume that there is only one sprite: the lidar image.
    sprite: Single<&Sprite>,
    mut images: ResMut<Assets<Image>>,
    key_input: Res<ButtonInput<KeyCode>>,
) -> bevy::prelude::Result {
    if key_input.just_pressed(KeyCode::ArrowUp) || key_input.just_pressed(KeyCode::ArrowDown) {
        let color_field = current_point_cloud.color_field.load(Ordering::Relaxed);
        let point_cloud = current_point_cloud
            .point_cloud
            .read()
            .expect("lock poisoned");
        let fields_number = point_cloud.fields_number();
        let new_color_field = if key_input.just_pressed(KeyCode::ArrowDown) {
            (color_field + 1) % fields_number
        } else if color_field == 0 {
            fields_number - 1
        } else {
            color_field - 1
        };

        let loaded_point_cloud = point_clouds
            .get_mut(&point_cloud_handle.0)
            .ok_or(anyhow!("failed"))?;
        for (i, point) in loaded_point_cloud.points.iter_mut().enumerate() {
            let new_color = point_cloud.points[i].fields[new_color_field].1.to_srgba();
            point.color = [
                new_color.red,
                new_color.green,
                new_color.blue,
                new_color.alpha,
            ];
        }

        let image = images.get_mut(&sprite.image).unwrap();
        for row in 0..point_cloud.height {
            for column in 0..point_cloud.width {
                image.set_color_at(
                    column as u32,
                    row as u32,
                    point_cloud.field_color_at(
                        point_cloud.height - 1 - row,
                        column,
                        new_color_field,
                    ),
                )?;
            }
        }

        current_point_cloud
            .color_field
            .store(new_color_field, Ordering::Relaxed);
    }

    Ok(())
}

/// A marker component for the highlighted point.
/// The bool indicates if there is any highlighted point at all.
#[derive(Component)]
pub struct Highlight;

// Update the size of each point.
fn update_size(
    point_cloud_material_handle: Single<&mut PointCloudMaterial3d>,
    mut point_cloud_materials: ResMut<Assets<PointCloudMaterial>>,
    key_input: Res<ButtonInput<KeyCode>>,
) -> bevy::prelude::Result {
    let offset = if key_input.just_pressed(KeyCode::KeyN) {
        -5.0
    } else if key_input.just_pressed(KeyCode::KeyM) {
        5.0
    } else {
        return Ok(());
    };

    let loaded_point_cloud_material = point_cloud_materials
        .get_mut(&point_cloud_material_handle.0)
        .ok_or(anyhow!("failed"))?;

    loaded_point_cloud_material.point_size += offset;

    Ok(())
}

// find the closest point from the pointer and highlight it or teleport to it
fn find_closest(
    current_point_cloud: Res<CurrentPointCloud>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut camera: Single<&mut Transform, With<FlyCam>>,
    highlight: Single<
        (&mut Visibility, &mut Transform, &mut Point),
        (With<Highlight>, Without<FlyCam>),
    >,
) -> bevy::prelude::Result {
    let (mut visibility, mut transform, mut point) = highlight.into_inner();
    if mouse_button_input.just_released(MouseButton::Left)
        || mouse_button_input.just_released(MouseButton::Right)
    {
        let point_cloud = current_point_cloud
            .point_cloud
            .read()
            .expect("lock poisoned");
        let mut min = f32::MAX;
        let mut closest_point = &point_cloud.points[0];
        for point in &point_cloud.points {
            let position_from_camera =
                Vec3::new(point.x(), point.y(), point.z()) - camera.translation;
            let d = position_from_camera
                .cross(camera.forward().as_vec3())
                .length();
            // Force the point to be in front of the camera.
            if d < min && position_from_camera.dot(camera.forward().as_vec3()) > 0.0 {
                min = d;
                closest_point = point;
            }
        }

        // The point is too far from the cursor to be highlighted.
        if min > HIGHLIGHT_POINT_MAX_DIST {
            *visibility = Visibility::Hidden;
            transform.translation = camera.translation;
        } else if mouse_button_input.just_released(MouseButton::Left) {
            // Highlight the point
            *visibility = Visibility::Visible;
            transform.translation =
                Vec3::new(closest_point.x(), closest_point.y(), closest_point.z());
            point.fields = closest_point.fields.clone();
        } else {
            // Teleport one meter to the point
            camera.translation = Vec3::new(closest_point.x(), closest_point.y(), closest_point.z());
            let forward = camera.forward().as_vec3();
            camera.translation -= forward;
        }
    }

    Ok(())
}
