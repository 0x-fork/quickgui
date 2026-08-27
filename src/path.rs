use std::{
    collections::HashMap,
    fmt, mem,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use lyon::{
    algorithms::measure::{PathMeasurements, SampleType},
    geom::Angle,
    math::{Transform, point, vector},
    path::{ArcFlags, Path as LyonPath, Polygon, traits::SvgPathBuilder},
    tessellation::{
        FillGeometryBuilder, FillOptions as LyonFillOptions, FillTessellator, FillVertex,
        GeometryBuilder, GeometryBuilderError, StrokeGeometryBuilder,
        StrokeOptions as LyonStrokeOptions, StrokeTessellator, StrokeVertex, TessellationError,
        VertexId,
    },
};
use thiserror::Error;

use crate::{Color, Point, Rect, Size};

/// Maximum path-building commands accepted before tessellation.
pub const MAX_PATH_COMMANDS: usize = 65_536;
/// Maximum de-indexed vertices retained by one path.
pub const MAX_PATH_VERTICES: usize = 196_605;
/// Maximum retained triangle-position and boundary metadata owned by one path.
pub const MAX_PATH_BYTES: usize = 4 * 1024 * 1024;
/// Maximum dash segments generated from one stroked path.
pub const MAX_PATH_DASH_SEGMENTS: usize = 262_144;
/// Largest absolute source coordinate accepted by the builder.
pub const MAX_PATH_COORDINATE: f32 = 1_000_000.0;

const MAX_STROKE_WIDTH: f32 = 100_000.0;
const MAX_MITER_LIMIT: f32 = 1_000.0;
const MIN_TOLERANCE: f32 = 0.01;
const MAX_TOLERANCE: f32 = 10.0;
const MAX_DASH_VALUES: usize = 256;
const MAX_TRANSFORM_SCALE: f32 = 1_024.0;

static NEXT_PATH_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineCap {
    Butt,
    Square,
    Round,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineJoin {
    Miter,
    MiterClip,
    Round,
    Bevel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillOptions {
    pub fill_rule: FillRule,
    pub tolerance: f32,
}

impl FillOptions {
    pub const fn new() -> Self {
        Self {
            fill_rule: FillRule::NonZero,
            tolerance: 0.1,
        }
    }

    pub fn with_fill_rule(mut self, fill_rule: FillRule) -> Self {
        self.fill_rule = fill_rule;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl Default for FillOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeOptions {
    pub line_width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
    pub miter_limit: f32,
    pub tolerance: f32,
}

impl StrokeOptions {
    pub const fn new() -> Self {
        Self {
            line_width: 1.0,
            line_cap: LineCap::Butt,
            line_join: LineJoin::Miter,
            miter_limit: 4.0,
            tolerance: 0.1,
        }
    }

    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    pub fn with_line_cap(mut self, cap: LineCap) -> Self {
        self.line_cap = cap;
        self
    }

    pub fn with_line_join(mut self, join: LineJoin) -> Self {
        self.line_join = join;
        self
    }

    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }

    pub fn with_tolerance(mut self, tolerance: f32) -> Self {
        self.tolerance = tolerance;
        self
    }
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathStyle {
    Fill(FillOptions),
    Stroke(StrokeOptions),
}

/// Color interpolation used by a two-stop linear gradient.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GradientColorSpace {
    /// Interpolate the framework's linear-light RGB values directly.
    #[default]
    LinearSrgb,
    /// Interpolate encoded sRGB values, matching traditional CSS gradients.
    Srgb,
    /// Interpolate perceptually in Oklab.
    Oklab,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearColorStop {
    pub color: Color,
    pub position: f32,
}

impl LinearColorStop {
    pub fn new(color: Color, position: f32) -> Self {
        Self {
            color: sanitize_color(color),
            position: finite_or(position, 0.0).clamp(0.0, 1.0),
        }
    }
}

pub fn linear_color_stop(color: Color, position: f32) -> LinearColorStop {
    LinearColorStop::new(color, position)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearGradient {
    angle_degrees: f32,
    stops: [LinearColorStop; 2],
    color_space: GradientColorSpace,
}

impl LinearGradient {
    pub fn new(angle_degrees: f32, mut start: LinearColorStop, mut end: LinearColorStop) -> Self {
        if start.position > end.position {
            mem::swap(&mut start, &mut end);
        }
        Self {
            angle_degrees: finite_or(angle_degrees, 180.0).rem_euclid(360.0),
            stops: [start, end],
            color_space: GradientColorSpace::LinearSrgb,
        }
    }

    pub fn color_space(mut self, color_space: GradientColorSpace) -> Self {
        self.color_space = color_space;
        self
    }

    pub const fn angle_degrees(self) -> f32 {
        self.angle_degrees
    }

    pub const fn stops(self) -> [LinearColorStop; 2] {
        self.stops
    }

    pub const fn interpolation(self) -> GradientColorSpace {
        self.color_space
    }
}

pub fn linear_gradient(
    angle_degrees: f32,
    start: LinearColorStop,
    end: LinearColorStop,
) -> Background {
    Background::LinearGradient(LinearGradient::new(angle_degrees, start, end))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Background {
    Solid(Color),
    LinearGradient(LinearGradient),
}

impl Background {
    pub(crate) fn is_visible(self) -> bool {
        match self {
            Self::Solid(color) => color.a > 0.0,
            Self::LinearGradient(gradient) => gradient.stops.iter().any(|stop| stop.color.a > 0.0),
        }
    }

    pub(crate) fn multiply_alpha(self, opacity: f32) -> Self {
        match self {
            Self::Solid(color) => Self::Solid(color.multiply_alpha(opacity)),
            Self::LinearGradient(mut gradient) => {
                for stop in &mut gradient.stops {
                    stop.color = stop.color.multiply_alpha(opacity);
                }
                Self::LinearGradient(gradient)
            }
        }
    }
}

impl From<Color> for Background {
    fn from(color: Color) -> Self {
        Self::Solid(sanitize_color(color))
    }
}

impl From<LinearGradient> for Background {
    fn from(gradient: LinearGradient) -> Self {
        Self::LinearGradient(gradient)
    }
}

#[derive(Clone)]
pub struct Path(Arc<PathData>);

struct PathData {
    id: u64,
    /// Three positions per triangle, in source logical coordinates.
    positions: Arc<[[f32; 2]]>,
    /// One three-bit mask per triangle. Bit N marks the edge opposite vertex N as an outline.
    boundary_masks: Arc<[u8]>,
    bounds: Rect,
}

impl Path {
    fn from_geometry(geometry: LimitedGeometry) -> Result<Self, PathError> {
        if geometry.overflowed {
            return Err(PathError::TooComplex {
                vertices: geometry.indices.len(),
                maximum: MAX_PATH_VERTICES,
            });
        }
        if geometry.invalid_triangle {
            return Err(PathError::InvalidGeometry);
        }
        if geometry.indices.len() > MAX_PATH_VERTICES {
            return Err(PathError::TooComplex {
                vertices: geometry.indices.len(),
                maximum: MAX_PATH_VERTICES,
            });
        }

        let mut edge_counts = HashMap::<(u32, u32), u8>::with_capacity(geometry.indices.len());
        for triangle in geometry.indices.chunks_exact(3) {
            for (first, second) in [
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
                (triangle[0], triangle[1]),
            ] {
                let edge = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                let count = edge_counts.entry(edge).or_default();
                *count = count.saturating_add(1);
            }
        }

        let mut positions = Vec::with_capacity(geometry.indices.len());
        let mut boundary_masks = Vec::with_capacity(geometry.indices.len() / 3);
        let mut minimum = [f32::INFINITY; 2];
        let mut maximum = [f32::NEG_INFINITY; 2];
        for triangle in geometry.indices.chunks_exact(3) {
            let edges = [
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
                (triangle[0], triangle[1]),
            ];
            let mut mask = 0_u8;
            for (edge_index, (first, second)) in edges.into_iter().enumerate() {
                let edge = if first <= second {
                    (first, second)
                } else {
                    (second, first)
                };
                if edge_counts.get(&edge) == Some(&1) {
                    mask |= 1 << edge_index;
                }
            }
            boundary_masks.push(mask);
            for index in triangle {
                let position = geometry.vertices[*index as usize];
                if !valid_coordinate(position[0]) || !valid_coordinate(position[1]) {
                    return Err(PathError::InvalidGeometry);
                }
                minimum[0] = minimum[0].min(position[0]);
                minimum[1] = minimum[1].min(position[1]);
                maximum[0] = maximum[0].max(position[0]);
                maximum[1] = maximum[1].max(position[1]);
                positions.push(position);
            }
        }

        let bytes = positions
            .len()
            .saturating_mul(mem::size_of::<[f32; 2]>())
            .saturating_add(boundary_masks.len());
        if bytes > MAX_PATH_BYTES {
            return Err(PathError::TooLarge {
                bytes,
                maximum: MAX_PATH_BYTES,
            });
        }
        let bounds = if positions.is_empty() {
            Rect::ZERO
        } else {
            Rect::new(
                minimum[0],
                minimum[1],
                maximum[0] - minimum[0],
                maximum[1] - minimum[1],
            )
        };
        Ok(Self(Arc::new(PathData {
            id: NEXT_PATH_ID.fetch_add(1, Ordering::Relaxed),
            positions: positions.into(),
            boundary_masks: boundary_masks.into(),
            bounds,
        })))
    }

    pub fn bounds(&self) -> Rect {
        self.0.bounds
    }

    pub fn size(&self) -> Size {
        Size::new(self.0.bounds.width, self.0.bounds.height)
    }

    pub fn is_empty(&self) -> bool {
        self.0.positions.is_empty()
    }

    pub fn triangle_count(&self) -> usize {
        self.0.boundary_masks.len()
    }

    pub fn vertex_count(&self) -> usize {
        self.0.positions.len()
    }

    pub fn byte_len(&self) -> usize {
        self.0.positions.len() * mem::size_of::<[f32; 2]>() + self.0.boundary_masks.len()
    }

    pub(crate) fn positions(&self) -> &[[f32; 2]] {
        &self.0.positions
    }

    pub(crate) fn boundary_masks(&self) -> &[u8] {
        &self.0.boundary_masks
    }
}

impl From<&Path> for Path {
    fn from(path: &Path) -> Self {
        path.clone()
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Path")
            .field("bounds", &self.bounds())
            .field("triangles", &self.triangle_count())
            .field("bytes", &self.byte_len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Path {}

pub struct PathBuilder {
    raw: lyon::path::builder::WithSvg<lyon::path::BuilderImpl>,
    style: PathStyle,
    dash_array: Option<Vec<f32>>,
    transform: Option<Transform>,
    commands: usize,
    error: Option<PathError>,
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::fill()
    }
}

impl PathBuilder {
    pub fn fill() -> Self {
        Self {
            raw: LyonPath::svg_builder(),
            style: PathStyle::Fill(FillOptions::default()),
            dash_array: None,
            transform: None,
            commands: 0,
            error: None,
        }
    }

    pub fn stroke(width: f32) -> Self {
        Self {
            style: PathStyle::Stroke(StrokeOptions::default().with_line_width(width)),
            ..Self::fill()
        }
    }

    pub fn with_style(mut self, style: PathStyle) -> Self {
        self.style = style;
        self
    }

    pub fn dash_array(mut self, values: &[f32]) -> Self {
        if values.is_empty() {
            self.dash_array = None;
            return self;
        }
        let output_len = if values.len().is_multiple_of(2) {
            values.len()
        } else {
            values.len().saturating_mul(2)
        };
        if output_len > MAX_DASH_VALUES
            || values
                .iter()
                .any(|value| !value.is_finite() || *value <= 0.0 || *value > MAX_PATH_COORDINATE)
        {
            self.record_error(PathError::InvalidDashPattern);
            return self;
        }
        let mut dash_array = Vec::with_capacity(output_len);
        dash_array.extend_from_slice(values);
        if !values.len().is_multiple_of(2) {
            dash_array.extend_from_slice(values);
        }
        self.dash_array = Some(dash_array);
        self
    }

    pub fn move_to(&mut self, to: Point) {
        if self.accept_points(&[to]) {
            self.raw.move_to(point(to.x, to.y));
        }
    }

    pub fn line_to(&mut self, to: Point) {
        if self.accept_points(&[to]) {
            self.raw.line_to(point(to.x, to.y));
        }
    }

    pub fn quadratic_to(&mut self, control: Point, to: Point) {
        if self.accept_points(&[control, to]) {
            self.raw
                .quadratic_bezier_to(point(control.x, control.y), point(to.x, to.y));
        }
    }

    /// GPUI-compatible argument order: destination followed by the control point.
    pub fn curve_to(&mut self, to: Point, control: Point) {
        self.quadratic_to(control, to);
    }

    pub fn cubic_to(&mut self, control_a: Point, control_b: Point, to: Point) {
        if self.accept_points(&[control_a, control_b, to]) {
            self.raw.cubic_bezier_to(
                point(control_a.x, control_a.y),
                point(control_b.x, control_b.y),
                point(to.x, to.y),
            );
        }
    }

    /// GPUI-compatible argument order: destination followed by both control points.
    pub fn cubic_bezier_to(&mut self, to: Point, control_a: Point, control_b: Point) {
        self.cubic_to(control_a, control_b, to);
    }

    pub fn arc_to(
        &mut self,
        radii: Size,
        x_rotation_degrees: f32,
        large_arc: bool,
        sweep: bool,
        to: Point,
    ) {
        if !valid_nonnegative_coordinate(radii.width)
            || !valid_nonnegative_coordinate(radii.height)
            || !x_rotation_degrees.is_finite()
        {
            self.record_error(PathError::InvalidCoordinate);
            return;
        }
        if self.accept_points(&[to]) {
            self.raw.arc_to(
                vector(radii.width, radii.height),
                Angle::degrees(x_rotation_degrees),
                ArcFlags { large_arc, sweep },
                point(to.x, to.y),
            );
        }
    }

    pub fn relative_arc_to(
        &mut self,
        radii: Size,
        x_rotation_degrees: f32,
        large_arc: bool,
        sweep: bool,
        to: Point,
    ) {
        if !valid_nonnegative_coordinate(radii.width)
            || !valid_nonnegative_coordinate(radii.height)
            || !x_rotation_degrees.is_finite()
        {
            self.record_error(PathError::InvalidCoordinate);
            return;
        }
        if self.accept_points(&[to]) {
            self.raw.relative_arc_to(
                vector(radii.width, radii.height),
                Angle::degrees(x_rotation_degrees),
                ArcFlags { large_arc, sweep },
                vector(to.x, to.y),
            );
        }
    }

    pub fn add_polygon(&mut self, points: &[Point], closed: bool) {
        if points.is_empty() {
            return;
        }
        if self.commands.saturating_add(points.len()) > MAX_PATH_COMMANDS
            || points.iter().any(|point| !valid_point(*point))
        {
            self.record_error(
                if self.commands.saturating_add(points.len()) > MAX_PATH_COMMANDS {
                    PathError::TooManyCommands {
                        commands: self.commands.saturating_add(points.len()),
                        maximum: MAX_PATH_COMMANDS,
                    }
                } else {
                    PathError::InvalidCoordinate
                },
            );
            return;
        }
        self.commands += points.len();
        let lyon_points = points
            .iter()
            .map(|source_point| point(source_point.x, source_point.y))
            .collect::<Vec<_>>();
        self.raw.add_polygon(Polygon {
            points: &lyon_points,
            closed,
        });
    }

    pub fn close(&mut self) {
        if self.accept_command() {
            self.raw.close();
        }
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        if !valid_coordinate(x) || !valid_coordinate(y) {
            self.record_error(PathError::InvalidTransform);
            return;
        }
        self.transform = Some(
            self.transform
                .unwrap_or_else(Transform::identity)
                .then_translate(vector(x, y)),
        );
    }

    pub fn scale(&mut self, scale: f32) {
        self.scale_xy(scale, scale);
    }

    pub fn scale_xy(&mut self, x: f32, y: f32) {
        if !x.is_finite()
            || !y.is_finite()
            || x.abs() > MAX_TRANSFORM_SCALE
            || y.abs() > MAX_TRANSFORM_SCALE
        {
            self.record_error(PathError::InvalidTransform);
            return;
        }
        self.transform = Some(
            self.transform
                .unwrap_or_else(Transform::identity)
                .then_scale(x, y),
        );
    }

    pub fn rotate(&mut self, radians: f32) {
        if !radians.is_finite() {
            self.record_error(PathError::InvalidTransform);
            return;
        }
        self.transform = Some(
            self.transform
                .unwrap_or_else(Transform::identity)
                .then_rotate(Angle::radians(radians)),
        );
    }

    pub fn build(self) -> Result<Path, PathError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        validate_style(self.style)?;
        let path = match self.transform {
            Some(transform) => self.raw.build().transformed(&transform),
            None => self.raw.build(),
        };
        let dashed;
        let path = if let (PathStyle::Stroke(_), Some(pattern)) = (self.style, self.dash_array) {
            dashed = dashed_path(&path, &pattern)?;
            &dashed
        } else {
            &path
        };
        let mut geometry = LimitedGeometry::default();
        let result = match self.style {
            PathStyle::Fill(options) => {
                let options = LyonFillOptions::default()
                    .with_fill_rule(match options.fill_rule {
                        FillRule::NonZero => lyon::path::FillRule::NonZero,
                        FillRule::EvenOdd => lyon::path::FillRule::EvenOdd,
                    })
                    .with_tolerance(options.tolerance);
                FillTessellator::new().tessellate_path(path, &options, &mut geometry)
            }
            PathStyle::Stroke(options) => {
                let options = LyonStrokeOptions::default()
                    .with_line_width(options.line_width)
                    .with_line_cap(match options.line_cap {
                        LineCap::Butt => lyon::path::LineCap::Butt,
                        LineCap::Square => lyon::path::LineCap::Square,
                        LineCap::Round => lyon::path::LineCap::Round,
                    })
                    .with_line_join(match options.line_join {
                        LineJoin::Miter => lyon::path::LineJoin::Miter,
                        LineJoin::MiterClip => lyon::path::LineJoin::MiterClip,
                        LineJoin::Round => lyon::path::LineJoin::Round,
                        LineJoin::Bevel => lyon::path::LineJoin::Bevel,
                    })
                    .with_miter_limit(options.miter_limit)
                    .with_tolerance(options.tolerance);
                StrokeTessellator::new().tessellate_path(path, &options, &mut geometry)
            }
        };
        if geometry.overflowed {
            return Err(PathError::TooComplex {
                vertices: geometry.indices.len().saturating_add(3),
                maximum: MAX_PATH_VERTICES,
            });
        }
        result.map_err(PathError::Tessellation)?;
        Path::from_geometry(geometry)
    }

    fn accept_points(&mut self, points: &[Point]) -> bool {
        if points.iter().any(|point| !valid_point(*point)) {
            self.record_error(PathError::InvalidCoordinate);
            return false;
        }
        self.accept_command()
    }

    fn accept_command(&mut self) -> bool {
        if self.error.is_some() {
            return false;
        }
        if self.commands == MAX_PATH_COMMANDS {
            self.record_error(PathError::TooManyCommands {
                commands: self.commands.saturating_add(1),
                maximum: MAX_PATH_COMMANDS,
            });
            return false;
        }
        self.commands += 1;
        true
    }

    fn record_error(&mut self, error: PathError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
}

fn validate_style(style: PathStyle) -> Result<(), PathError> {
    let tolerance = match style {
        PathStyle::Fill(options) => options.tolerance,
        PathStyle::Stroke(options) => {
            if !options.line_width.is_finite()
                || options.line_width <= 0.0
                || options.line_width > MAX_STROKE_WIDTH
            {
                return Err(PathError::InvalidStrokeWidth(options.line_width));
            }
            if !options.miter_limit.is_finite()
                || options.miter_limit < LyonStrokeOptions::MINIMUM_MITER_LIMIT
                || options.miter_limit > MAX_MITER_LIMIT
            {
                return Err(PathError::InvalidMiterLimit(options.miter_limit));
            }
            options.tolerance
        }
    };
    if !tolerance.is_finite() || !(MIN_TOLERANCE..=MAX_TOLERANCE).contains(&tolerance) {
        return Err(PathError::InvalidTolerance(tolerance));
    }
    Ok(())
}

fn dashed_path(path: &LyonPath, pattern: &[f32]) -> Result<LyonPath, PathError> {
    debug_assert!(!pattern.is_empty());
    let measurements = PathMeasurements::from_path(path, MIN_TOLERANCE);
    let total_length = measurements.length();
    if !total_length.is_finite() {
        return Err(PathError::InvalidGeometry);
    }
    if total_length <= 0.0 {
        return Ok(path.clone());
    }
    let minimum_dash = pattern.iter().copied().fold(f32::INFINITY, f32::min);
    let estimated_segments = (total_length / minimum_dash).ceil() as usize;
    if estimated_segments > MAX_PATH_DASH_SEGMENTS {
        return Err(PathError::TooManyDashSegments {
            segments: estimated_segments,
            maximum: MAX_PATH_DASH_SEGMENTS,
        });
    }

    let mut sampler = measurements.create_sampler(path, SampleType::Normalized);
    let mut builder = LyonPath::builder();
    let mut position = 0.0;
    let mut dash_index = 0_usize;
    while position < total_length {
        if dash_index == MAX_PATH_DASH_SEGMENTS {
            return Err(PathError::TooManyDashSegments {
                segments: dash_index.saturating_add(1),
                maximum: MAX_PATH_DASH_SEGMENTS,
            });
        }
        let end = (position + pattern[dash_index % pattern.len()]).min(total_length);
        if dash_index.is_multiple_of(2) {
            sampler.split_range(position / total_length..end / total_length, &mut builder);
        }
        position = end;
        dash_index += 1;
    }
    Ok(builder.build())
}

#[derive(Default)]
struct LimitedGeometry {
    vertices: Vec<[f32; 2]>,
    indices: Vec<u32>,
    overflowed: bool,
    invalid_triangle: bool,
}

impl GeometryBuilder for LimitedGeometry {
    fn add_triangle(&mut self, a: VertexId, b: VertexId, c: VertexId) {
        if self.indices.len().saturating_add(3) > MAX_PATH_VERTICES {
            self.overflowed = true;
            return;
        }
        let indices = [a.offset(), b.offset(), c.offset()];
        if indices
            .iter()
            .any(|index| *index as usize >= self.vertices.len())
        {
            self.invalid_triangle = true;
            return;
        }
        self.indices.extend_from_slice(&indices);
    }
}

impl FillGeometryBuilder for LimitedGeometry {
    fn add_fill_vertex(&mut self, vertex: FillVertex) -> Result<VertexId, GeometryBuilderError> {
        self.add_vertex(vertex.position().to_array())
    }
}

impl StrokeGeometryBuilder for LimitedGeometry {
    fn add_stroke_vertex(
        &mut self,
        vertex: StrokeVertex,
    ) -> Result<VertexId, GeometryBuilderError> {
        self.add_vertex(vertex.position().to_array())
    }
}

impl LimitedGeometry {
    fn add_vertex(&mut self, position: [f32; 2]) -> Result<VertexId, GeometryBuilderError> {
        if self.vertices.len() >= MAX_PATH_VERTICES {
            self.overflowed = true;
            return Err(GeometryBuilderError::TooManyVertices);
        }
        if !valid_coordinate(position[0]) || !valid_coordinate(position[1]) {
            return Err(GeometryBuilderError::InvalidVertex);
        }
        let id = VertexId::from_usize(self.vertices.len());
        self.vertices.push(position);
        Ok(id)
    }
}

fn valid_point(point: Point) -> bool {
    valid_coordinate(point.x) && valid_coordinate(point.y)
}

fn valid_coordinate(value: f32) -> bool {
    value.is_finite() && value.abs() <= MAX_PATH_COORDINATE
}

fn valid_nonnegative_coordinate(value: f32) -> bool {
    value.is_finite() && (0.0..=MAX_PATH_COORDINATE).contains(&value)
}

fn sanitize_color(color: Color) -> Color {
    Color::linear(
        finite_or(color.r, 0.0).clamp(0.0, 1.0),
        finite_or(color.g, 0.0).clamp(0.0, 1.0),
        finite_or(color.b, 0.0).clamp(0.0, 1.0),
        finite_or(color.a, 0.0).clamp(0.0, 1.0),
    )
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("path coordinates must be finite and within +/-{MAX_PATH_COORDINATE}")]
    InvalidCoordinate,
    #[error("path transforms must be finite and within the supported range")]
    InvalidTransform,
    #[error("stroke width {0} must be finite and in (0, {MAX_STROKE_WIDTH}]")]
    InvalidStrokeWidth(f32),
    #[error(
        "miter limit {0} must be finite and between {minimum} and {MAX_MITER_LIMIT}",
        minimum = LyonStrokeOptions::MINIMUM_MITER_LIMIT
    )]
    InvalidMiterLimit(f32),
    #[error("tolerance {0} must be finite and between {MIN_TOLERANCE} and {MAX_TOLERANCE}")]
    InvalidTolerance(f32),
    #[error(
        "dash values must be finite, positive, bounded, and contain at most {MAX_DASH_VALUES} values"
    )]
    InvalidDashPattern,
    #[error("path contains {commands} commands; the maximum is {maximum}")]
    TooManyCommands { commands: usize, maximum: usize },
    #[error("dashing would generate {segments} segments; the maximum is {maximum}")]
    TooManyDashSegments { segments: usize, maximum: usize },
    #[error("path tessellation generated {vertices} vertices; the maximum is {maximum}")]
    TooComplex { vertices: usize, maximum: usize },
    #[error("retained path data is {bytes} bytes; the maximum is {maximum} bytes")]
    TooLarge { bytes: usize, maximum: usize },
    #[error("path tessellation produced invalid geometry")]
    InvalidGeometry,
    #[error("path tessellation failed: {0}")]
    Tessellation(#[source] TessellationError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_tessellates_once_and_clones_share_identity() {
        let mut builder = PathBuilder::fill();
        builder.move_to(Point::new(10.0, 20.0));
        builder.line_to(Point::new(50.0, 20.0));
        builder.line_to(Point::new(50.0, 60.0));
        builder.close();
        let path = builder.build().unwrap();

        assert_eq!(path.triangle_count(), 1);
        assert_eq!(path.vertex_count(), 3);
        assert_eq!(path.bounds(), Rect::new(10.0, 20.0, 40.0, 40.0));
        assert_eq!(path, path.clone());
    }

    #[test]
    fn internal_triangle_edges_are_not_antialiased() {
        let mut builder = PathBuilder::fill();
        builder.add_polygon(
            &[
                Point::new(0.0, 0.0),
                Point::new(20.0, 0.0),
                Point::new(20.0, 20.0),
                Point::new(0.0, 20.0),
            ],
            true,
        );
        let path = builder.build().unwrap();

        assert_eq!(path.triangle_count(), 2);
        assert_eq!(
            path.boundary_masks()
                .iter()
                .map(|mask| mask.count_ones())
                .sum::<u32>(),
            4
        );
    }

    #[test]
    fn strokes_support_safe_dash_patterns_and_curve_commands() {
        let mut builder = PathBuilder::stroke(3.0)
            .with_style(PathStyle::Stroke(
                StrokeOptions::default()
                    .with_line_width(3.0)
                    .with_line_cap(LineCap::Round)
                    .with_line_join(LineJoin::Bevel),
            ))
            .dash_array(&[6.0, 3.0]);
        builder.move_to(Point::new(0.0, 10.0));
        builder.quadratic_to(Point::new(25.0, -10.0), Point::new(50.0, 10.0));
        builder.cubic_to(
            Point::new(60.0, 30.0),
            Point::new(80.0, -10.0),
            Point::new(100.0, 10.0),
        );
        let path = builder.build().unwrap();
        assert!(!path.is_empty());
        assert!(path.bounds().height > 3.0);
    }

    #[test]
    fn zero_or_non_finite_dash_values_are_rejected_without_looping() {
        let mut builder = PathBuilder::stroke(1.0).dash_array(&[4.0, 0.0]);
        builder.move_to(Point::ZERO);
        builder.line_to(Point::new(10.0, 0.0));
        assert!(matches!(
            builder.build(),
            Err(PathError::InvalidDashPattern)
        ));

        let mut builder = PathBuilder::stroke(1.0).dash_array(&[f32::NAN]);
        builder.move_to(Point::ZERO);
        builder.line_to(Point::new(10.0, 0.0));
        assert!(matches!(
            builder.build(),
            Err(PathError::InvalidDashPattern)
        ));
    }

    #[test]
    fn invalid_geometry_and_styles_fail_at_the_api_boundary() {
        let mut invalid_point = PathBuilder::fill();
        invalid_point.move_to(Point::new(f32::INFINITY, 0.0));
        assert!(matches!(
            invalid_point.build(),
            Err(PathError::InvalidCoordinate)
        ));

        let invalid_stroke = PathBuilder::stroke(0.0).build();
        assert!(matches!(
            invalid_stroke,
            Err(PathError::InvalidStrokeWidth(0.0))
        ));

        let invalid_tolerance = PathBuilder::fill()
            .with_style(PathStyle::Fill(FillOptions::default().with_tolerance(0.0)))
            .build();
        assert!(matches!(
            invalid_tolerance,
            Err(PathError::InvalidTolerance(0.0))
        ));
    }

    #[test]
    fn gradients_sanitize_angles_stops_and_colors() {
        let gradient = LinearGradient::new(
            f32::NAN,
            LinearColorStop::new(Color::linear(f32::NAN, 2.0, -1.0, 2.0), 2.0),
            LinearColorStop::new(Color::WHITE, -1.0),
        );
        assert_eq!(gradient.angle_degrees(), 180.0);
        assert_eq!(gradient.stops()[0].position, 0.0);
        assert_eq!(gradient.stops()[1].position, 1.0);
        assert_eq!(gradient.stops()[1].color, Color::linear(0.0, 1.0, 0.0, 1.0));
    }

    #[test]
    fn empty_paths_are_valid_and_allocation_small() {
        let path = PathBuilder::fill().build().unwrap();
        assert!(path.is_empty());
        assert_eq!(path.bounds(), Rect::ZERO);
        assert_eq!(path.byte_len(), 0);
    }

    #[test]
    fn constants_keep_retained_and_gpu_expansion_bounded() {
        assert_eq!(MAX_PATH_VERTICES % 3, 0);
        assert!(MAX_PATH_VERTICES * mem::size_of::<[f32; 2]>() <= MAX_PATH_BYTES);
    }
}
