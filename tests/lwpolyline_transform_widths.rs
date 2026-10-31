//! A scale moves a lightweight polyline's widths with its geometry: the constant width and the start and end width of each vertex.

use acadrust::entities::*;
use acadrust::types::{Transform, Vector2, Vector3};

fn polyline() -> LwPolyline {
    let mut polyline = LwPolyline::new();
    polyline.constant_width = 0.25;
    for (x, start, end) in [(0.0, 0.0625, 0.125), (3.0, 0.5, 0.0)] {
        let mut vertex = LwVertex::new(Vector2::new(x, 1.0));
        vertex.start_width = start;
        vertex.end_width = end;
        polyline.vertices.push(vertex);
    }
    polyline
}

#[test]
fn an_xy_scale_scales_the_constant_and_the_vertex_widths() {
    let mut polyline = polyline();
    polyline.apply_transform(&Transform::from_scaling(Vector3::new(2.0, 2.0, 1.0)));
    assert_eq!(polyline.constant_width, 0.5);
    let widths: Vec<(f64, f64)> = polyline
        .vertices
        .iter()
        .map(|v| (v.start_width, v.end_width))
        .collect();
    assert_eq!(widths, [(0.125, 0.25), (1.0, 0.0)]);
    assert_eq!(polyline.vertices[1].location, Vector2::new(6.0, 2.0));
}

#[test]
fn a_rotation_leaves_the_widths_as_they_are() {
    let mut polyline = polyline();
    polyline.apply_transform(&Transform::from_rotation(
        Vector3::new(0.0, 0.0, 1.0),
        std::f64::consts::FRAC_PI_2,
    ));
    let near = |a: f64, b: f64| (a - b).abs() < 1e-12;
    assert!(near(polyline.constant_width, 0.25));
    assert!(near(polyline.vertices[0].start_width, 0.0625));
    assert!(near(polyline.vertices[1].start_width, 0.5));
}
