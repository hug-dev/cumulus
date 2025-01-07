use bevy::prelude::*;

pub fn turbo_color(val: f32, min: f32, max: f32) -> Color {
    let ratio = (val - min) / (max - min);
    let tmp_color = colorous::TURBO.eval_continuous(ratio as f64);
    Color::srgb(
        tmp_color.r as f32 / 256.0,
        tmp_color.g as f32 / 256.0,
        tmp_color.b as f32 / 256.0,
    )
}
