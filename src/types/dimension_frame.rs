//! The frame of a dimension's group 12 point: an OCS point of the entity's frame in the entity, the X and Y of the WCS point in a DWG.

use super::transform::Matrix3;
use super::vector::Vector3;

/// The arbitrary-axis frame of `normal`, or `None` for a normal that has no direction (the point is then taken as it is).
fn frame(normal: Vector3) -> Option<Matrix3> {
    (normal.length() > 1e-12).then(|| Matrix3::arbitrary_axis(normal))
}

/// The OCS point of the entity of `normal` that is the WCS point `wcs`.
pub(crate) fn to_ocs(normal: Vector3, wcs: Vector3) -> Vector3 {
    frame(normal).map_or(wcs, |m| m.transpose() * wcs)
}

/// The WCS point of the OCS point `ocs` of the entity of `normal`.
pub(crate) fn to_wcs(normal: Vector3, ocs: Vector3) -> Vector3 {
    frame(normal).map_or(ocs, |m| m * ocs)
}
