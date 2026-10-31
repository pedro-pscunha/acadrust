//! A scale moves a 2D polyline's widths with its geometry: the polyline's default widths and the start and end width of each vertex.

use acadrust::entities::*;
use acadrust::types::{Transform, Vector3};

#[test]
fn an_xy_scale_scales_the_default_and_the_vertex_widths() {
    let mut polyline = Polyline2D::new();
    polyline.start_width = 0.25;
    polyline.end_width = 0.5;
    for x in [0.0, 3.0] {
        let mut vertex = Vertex2D::new(Vector3::new(x, 1.0, 0.0));
        vertex.start_width = 0.0625;
        vertex.end_width = 0.125;
        polyline.vertices.push(vertex);
    }
    polyline.apply_transform(&Transform::from_scaling(Vector3::new(2.0, 2.0, 1.0)));
    assert_eq!((polyline.start_width, polyline.end_width), (0.5, 1.0));
    let widths: Vec<(f64, f64)> = polyline
        .vertices
        .iter()
        .map(|v| (v.start_width, v.end_width))
        .collect();
    assert_eq!(widths, [(0.125, 0.25), (0.125, 0.25)]);
}
