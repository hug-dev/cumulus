use crate::pointcloud::{Point, PointCloud};
use anyhow::Result;
use bevy::color::Color;
use bevy::log::warn;
use pcd_rs::{DynReader, DynRecord, Field};

fn field_to_f32(field: &Field) -> f32 {
    match field {
        Field::I8(val) => val[0] as f32,
        Field::I16(val) => val[0] as f32,
        Field::I32(val) => val[0] as f32,
        Field::U8(val) => val[0] as f32,
        Field::U16(val) => val[0] as f32,
        Field::U32(val) => val[0] as f32,
        Field::F32(val) => val[0],
        Field::F64(val) => val[0] as f32,
    }
}

pub fn init_pcd(pcd: &[u8]) -> Result<PointCloud> {
    // Declare the reader
    let test_reader = DynReader::from_bytes(pcd)?;

    let meta = test_reader.meta();
    let height = meta.height;
    let width = meta.width;

    // This vector will contain the min and max value of each field in the PCD file so that
    // an adequate color can be found for each value.
    let mut fields_map = vec![
        ("x".to_string(), 0, f32::MAX, f32::MIN),
        ("y".to_string(), 0, f32::MAX, f32::MIN),
        ("z".to_string(), 0, f32::MAX, f32::MIN),
    ];
    let mut field_names = vec!["x".to_string(), "y".to_string(), "z".to_string()];

    // organized point cloud: add row and columns fields
    if height > 1 {
        fields_map.push(("row".to_string(), 0, f32::MAX, f32::MIN));
        fields_map.push(("column".to_string(), 0, f32::MAX, f32::MIN));
        field_names.push("row".to_string());
        field_names.push("column".to_string());
    }

    for (i, field) in meta.field_defs.fields.iter().enumerate() {
        match field.name.as_str() {
            "x" => fields_map[0].1 = i,
            "y" => fields_map[1].1 = i,
            "z" => fields_map[2].1 = i,
            // row and column ignored as we created them as new fields.
            "row" | "column" => {
                warn!(
                    "row and column fields in the PCD file are ignored since it it already organised"
                );
            }
            _ => {
                fields_map.push((field.name.clone(), i, f32::MAX, f32::MIN));
                field_names.push(field.name.clone());
            }
        }
    }

    let records: pcd_rs::Result<Vec<DynRecord>> = test_reader.collect();
    let records = records?;

    for (i, record) in records.iter().enumerate() {
        for (field_name, index, min, max) in &mut fields_map {
            let val = if height > 1 && field_name == "row" {
                (i as u64 / width) as f32
            } else if height > 1 && field_name == "column" {
                // column
                (i as u64 % width) as f32
            } else {
                // everything else
                field_to_f32(&record.0[*index])
            };
            if val < *min {
                *min = val;
            }

            if val > *max {
                *max = val;
            }
        }
    }

    let points: Result<Vec<Point>> = records
        .into_iter()
        .enumerate()
        .map(|(i, u)| {
            let mut fields = vec![(0., Color::WHITE); 3];

            for (field_name, index, min, max) in &fields_map {
                let val = if height > 1 && field_name == "row" {
                    (i as u64 / width) as f32
                } else if height > 1 && field_name == "column" {
                    (i as u64 % width) as f32
                } else {
                    // everything else
                    field_to_f32(&u.0[*index])
                };

                // The TURBO scale is found to find the color of a point.
                let color = crate::utils::turbo_color(val, *min, *max);

                // See the coordinate system: https://bevy-cheatbook.github.io/fundamentals/coords.html
                // Y is up
                // -Z is forward
                // Not the same as ours where X is forward and Z up.
                match field_name.as_str() {
                    "y" => fields[0] = (-val, color),
                    "z" => fields[1] = (val, color),
                    "x" => fields[2] = (-val, color),
                    _ => fields.push((val, color)),
                }
            }

            Point::new(fields)
        })
        .collect();

    PointCloud::new(points?, field_names)
}
