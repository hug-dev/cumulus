use crate::pointcloud::{CurrentPointCloud, PointCloud, PointCloudLoaderPlugin};
use bevy::color::Color;
use bevy::prelude::*;
use clap::Parser;
use flycam::FlyCamPlugin;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize};

mod csv;
mod flycam;
mod pcd;
mod ply;
mod pointcloud;
mod ui;
mod utils;

const BACKGROUND_COLOR: Color = Color::srgb_u8(255, 161, 247);

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Paths of the files to display
    paths: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    // When running the CLI version, allow a point cloud file as input.
    let args = Args::parse();
    let (point_clouds, names): (Vec<PointCloud>, Vec<String>) = if args.paths.is_empty() {
        (
            vec![PointCloud::from_bytes(
                "ply",
                include_bytes!("../resources/ply_test/example.ply"),
            )?],
            vec!["example.ply".to_string()],
        )
    } else {
        let res: anyhow::Result<Vec<PointCloud>> = args
            .paths
            .iter()
            .map(|path| PointCloud::from_file(path))
            .collect();
        (
            res?,
            args.paths
                .iter()
                .map(|p| {
                    Path::new(p)
                        .file_name()
                        .expect("path without filename")
                        .to_str()
                        .unwrap()
                        .into()
                })
                .collect::<Vec<_>>(),
        )
    };

    // Merge all point clouds together into one.
    let point_cloud = PointCloud::merge(point_clouds)?;

    let fields_number = point_cloud.fields_number();

    let current_point_cloud = CurrentPointCloud {
        is_new: Arc::new(AtomicBool::new(true)),
        point_cloud: Arc::new(RwLock::new(point_cloud)),
        color_field: Arc::new(AtomicUsize::new(fields_number - 1)),
        names: Arc::new(RwLock::new(names)),
    };

    let mut window = Window {
        // Maximize the Bevy window on the whole browser page for wasm.
        fit_canvas_to_parent: true,
        ..default()
    };
    // Maximize the window when running locally.
    window.set_maximized(true);

    let code = App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(window),
                ..default()
            }),
            FlyCamPlugin,
            ui::UiPlugin,
            PointCloudLoaderPlugin,
        ))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        // Add the first point cloud to load.
        .insert_resource(current_point_cloud)
        .run();

    if code.is_error() {
        Err(anyhow::anyhow!("error running cumulus CLI"))
    } else {
        Ok(())
    }
}
