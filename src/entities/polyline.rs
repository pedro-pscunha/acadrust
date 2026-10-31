//! Polyline entities (2D and 3D polylines)

use super::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Handle, Vector2, Vector3};

/// Polyline flags (matches DXF group code 70)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolylineFlags {
    bits: u16,
}

impl PolylineFlags {
    pub const CLOSED: Self = Self { bits: 1 };
    pub const CURVE_FIT: Self = Self { bits: 2 };
    pub const SPLINE_FIT: Self = Self { bits: 4 };
    pub const POLYLINE_3D: Self = Self { bits: 8 };
    pub const POLYGON_MESH: Self = Self { bits: 16 };
    pub const CLOSED_N: Self = Self { bits: 32 };
    pub const POLYFACE_MESH: Self = Self { bits: 64 };
    pub const LINETYPE_CONTINUOUS: Self = Self { bits: 128 };

    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn from_bits(bits: u16) -> Self {
        Self { bits }
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn is_closed(&self) -> bool {
        self.bits & 1 != 0
    }

    pub fn is_3d(&self) -> bool {
        self.bits & 8 != 0
    }

    pub fn is_spline_fit(&self) -> bool {
        self.bits & 4 != 0
    }

    pub fn set_closed(&mut self, value: bool) {
        if value {
            self.bits |= 1;
        } else {
            self.bits &= !1;
        }
    }

    pub fn set_3d(&mut self, value: bool) {
        if value {
            self.bits |= 8;
        } else {
            self.bits &= !8;
        }
    }
}

impl std::ops::BitOr for PolylineFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl std::ops::BitOrAssign for PolylineFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

/// Vertex flags (matches DXF group code 70)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VertexFlags {
    bits: u8,
}

impl VertexFlags {
    pub const EXTRA_VERTEX: Self = Self { bits: 1 };
    pub const CURVE_FIT_TANGENT: Self = Self { bits: 2 };
    pub const SPLINE_VERTEX: Self = Self { bits: 8 };
    pub const SPLINE_CONTROL: Self = Self { bits: 16 };
    pub const POLYLINE_3D: Self = Self { bits: 32 };
    pub const POLYGON_MESH: Self = Self { bits: 64 };
    pub const POLYFACE_FACE: Self = Self { bits: 128 };

    pub fn new() -> Self {
        Self { bits: 0 }
    }

    pub fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub fn bits(&self) -> u8 {
        self.bits
    }
}

/// Smooth surface type (matches DXF group code 75)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SmoothSurfaceType {
    #[default]
    None = 0,
    QuadraticBSpline = 5,
    CubicBSpline = 6,
    Bezier = 8,
}

impl From<i16> for SmoothSurfaceType {
    fn from(value: i16) -> Self {
        match value {
            5 => SmoothSurfaceType::QuadraticBSpline,
            6 => SmoothSurfaceType::CubicBSpline,
            8 => SmoothSurfaceType::Bezier,
            _ => SmoothSurfaceType::None,
        }
    }
}

/// The common data a writer gives a child record (a VERTEX or the SEQEND) of the polyline whose
/// common data is `parent`: `extra` when the child has data of its own, else the polyline's layer,
/// colour, linetype, linetype scale and lineweight (AutoCAD keeps them equal and AUDIT reports any
/// other value). The handle is `handle`, the owner is the polyline, and an empty `layer` is the
/// polyline's.
pub(crate) fn child_common(
    parent: &EntityCommon,
    handle: Handle,
    layer: &str,
    extra: Option<&EntityCommon>,
) -> EntityCommon {
    let mut child = match extra {
        Some(extra) => extra.clone(),
        None => {
            let mut child = EntityCommon::new();
            child.color = parent.color;
            child.linetype = parent.linetype.clone();
            child.linetype_handle = parent.linetype_handle;
            child.line_weight = parent.line_weight;
            child.linetype_scale = parent.linetype_scale;
            child
        }
    };
    child.handle = handle;
    child.owner_handle = parent.handle;
    child.layer = if layer.is_empty() {
        parent.layer.clone()
    } else {
        layer.to_string()
    };
    child
}

/// The common data a writer gives a vertex of a polygon mesh, whose `common` always exists: the
/// vertex's own, with the mesh's color, linetype, lineweight and linetype scale where the vertex
/// has only the defaults (AutoCAD keeps them equal and AUDIT reports any other value). A vertex
/// cannot tell an own override from an absent one, so a default value is taken as absent.
pub(crate) fn mesh_vertex_common(vertex: &EntityCommon, parent: &EntityCommon) -> EntityCommon {
    let mut own = vertex.clone();
    if own.color == crate::types::Color::ByLayer {
        own.color = parent.color;
    }
    if !own.has_linetype() {
        own.linetype = parent.linetype.clone();
        own.linetype_handle = parent.linetype_handle;
    }
    if own.line_weight == crate::types::LineWeight::ByLayer {
        own.line_weight = parent.line_weight;
    }
    if (own.linetype_scale - 1.0).abs() < 1e-12 {
        own.linetype_scale = parent.linetype_scale;
    }
    own
}

/// Whether a child record's common data holds anything that the polyline's own data does not give
/// it (see [`child_common`]). A ByLayer child under a polyline of another colour has data of its
/// own: AutoCAD keeps it, and does not repair it on a save.
pub(crate) fn child_has_data(child: &EntityCommon, parent: &EntityCommon) -> bool {
    let plain = child.extended_data.is_empty()
        && child.extended_data.raw_dwg_eed.is_empty()
        && child.xdictionary_handle.is_none()
        && child.reactors.is_empty()
        && child.color == parent.color
        && child.linetype == parent.linetype
        && child.line_weight == parent.line_weight
        && (child.linetype_scale - parent.linetype_scale).abs() < 1e-12
        && !child.invisible
        && child.transparency.is_by_layer()
        && child.plotstyle_flags == 0
        && child.material_flags == 0
        && child.shadow_flags == 0;
    !plain
}

/// The common data of a child record read from a file, when it carries anything that the
/// polyline's own data does not give it (see [`child_common`]); `None` for a plain child.
pub(crate) fn child_extras(
    child: EntityCommon,
    parent: &EntityCommon,
) -> Option<Box<EntityCommon>> {
    child_has_data(&child, parent).then(|| Box::new(child))
}

/// The common data held by the child records (VERTEX, face records, SEQEND) of a polyline entity
/// of any kind: only the children that hold data of their own for a 2D and a 3D polyline.
pub(crate) fn child_commons(entity: &super::EntityType) -> Vec<&EntityCommon> {
    use super::EntityType;
    match entity {
        EntityType::Polyline2D(p) => p
            .vertices
            .iter()
            .filter_map(|v| v.common.as_deref())
            .chain(p.seqend.as_deref())
            .collect(),
        EntityType::Polyline3D(p) => p
            .vertices
            .iter()
            .filter_map(|v| v.common.as_deref())
            .chain(p.seqend.as_deref())
            .collect(),
        EntityType::PolygonMesh(m) => m
            .vertices
            .iter()
            .map(|v| &v.common)
            .chain(m.seqend.as_deref())
            .collect(),
        EntityType::PolyfaceMesh(m) => m
            .vertices
            .iter()
            .map(|v| &v.common)
            .chain(m.faces.iter().map(|f| &f.common))
            .chain(m.seqend.as_deref())
            .collect(),
        _ => Vec::new(),
    }
}

/// As [`child_commons`], mutable.
pub(crate) fn child_commons_mut(entity: &mut super::EntityType) -> Vec<&mut EntityCommon> {
    use super::EntityType;
    match entity {
        EntityType::Polyline2D(p) => p
            .vertices
            .iter_mut()
            .filter_map(|v| v.common.as_deref_mut())
            .chain(p.seqend.as_deref_mut())
            .collect(),
        EntityType::Polyline3D(p) => p
            .vertices
            .iter_mut()
            .filter_map(|v| v.common.as_deref_mut())
            .chain(p.seqend.as_deref_mut())
            .collect(),
        EntityType::PolygonMesh(m) => m
            .vertices
            .iter_mut()
            .map(|v| &mut v.common)
            .chain(m.seqend.as_deref_mut())
            .collect(),
        EntityType::PolyfaceMesh(m) => m
            .vertices
            .iter_mut()
            .map(|v| &mut v.common)
            .chain(m.faces.iter_mut().map(|f| &mut f.common))
            .chain(m.seqend.as_deref_mut())
            .collect(),
        _ => Vec::new(),
    }
}

/// A vertex in a 2D polyline
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex2D {
    /// Location of the vertex (X, Y in OCS, Z is elevation)
    pub location: Vector3,
    /// Vertex flags
    pub flags: VertexFlags,
    /// Start width (0 = use default)
    pub start_width: f64,
    /// End width (0 = use default)
    pub end_width: f64,
    /// Bulge (0 = straight segment, <0 = clockwise arc, >0 = counter-clockwise arc)
    pub bulge: f64,
    /// Curve fit tangent direction in radians
    pub curve_tangent: f64,
    /// Vertex ID (R2010+)
    pub id: i32,
    /// Handle of the VERTEX record: the null handle when it has none, and the writers then allocate
    /// one. A reference to the vertex (an XRECORD group 340, for example) names it.
    pub handle: Handle,
    /// Layer of the VERTEX record when it is not the polyline's (empty: the polyline's).
    pub layer: String,
    /// The other common data of the VERTEX record that is not the polyline's (colour, linetype,
    /// lineweight, XDATA, extension dictionary, reactors...), or `None` when it has none. Its
    /// `handle` and `layer` are not used: the fields above are.
    pub common: Option<Box<EntityCommon>>,
}

impl Vertex2D {
    pub fn new(location: Vector3) -> Self {
        Self {
            location,
            flags: VertexFlags::new(),
            start_width: 0.0,
            end_width: 0.0,
            bulge: 0.0,
            curve_tangent: 0.0,
            id: 0,
            handle: Handle::NULL,
            layer: String::new(),
            common: None,
        }
    }

    pub fn from_point(point: Vector2) -> Self {
        Self::new(Vector3::new(point.x, point.y, 0.0))
    }

    pub fn with_bulge(mut self, bulge: f64) -> Self {
        self.bulge = bulge;
        self
    }

    pub fn with_width(mut self, start_width: f64, end_width: f64) -> Self {
        self.start_width = start_width;
        self.end_width = end_width;
        self
    }
}

/// A vertex in a 3D polyline
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex3D {
    /// Location of the vertex
    pub location: Vector3,
    /// Vertex flags
    pub flags: VertexFlags,
}

impl Vertex3D {
    /// Create a new vertex
    pub fn new(location: Vector3) -> Self {
        Self {
            location,
            flags: VertexFlags::new(),
        }
    }

    /// Create a vertex from coordinates
    pub fn from_coords(x: f64, y: f64, z: f64) -> Self {
        Vertex3D::new(Vector3::new(x, y, z))
    }
}

/// A 2D polyline entity (heavy polyline with vertices)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline2D {
    /// Common entity data
    pub common: EntityCommon,
    /// Polyline flags
    pub flags: PolylineFlags,
    /// Smooth surface type
    pub smooth_surface: SmoothSurfaceType,
    /// Default start width
    pub start_width: f64,
    /// Default end width
    pub end_width: f64,
    /// Thickness (extrusion height)
    pub thickness: f64,
    /// Elevation (Z coordinate in OCS)
    pub elevation: f64,
    /// Normal vector (extrusion direction)
    pub normal: Vector3,
    /// Vertices
    pub vertices: Vec<Vertex2D>,
    /// The common data of the SEQEND record that ends the vertices (its handle, layer, XDATA...),
    /// or `None` when the polyline has none and the writers generate it.
    pub seqend: Option<Box<EntityCommon>>,
}

impl Polyline2D {
    pub fn new() -> Self {
        Self {
            common: EntityCommon::new(),
            flags: PolylineFlags::new(),
            smooth_surface: SmoothSurfaceType::None,
            start_width: 0.0,
            end_width: 0.0,
            thickness: 0.0,
            elevation: 0.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
            vertices: Vec::new(),
            seqend: None,
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex2D) {
        self.vertices.push(vertex);
    }

    pub fn is_closed(&self) -> bool {
        self.flags.is_closed()
    }

    pub fn close(&mut self) {
        self.flags.set_closed(true);
    }
}

impl Default for Polyline2D {
    fn default() -> Self {
        Self::new()
    }
}

/// A 3D polyline entity
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline {
    /// Common entity data
    pub common: EntityCommon,
    /// Polyline flags
    pub flags: PolylineFlags,
    /// Vertices of the polyline
    pub vertices: Vec<Vertex3D>,
}

impl Polyline {
    /// Create a new empty polyline
    pub fn new() -> Self {
        let mut flags = PolylineFlags::new();
        flags.set_3d(true);
        Polyline {
            common: EntityCommon::new(),
            flags,
            vertices: Vec::new(),
        }
    }

    /// Create a polyline from a list of points
    pub fn from_points(points: Vec<Vector3>) -> Self {
        Polyline {
            vertices: points.into_iter().map(Vertex3D::new).collect(),
            ..Self::new()
        }
    }

    /// Add a vertex to the polyline
    pub fn add_vertex(&mut self, vertex: Vertex3D) {
        self.vertices.push(vertex);
    }

    /// Add a point to the polyline
    pub fn add_point(&mut self, point: Vector3) {
        self.vertices.push(Vertex3D::new(point));
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Check if closed
    pub fn is_closed(&self) -> bool {
        self.flags.is_closed()
    }

    /// Close the polyline
    pub fn close(&mut self) {
        self.flags.set_closed(true);
    }
}

impl Default for Polyline {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for Polyline2D {
    fn common(&self) -> &super::EntityCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::EntityCommon {
        &mut self.common
    }

    fn bounding_box(&self) -> BoundingBox3D {
        if self.vertices.is_empty() {
            return BoundingBox3D::from_point(Vector3::ZERO);
        }

        let points: Vec<Vector3> = self.vertices.iter().map(|v| v.location).collect();
        BoundingBox3D::from_points(&points)
            .unwrap()
            .ocs_to_wcs(self.normal)
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_polyline2d(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "POLYLINE"
    }

    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_polyline2d(self, transform);
    }

    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        super::mirror::mirror_polyline2d(self, transform);
    }
}

impl Entity for Polyline {
    fn common(&self) -> &super::EntityCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::EntityCommon {
        &mut self.common
    }

    fn bounding_box(&self) -> BoundingBox3D {
        if self.vertices.is_empty() {
            return BoundingBox3D::from_point(Vector3::ZERO);
        }

        let points: Vec<Vector3> = self.vertices.iter().map(|v| v.location).collect();
        BoundingBox3D::from_points(&points).unwrap()
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_polyline(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "POLYLINE"
    }

    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_polyline(self, transform);
    }
}
