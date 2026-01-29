use crate::flycam::{FlyCam, Speed};
use crate::pointcloud::{CurrentPointCloud, Highlight, Point, PointCloud};
use bevy::log::info;
use bevy::prelude::*;
use bevy_egui::egui::{Color32, FontFamily, FontId, TextFormat, text::LayoutJob};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

const FONT_SIZE: f32 = 12.0;

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}

#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

fn main_window(
    mut contexts: EguiContexts,
    current_point_cloud: Res<CurrentPointCloud>,
    speed: Single<&Speed, With<FlyCam>>,
) -> Result {
    let point_cloud = Arc::clone(&current_point_cloud.point_cloud);
    let point_cloud_names = Arc::clone(&current_point_cloud.names);
    let is_new = Arc::clone(&current_point_cloud.is_new);
    let color_field = Arc::clone(&current_point_cloud.color_field);

    egui::Window::new("cumulus")
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(15.0, 10.0))
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("Welcome to cumulus! Press G to grab the mouse.");
            ui.add_space(8.0);
            if ui.button("📂 Open point cloud").clicked() {
                let task = rfd::AsyncFileDialog::new().pick_files();
                execute(async move {
                    let files = task.await;
                    if let Some(files) = files {
                        let mut point_clouds: Vec<PointCloud> = vec![];
                        let mut names: Vec<String> = vec![];

                        for file in files {
                            info!("reading new file {}...", file.file_name());
                            let text = file.read().await;

                            let filename = file.file_name();

                            names.push(filename.clone());

                            let extension = Path::new(&filename)
                                .extension()
                                .and_then(OsStr::to_str)
                                .unwrap_or("");

                            point_clouds.push(match PointCloud::from_bytes(extension, &text) {
                                Ok(pc) => pc,
                                Err(e) => {
                                    error!("failed creating pointcloud: {e}");
                                    return;
                                }
                            });
                        }

                        let mut point_cloud = point_cloud.write().expect("lock poisoned");
                        let mut point_cloud_names =
                            point_cloud_names.write().expect("lock poisoned");
                        *point_cloud_names = names;
                        *point_cloud = match PointCloud::merge(point_clouds) {
                            Ok(pc) => pc,
                            Err(e) => {
                                error!("failed merging pointcloud: {e}");
                                return;
                            }
                        };
                        is_new.store(true, Ordering::Relaxed);
                        color_field.store((*point_cloud).fields_number() - 1, Ordering::Relaxed);
                        info!("new file read!");
                    }
                });
            }

            ui.add_space(12.0);
            ui.heading("Opened files");
            let point_cloud_names = current_point_cloud.names.read().expect("lock poisoned");
            let format = TextFormat {
                font_id: FontId::new(FONT_SIZE, FontFamily::Monospace),
                ..Default::default()
            };
            for name in &(*point_cloud_names) {
                let mut job = LayoutJob::default();
                job.append(name, 0.0, format.clone());
                ui.label(job);
            }

            ui.add_space(12.0);
            ui.heading("Controls");
            ui.columns(2, |cols| {
                cols[0].label("G");
                cols[0].label("Mouse movement");
                cols[0].label("Mouse left click");
                cols[0].label("Mouse right click");
                cols[0].label("Mouse wheel");
                cols[0].label("W");
                cols[0].label("S");
                cols[0].label("A");
                cols[0].label("D");
                cols[0].label("SPACE");
                cols[0].label("left SHIFT");
                cols[0].label("up/down arrows");
                cols[0].label("N");
                cols[0].label("M");
                cols[0].label("TAB");

                cols[1].label("release mouse grab");
                cols[1].label("look around");
                cols[1].label("teleport to the point");
                cols[1].label("inspect a point");
                cols[1].label(format!("change speed ({:.1})", speed.0));
                cols[1].label("go forward");
                cols[1].label("go backward");
                cols[1].label("go left");
                cols[1].label("go right");
                cols[1].label("go up");
                cols[1].label("go down");
                cols[1].label("change the points color");
                cols[1].label("decrease the points size");
                cols[1].label("increase the points size");
                cols[1].label("show image view");
            });

            ui.add_space(12.0);
            ui.add(
                egui::Hyperlink::from_label_and_url(
                    "Bugs or feature request?",
                    "https://github.com/hug-dev/cumulus/issues/new",
                )
                .open_in_new_tab(true),
            );
        });

    Ok(())
}

fn point_inspector(
    mut contexts: EguiContexts,
    current_point_cloud: Res<CurrentPointCloud>,
    highlight: Single<(&Point, &Visibility), With<Highlight>>,
) -> Result {
    let point_cloud = Arc::clone(&current_point_cloud.point_cloud);
    let point_cloud_names = Arc::clone(&current_point_cloud.names);
    let color_field = Arc::clone(&current_point_cloud.color_field);

    egui::Window::new("Point Inspector")
        .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-15.0, 10.0))
        .show(contexts.ctx_mut()?, |ui| {
            let point_cloud = point_cloud.read().expect("lock poisoned");
            let point_cloud_names = point_cloud_names.read().expect("lock poisoned");
            let color_field = color_field.load(Ordering::Relaxed);

            ui.columns(2, |cols| {
                for field_index in 0..point_cloud.fields_number() {
                    let field_name = point_cloud.get_field_name(field_index);

                    let mut field_job = LayoutJob::default();
                    let mut format = TextFormat {
                        font_id: FontId::new(FONT_SIZE, FontFamily::Monospace),
                        ..Default::default()
                    };
                    if field_index == color_field {
                        format.color = Color32::RED;
                    }
                    field_job.append(field_name, 0.0, format);
                    cols[0].label(field_job);

                    if highlight.1 == Visibility::Visible
                        && !current_point_cloud.is_new.load(Ordering::Relaxed)
                    {
                        let mut field_job = LayoutJob::default();
                        let mut format = TextFormat {
                            font_id: FontId::new(FONT_SIZE, FontFamily::Monospace),
                            ..Default::default()
                        };
                        if field_index == color_field {
                            format.color = Color32::RED;
                        }
                        let field_value = highlight.0.get_field(field_index);
                        if field_name == "filename" {
                            field_job.append(
                                &point_cloud_names[field_value as usize].to_string(),
                                0.0,
                                format,
                            );
                            cols[1].label(field_job);
                        } else {
                            field_job.append(&format!("{}", field_value), 0.0, format);
                            cols[1].label(field_job);
                        }
                    }
                }
            });
        });

    Ok(())
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default());
        app.add_systems(
            EguiPrimaryContextPass,
            (main_window, point_inspector).chain(),
        );
    }
}
