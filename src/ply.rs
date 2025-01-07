use crate::pointcloud::{Point, PointCloud};
use anyhow::Result;
use bevy::color::Color;
use ply_rs_bw::{parser::Parser, ply::Property};

pub fn init_ply(buf: &[u8]) -> Result<PointCloud> {
    let mut cursor = std::io::Cursor::new(buf);
    let parser = Parser::<ply_rs_bw::ply::DefaultElement>::new();
    let ply = parser.read_ply(&mut cursor)?;

    let points_element = match ply.payload.get("vertex") {
        Some(points) => points,
        None => {
            return Err(anyhow::anyhow!(
                "PLY file does not contain a vertex element"
            ));
        }
    };
    if points_element.is_empty() {
        return Err(anyhow::anyhow!("PLY file does not contain any points"));
    }

    let mut fields_map = Vec::new();
    let mut field_names = Vec::new();

    for (name, _prop) in points_element[0].iter() {
        fields_map.push((name.clone(), f32::MAX, f32::MIN));
        field_names.push(name.clone());
    }

    for p in points_element.iter() {
        for (i, (_name, prop)) in p.iter().enumerate() {
            let val = property_to_f32(prop);
            if val < fields_map[i].1 {
                fields_map[i].1 = val;
            }
            if val > fields_map[i].2 {
                fields_map[i].2 = val;
            }
        }
    }

    let points: Result<Vec<Point>> = points_element
        .iter()
        .map(|p| {
            let mut fields = vec![(0.0, Color::WHITE); 3];
            let mut other_fields = Vec::new();
            for (i, (name, prop)) in p.iter().enumerate() {
                let val = property_to_f32(prop);
                let color = crate::utils::turbo_color(val, fields_map[i].1, fields_map[i].2);

                // See the coordinate system: https://bevy-cheatbook.github.io/fundamentals/coords.html
                // Y is up
                // -Z is forward
                // Not the same as ours where X is forward and Z up.
                match name.as_str() {
                    "y" => fields[0] = (-val, color),
                    "z" => fields[1] = (val, color),
                    "x" => fields[2] = (-val, color),
                    _ => other_fields.push((val, color)),
                }
            }
            fields.append(&mut other_fields);
            Point::new(fields)
        })
        .collect();

    PointCloud::new(points?, field_names)
}

fn property_to_f32(prop: &Property) -> f32 {
    match prop {
        Property::Float(v) => *v,
        Property::Double(v) => *v as f32,
        Property::Char(v) => *v as f32,
        Property::UChar(v) => *v as f32,
        Property::Short(v) => *v as f32,
        Property::UShort(v) => *v as f32,
        Property::Int(v) => *v as f32,
        Property::UInt(v) => *v as f32,
        _ => 0.0,
    }
}
