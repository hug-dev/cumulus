use crate::pointcloud::{Point, PointCloud};
use anyhow::Result;
use bevy::color::Color;
use csv::Reader;

pub fn init_csv(buf: &[u8]) -> Result<PointCloud> {
    let mut rdr = Reader::from_reader(buf);

    let mut fields_map = Vec::new();
    let mut field_names = Vec::new();

    for name in rdr.headers()? {
        fields_map.push((name.to_string(), f32::MAX, f32::MIN));
        field_names.push(name.to_string());
    }

    for (n, record) in rdr.records().enumerate() {
        for (i, val) in record?.iter().enumerate() {
            let val: f32 = val
                .parse()
                .map_err(|e| anyhow::anyhow!("can not parse record {n} (\"{val}\") to f32: {e}"))?;

            if val < fields_map[i].1 {
                fields_map[i].1 = val;
            }
            if val > fields_map[i].2 {
                fields_map[i].2 = val;
            }
        }
    }

    // Reset the reader from the start.
    let mut rdr = Reader::from_reader(buf);

    let points: Result<Vec<Point>> = rdr
        .records()
        .map(|u| {
            let mut fields = vec![(0., Color::WHITE); 3];
            let record = u?;

            for (i, (field_name, min, max)) in fields_map.iter().enumerate() {
                let val: f32 = record[i].parse()?;

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

    let points = points?;
    let width = points.len();

    PointCloud::new(points, field_names, width, 1)
}
