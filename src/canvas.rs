use crate::{
    Background, Color, Path, PathPrimitive, Point, Quad, Rect, Scene, Vector, scene::PaintLayerKey,
};

/// Immediate paint access scoped to one declarative canvas element.
///
/// Coordinates are local to the canvas. Commands are appended to the retained scene only when the
/// view is repainted, and every command is clipped to the canvas and its visible ancestors.
pub struct Canvas<'a> {
    scene: &'a mut Scene,
    layer: PaintLayerKey,
    origin: Point,
    bounds: Rect,
    clip: Rect,
}

impl<'a> Canvas<'a> {
    pub(crate) fn new(
        scene: &'a mut Scene,
        layer: PaintLayerKey,
        origin: Point,
        size: crate::Size,
        clip: Rect,
    ) -> Self {
        Self {
            scene,
            layer,
            origin,
            bounds: Rect::from_size(size),
            clip,
        }
    }

    /// The local drawable rectangle, starting at `(0, 0)`.
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Fill a local rectangle.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.scene.push_quad_in(
            self.layer,
            Quad::new(self.absolute_rect(rect), color).clip(self.clip),
        );
    }

    /// Fill a local rounded rectangle.
    pub fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.push_quad_in(
            self.layer,
            Quad::new(self.absolute_rect(rect), color)
                .radius(radius)
                .clip(self.clip),
        );
    }

    /// Paint a retained path in local canvas coordinates.
    pub fn paint_path(&mut self, path: impl Into<Path>, background: impl Into<Background>) {
        self.paint_path_at(path, Point::ZERO, background);
    }

    /// Paint a retained path translated by a local offset.
    pub fn paint_path_at(
        &mut self,
        path: impl Into<Path>,
        position: Point,
        background: impl Into<Background>,
    ) {
        self.scene.push_path_in(
            self.layer,
            PathPrimitive::new(path, background)
                .translate(self.origin.x + position.x, self.origin.y + position.y)
                .clip(self.clip),
        );
    }

    /// Paint a retained path with a local axis-aligned transform.
    pub fn paint_path_transformed(
        &mut self,
        path: impl Into<Path>,
        scale: [f32; 2],
        translation: Point,
        background: impl Into<Background>,
    ) {
        self.scene.push_path_in(
            self.layer,
            PathPrimitive::new(path, background)
                .scale_xy(scale[0], scale[1])
                .translate(self.origin.x + translation.x, self.origin.y + translation.y)
                .clip(self.clip),
        );
    }

    fn absolute_rect(&self, rect: Rect) -> Rect {
        rect.translate(Vector::new(self.origin.x, self.origin.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PathBuilder, Size};

    #[test]
    fn local_commands_translate_and_inherit_the_canvas_clip() {
        let mut scene = Scene::new();
        let mut builder = PathBuilder::fill();
        builder.move_to(Point::new(0.0, 0.0));
        builder.line_to(Point::new(10.0, 0.0));
        builder.line_to(Point::new(0.0, 10.0));
        builder.close();
        let path = builder.build().unwrap();
        let clip = Rect::new(12.0, 24.0, 30.0, 20.0);
        {
            let mut canvas = Canvas::new(
                &mut scene,
                PaintLayerKey::default(),
                Point::new(10.0, 20.0),
                Size::new(40.0, 30.0),
                clip,
            );
            assert_eq!(canvas.bounds(), Rect::new(0.0, 0.0, 40.0, 30.0));
            canvas.fill_rect(Rect::new(1.0, 2.0, 5.0, 6.0), Color::WHITE);
            canvas.paint_path_at(&path, Point::new(3.0, 4.0), Color::WHITE);
        }

        assert_eq!(scene.quads()[0].rect, Rect::new(11.0, 22.0, 5.0, 6.0));
        assert_eq!(scene.quads()[0].clip, Some(clip));
        assert_eq!(
            scene.paths()[0].render_bounds(),
            Rect::new(13.0, 24.0, 10.0, 10.0)
        );
        assert_eq!(scene.paths()[0].clip, Some(clip));
    }
}
