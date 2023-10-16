use std::io;
use sfml::graphics::{Drawable, RenderTarget, Shape, Transformable};
use super::sf;

const LINE_THICKNESS: f32 = 2.0;
const LINE_DETECTION_DISTANCE: f32 = 10.0;
const POINT_RADIUS: f32 = 5.0;
const LINES_COLOR: sf::Color = sf::Color::rgb(180, 180, 179);
const LINES_COLOR_INCORRECT: sf::Color = sf::Color::rgb(237, 123, 123);
const POLY_EDGE_MIN_LEN: f32 = 5.;
const POINTS_COLOR: sf::Color = sf::Color::rgb(247, 233, 135);
const POINT_DETECTION_RADIUS: f32 = 10.0;
const POINT_DETECTION_COLOR_CORRECT: sf::Color = sf::Color::rgb(100, 204, 197);
const POINT_DETECTION_COLOR_INCORRECT: sf::Color = sf::Color::rgb(237, 123, 123);
const POINT_SELECTED_COLOR: sf::Color = sf::Color::rgb(167, 187, 236);

pub struct Polygon<'a> {
    points: Vec<sf::Vector2f>,
    points_circles: Vec<sf::CircleShape<'a>>,
    quads_vb: sf::VertexBuffer,
    lines_vb: sf::VertexBuffer,

    edges_color: sf::Color,
}

fn distance(point1: &sf::Vector2f, point2: &sf::Vector2f) -> f32 {
    let dx = point1.x - point2.x;
    let dy = point1.y - point2.y;
    (dx * dx + dy * dy).sqrt()
}

fn vec_len(vec: &sf::Vector2f) -> f32 {
    (vec.x * vec.x + vec.y * vec.y).sqrt()
}

fn vec_len2(vec: &sf::Vector2f) -> f32 {
    (vec.x * vec.x + vec.y * vec.y)
}

fn vec_norm(vec: &sf::Vector2f) -> sf::Vector2f {
    *vec / vec_len(vec)
}

fn dot_prod(vec1: &sf::Vector2f, vec2: &sf::Vector2f) -> f32 {
    vec1.x * vec2.x + vec1.y * vec2.y
}

impl<'a> Polygon<'a> {
    pub fn new() -> Polygon<'a> {
        Polygon {
            points: Vec::new(),
            points_circles: Vec::new(),
            quads_vb: sf::VertexBuffer::new(sf::PrimitiveType::QUADS, 0, sf::VertexBufferUsage::DYNAMIC),
            lines_vb: sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC),
            edges_color: LINES_COLOR,
        }
    }

    pub fn new_with_start_point(point: sf::Vector2f) -> Polygon<'a> {
        let mut result = Self::new();
        result.push_point(point);

        result
    }

    /// Creates polygon from the given points. This function doesn't assert that the returned
    /// Polygon is proper.
    pub fn create(mut points: Vec<sf::Vector2f>) -> Polygon<'a> {
        // Create points circles
        let points_circles = Self::generate_points_circles(&points);

        // Create lines vertex buffer
        let lines_vb = Self::generate_lines_vb(&points);

        // Create quads vertex buffer
        let quads_vb = Self::generate_quads_vb(&points);

        // Return the Polygon instance
        Polygon {
            points,
            points_circles,
            quads_vb,
            lines_vb,
            edges_color: LINES_COLOR,
        }
    }

    pub fn points_count(&self) -> usize {
        self.points.len()
    }

    fn generate_lines_vb(points: &Vec<sf::Vector2f>) -> sf::VertexBuffer {
        let vertices: Vec<sf::Vertex> = points
            .iter()
            .map(|&p| sf::Vertex::new(p, LINES_COLOR, sf::Vector2f::new(0., 0.)))
            .collect();

        let mut lines_vb = sf::VertexBuffer::new(
            sf::PrimitiveType::LINE_STRIP,
            points.len() as u32,
            sf::VertexBufferUsage::DYNAMIC,
        );
        lines_vb.update(&vertices, 0);

        lines_vb
    }

    fn generate_quads_vb(points: &Vec<sf::Vector2f>) -> sf::VertexBuffer {
        let mut vertices: Vec<sf::Vertex> = Vec::with_capacity(points.len() * 4);

        for i in 0..(points.len() - 1) {
            let p0p1 = points[i + 1] - points[i];
            let p0p1_len = (p0p1.x * p0p1.x + p0p1.y * p0p1.y).sqrt();
            let p0p1 = p0p1 / p0p1_len;

            let perp_cw = sf::Vector2f::new(p0p1.y, -p0p1.x);
            let perp_ccw = sf::Vector2f::new(-p0p1.y, p0p1.x);

            vertices.push(sf::Vertex::new(perp_cw * LINE_THICKNESS / 2. + points[i], LINES_COLOR, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_cw * LINE_THICKNESS / 2. + points[i + 1], LINES_COLOR, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_ccw * LINE_THICKNESS / 2. + points[i + 1], LINES_COLOR, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_ccw * LINE_THICKNESS / 2. + points[i], LINES_COLOR, sf::Vector2f::new(0., 0.)));
        }

        // Create the vertex buffer and fill it with the vertices
        let mut quads_vb = sf::VertexBuffer::new(
            sf::PrimitiveType::QUADS,
            (points.len() as u32) * 4,
            sf::VertexBufferUsage::DYNAMIC,
        );
        quads_vb.update(&vertices, 0);
        quads_vb
    }

    fn generate_points_circles(points: &Vec<sf::Vector2f>) -> Vec<sf::CircleShape<'static>> {
        let points_circles: Vec<sf::CircleShape> = points
            .iter()
            .map(|&p| {
                let mut c = sf::CircleShape::new(POINT_RADIUS, 20);
                c.set_position(p);
                c.set_origin(sf::Vector2f::new(c.radius(), c.radius()));
                c.set_fill_color(POINTS_COLOR);
                c
            })
            .collect();

        points_circles
    }

    pub fn push_point(&mut self, point: sf::Vector2f) {
        self.points.push(point);

        // Push a new point circle
        let mut new_circle = sf::CircleShape::new(POINT_RADIUS, 20);
        new_circle.set_position(point);
        new_circle.set_origin(sf::Vector2f::new(new_circle.radius(), new_circle.radius()));
        new_circle.set_fill_color(POINTS_COLOR);
        self.points_circles.push(new_circle);

        self.lines_vb = Self::generate_lines_vb(&self.points);

        self.quads_vb = Self::generate_quads_vb(&self.points);
    }

    fn update_vertex(&mut self, point: sf::Vector2f, color: sf::Color, index: usize) -> Result<(), io::Error> {
        if self.points_count() <= index {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        self.points[index] = point;
        self.points_circles[index].set_position(point);

        self.lines_vb.update(&[sf::Vertex::new(point, color, sf::Vector2f::new(0.0, 0.0))], index as u32);

        // TODO: quads_vb

        Ok(())
    }

    fn update_last_vertex(&mut self, point: sf::Vector2f, color: sf::Color) -> Result<(), io::Error> {
        self.update_vertex(point, color, self.points_count() - 1)
    }

    pub fn update_point(&mut self, point: sf::Vector2f, index: usize) -> Result<(), io::Error> {
        self.update_vertex(point, self.edges_color, index)
    }

    pub fn update_last_point(&mut self, point: sf::Vector2f) -> Result<(), io::Error> {
        self.update_point(point, self.points_count() - 1)
    }

    pub fn set_edges_color(&mut self, edges_color: sf::Color) {
        if edges_color == self.edges_color {
            return;
        }

        self.edges_color = edges_color;

        for i in 0..self.points.len() {
            self.lines_vb.update(&[sf::Vertex::new(self.points[i], self.edges_color, sf::Vector2f::new(0.0, 0.0))], i as u32);
        }

        // TODO: quads
    }

    /// Polygon is said to be proper iff the last element of points vector is an
    /// exact copy of the first element
    pub fn is_proper(&self) -> bool {
        if self.points.len() < 4 {
            return false;
        }

        // This comparison valid, since if the Polygon is proper, the last point must be
        // an exact copy of the first point
        if self.points[0] == self.points[self.points.len() - 1] {
            return true;
        }

        return false;
    }

    pub fn is_self_crossing(&self) -> bool {
        for i in 0..(self.points_count() - 2) {
            let line1 = geo::geometry::Line::new(
                geo::coord! {x: self.points[i].x, y: self.points[i].y},
                geo::coord! {x: self.points[i + 1].x, y: self.points[i + 1].y},
            );

            let mut end = self.points_count() - 1;
            if i == 0 {
                end = self.points_count() - 2;
            }

            // Do not check neighbor lines
            for j in (i + 2)..end {
                let line2 = geo::geometry::Line::new(
                    geo::coord! {x: self.points[j].x, y: self.points[j].y},
                    geo::coord! {x: self.points[j + 1].x, y: self.points[j + 1].y},
                );

                let result = geo::algorithm::line_intersection::line_intersection(
                    line1,
                    line2,
                );

                if result.is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// This method assumes, that first and the last element of the points are the same
    /// (polygon has to be a proper polygon). If points order has been reversed, returns true.
    fn assert_ccw(&mut self) -> bool {
        let mut sum: f32 = 0.;
        for i in 0..(self.points.len() - 1) {
            sum += (self.points[i + 1].x - self.points[i].x) * (self.points[i + 1].y + self.points[i].y);
        }

        if sum <= 0. {
            self.points.reverse();
            self.points_circles.reverse();
            self.lines_vb = Self::generate_lines_vb(&self.points);
            self.quads_vb = Self::generate_quads_vb(&self.points);
            return true;
        }

        false
    }

    pub fn first_point(&self) -> Option<sf::Vector2f> {
        if self.points_count() > 0 {
            return Some(self.points[0]);
        }
        None
    }

    pub fn clear(&mut self) {
        self.lines_vb = sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC);
        self.quads_vb = sf::VertexBuffer::new(sf::PrimitiveType::QUADS, 0, sf::VertexBufferUsage::DYNAMIC);
        self.points_circles.clear();
        self.points.clear();
    }

    pub fn draw_as_quads(&self, target: &mut dyn sf::RenderTarget) {
        self.quads_vb.draw(target, &Default::default());
    }

    pub fn draw_as_lines(&self, target: &mut dyn sf::RenderTarget) {
        self.lines_vb.draw(target, &Default::default());
    }

    pub fn draw_points_circles(&self, target: &mut dyn sf::RenderTarget) {
        for circle in &self.points_circles {
            circle.draw(target, &Default::default());
        }
    }

    pub fn draw_bresenham(&self, _img_target: &mut sf::Image) {
        // TODO
    }
}

impl<'a> Clone for Polygon<'a> {
    fn clone(&self) -> Self {
        Polygon {
            points: self.points.clone(),
            points_circles: self.points_circles.clone(),
            quads_vb: self.quads_vb.clone(),
            lines_vb: self.lines_vb.clone(),
            edges_color: self.edges_color.clone(),
        }
    }
}

pub struct PolygonBuilder<'s> {
    raw_polygon: Option<Polygon<'s>>,

    active: bool,

    helper_circle: sf::CircleShape<'s>,

    // PolygonBuilder events
    is_line_intersecting: bool,
    entered_correct_vertex_region: bool,
}

impl<'a> PolygonBuilder<'a> {
    pub fn new() -> PolygonBuilder<'a> {
        let mut helper_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 30);
        helper_circle.set_fill_color(POINT_DETECTION_COLOR_CORRECT);
        helper_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        PolygonBuilder {
            raw_polygon: None,
            active: false,
            is_line_intersecting: false,
            entered_correct_vertex_region: false,
            helper_circle,
        }
    }

    // If raw_polygon is None => creates a new one and adds starting point and the cursor point
    // Else just adds a new point
    fn add(&mut self, point: sf::Vector2f) {
        if self.raw_polygon.is_none() {
            // We need an additional point to attach it to the mouse cursor
            self.raw_polygon = Some(Polygon::new_with_start_point(point));
        }

        if let Some(ref mut polygon) = self.raw_polygon {
            polygon.push_point(point);
        }
    }

    fn clear_draw_flags(&mut self) {
        self.entered_correct_vertex_region = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn clear(&mut self) {
        let _poly = std::mem::replace(&mut self.raw_polygon, None);
        self.clear_draw_flags();
    }

    pub fn start(&mut self) {
        self.clear();
        self.active = true;
    }

    pub fn cancel(&mut self) {
        self.clear();
        self.active = false;
    }

    pub fn add_or_build(&mut self, add_pos: sf::Vector2f) -> Option<PolygonObject<'a>> {
        if !self.active || self.is_line_intersecting {
            return None;
        }

        if self.raw_polygon.is_some() {
            // Assert minimal length of the new edge
            if !self.entered_correct_vertex_region {
                for i in 1..(self.raw_polygon.as_ref().unwrap().points_count() - 1) {
                    if distance(&add_pos, &self.raw_polygon.as_ref().unwrap().points[i]) <= POLY_EDGE_MIN_LEN {
                        return None;
                    }
                }
            }

            // If a polygon already exists, there must be at least 2 vertices inside
            let first = self.raw_polygon.as_ref().unwrap().first_point().unwrap();

            if self.entered_correct_vertex_region {
                if self.raw_polygon.as_ref().unwrap().points_count() > 3 {
                    // If this condition is met, adding a new polygon is finished

                    // Change the position of the last vertex (cursor vertex)
                    self.raw_polygon.as_mut().unwrap().update_last_point(first).unwrap();

                    // Deactivate the builder
                    self.active = false;
                    self.clear_draw_flags();

                    // Build the PolygonObject
                    let poly = std::mem::replace(&mut self.raw_polygon, None);
                    return Some(PolygonObject::from(poly.unwrap().to_owned()));
                }

                // Prevent from putting all of the points in the same place
                return None;
            }
        }
        self.add(add_pos);

        None
    }

    pub fn update(&mut self, _dt: f32, mouse_pos: sf::Vector2f) {
        if !self.active {
            return;
        }

        if let Some(poly) = &mut self.raw_polygon {
            // Polygon should contain at least 2 vertices here
            let first = poly.first_point().unwrap();

            let mut m_pos = mouse_pos;

            let mut is_magnet_set: bool = false;

            if distance(&first, &m_pos) <= POINT_DETECTION_RADIUS {
                if poly.points_count() > 3 {
                    // Show the circle helper to complete the polygon creation
                    self.helper_circle.set_fill_color(POINT_DETECTION_COLOR_CORRECT);
                } else {
                    // Show the circle indicating that the completion is impossible
                    self.helper_circle.set_fill_color(POINT_DETECTION_COLOR_INCORRECT);
                }

                self.entered_correct_vertex_region = true;
                self.helper_circle.set_position(first);

                // Magnet
                is_magnet_set = true;
                m_pos = first;
            } else {
                self.entered_correct_vertex_region = false;
            }

            // Detect new line intersections
            self.is_line_intersecting = false;

            let line1 = geo::geometry::Line::new(
                geo::coord! {x: poly.points[poly.points_count() - 2].x, y: poly.points[poly.points_count() - 2].y},
                geo::coord! {x: poly.points[poly.points_count() - 1].x, y: poly.points[poly.points_count() - 1].y},
            );

            // Detect point intersections with the other lines
            if poly.points_count() > 3 && !is_magnet_set {
                for i in 0..(poly.points_count() - 3) {
                    let line2 = geo::geometry::Line::new(
                        geo::coord! {x: poly.points[i].x, y: poly.points[i].y},
                        geo::coord! {x: poly.points[i + 1].x, y: poly.points[i + 1].y},
                    );
                    let result = geo::algorithm::line_intersection::line_intersection(
                        line1,
                        line2,
                    );

                    if result.is_some() {
                        self.is_line_intersecting = true;
                        break;
                    }
                }
            }

            // Update cursor vertex position
            poly.update_last_point(m_pos).unwrap();
            if self.is_line_intersecting {
                poly.set_edges_color(LINES_COLOR_INCORRECT);
            } else {
                poly.set_edges_color(LINES_COLOR);
            }
        }
    }

    pub fn raw_polygon(&self) -> Option<&Polygon> {
        self.raw_polygon.as_ref()
    }

    pub fn draw(&self, target: &mut dyn sf::RenderTarget) {
        if self.entered_correct_vertex_region {
            target.draw(&self.helper_circle);
        }
    }
}

pub struct PolygonObject<'a> {
    raw_polygon: Polygon<'a>,

    // Selection
    selection: Vec<(bool, sf::CircleShape<'a>)>,
    selected_points_count: usize,

    // Point hover
    is_point_hovered: bool,
    hovered_point_id: usize,
    hover_circle: sf::CircleShape<'a>,

    // Line hover
    is_line_hovered: bool,
    // First point of the line is considered to be line_id
    hovered_line_id: usize,
    hover_quad: sf::ConvexShape<'a>,

    // Insert
    show_insert: bool,
    insert_circle: sf::CircleShape<'a>,
}

impl<'a> PolygonObject<'a> {
    pub fn from(raw: Polygon<'a>) -> PolygonObject<'a> {
        let mut selection: Vec<(bool, sf::CircleShape<'a>)> = vec![(false, sf::CircleShape::new(POINT_DETECTION_RADIUS, 20)); raw.points_count()];
        for circle in selection.iter_mut() {
            circle.1.set_radius(POINT_DETECTION_RADIUS);
            circle.1.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));
            circle.1.set_fill_color(POINT_SELECTED_COLOR);
        }

        let mut hover_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 20);
        hover_circle.set_fill_color(POINTS_COLOR);
        hover_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        let mut hover_quad = sf::ConvexShape::new(4);
        hover_quad.set_fill_color(POINTS_COLOR);

        let mut insert_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 20);
        insert_circle.set_fill_color(POINT_DETECTION_COLOR_CORRECT);
        insert_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        PolygonObject {
            raw_polygon: raw,
            selection,
            is_point_hovered: false,
            hovered_point_id: 0,
            hover_circle,
            selected_points_count: 0,
            insert_circle,
            show_insert: false,
            hover_quad,
            hovered_line_id: 0,
            is_line_hovered: false,
        }
    }
    pub fn raw_polygon(&self) -> &Polygon {
        &self.raw_polygon
    }

    fn update_on_point_hover(&mut self, pos: sf::Vector2f) {
        for (id, p) in self.raw_polygon.points.iter().enumerate() {
            if distance(&p, &pos) <= POINT_DETECTION_RADIUS {
                self.hover_circle.set_position(p.clone());
                self.hovered_point_id = id;
                self.is_point_hovered = true;
                return;
            }
        }
        self.is_point_hovered = false;
    }

    fn update_on_line_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..(self.raw_polygon.points_count() - 1) {
            let v01 = self.raw_polygon.points[i + 1] - self.raw_polygon.points[i];
            let v0m = pos - self.raw_polygon.points[i];

            if dot_prod(&v01, &v0m) < 0.0 {
                continue;
            }

            let proj1 = v01 * (dot_prod(&v01, &v0m) / vec_len2(&v01));

            if vec_len2(&proj1) > vec_len2(&v01) {
                continue;
            }

            let proj2 = v0m - proj1;

            let dist = vec_len(&proj2);

            if dist < LINE_DETECTION_DISTANCE {
                let proj_norm = vec_norm(&proj2);

                self.hover_quad.set_point(0, self.raw_polygon.points[i] + proj_norm * LINE_THICKNESS / 2.);
                self.hover_quad.set_point(1, self.raw_polygon.points[i + 1] + proj_norm * LINE_THICKNESS / 2.);
                self.hover_quad.set_point(2, self.raw_polygon.points[i + 1] - proj_norm * LINE_THICKNESS / 2.);
                self.hover_quad.set_point(3, self.raw_polygon.points[i] - proj_norm * LINE_THICKNESS / 2.);
                self.hovered_line_id = i;
                self.is_line_hovered = true;
                return;
            }
        }
        self.is_line_hovered = false;
    }

    pub fn update_hover(&mut self, mouse_pos: sf::Vector2f) {
        self.update_on_point_hover(mouse_pos);
        if self.is_point_hovered {
            self.is_line_hovered = false;
        } else {
            self.update_on_line_hover(mouse_pos);
        }
    }

    pub fn is_point_hovered(&self) -> bool {
        self.is_point_hovered
    }

    pub fn is_line_hovered(&self) -> bool {
        self.is_line_hovered
    }

    pub fn assert_ccw(&mut self) {
        if self.raw_polygon.assert_ccw() {
            self.selection.reverse();
        }
    }

    pub fn get_hovered_point_id(&self) -> usize {
        self.hovered_point_id
    }

    pub fn get_hovered_line_ids(&self) -> (usize, usize) {
        (self.hovered_line_id, self.hovered_line_id + 1)
    }

    pub fn select_point(&mut self, id: usize) -> Result<(), io::Error> {
        // selection.len() must always be equal to raw_polygon.points_count()
        if id >= self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        if self.selection[id].0 {
            return Ok(());
        }

        self.selected_points_count += 1;
        self.selection[id].0 = true;
        self.selection[id].1.set_position(self.raw_polygon.points[id]);

        if id == 0 {
            self.selected_points_count += 1;
            self.selection[self.raw_polygon.points_count() - 1].0 = true;
            self.selection[self.raw_polygon.points_count() - 1].1.set_position(self.raw_polygon.points[0]);
        } else if id == self.raw_polygon.points_count() - 1 {
            self.selected_points_count += 1;
            self.selection[0].0 = true;
            self.selection[0].1.set_position(self.raw_polygon.points[self.raw_polygon.points_count() - 1]);
        }

        Ok(())
    }

    pub fn is_point_selected(&self, id: usize) -> Result<bool, io::Error> {
        // selection.len() must always be equal to raw_polygon.points_count()
        if id >= self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        Ok(self.selection[id].0)
    }

    pub fn is_line_selected(&self, first_id: usize) -> Result<bool, io::Error> {
        if first_id >= self.raw_polygon.points_count() - 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        Ok(self.selection[first_id].0 && self.selection[first_id + 1].0)
    }

    pub fn deselect_all_points(&mut self) {
        for selection_circle in self.selection.iter_mut() {
            selection_circle.0 = false;
        }
        self.selected_points_count = 0;
    }

    pub fn selected_points_count(&self) -> usize {
        self.selected_points_count
    }

    pub fn deselect_point(&mut self, id: usize) -> Result<(), io::Error> {
        // selection.len() must always be equal to raw_polygon.points_count()
        if id >= self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        if !self.selection[id].0 {
            return Ok(());
        }

        self.selected_points_count -= 1;
        self.selection[id].0 = false;

        if id == 0 {
            self.selected_points_count -= 1;
            self.selection[self.raw_polygon.points_count() - 1].0 = false;
        } else if id == self.raw_polygon.points_count() - 1 {
            self.selected_points_count -= 1;
            self.selection[0].0 = false;
        }

        Ok(())
    }

    pub fn move_selected_points(&mut self, vec: sf::Vector2f) {
        // TODO: these 2 lines are ugly
        self.is_point_hovered = false;
        self.is_line_hovered = false;

        if self.selected_points_count == 0 {
            return;
        }

        // TODO: make it faster
        for i in 0..self.raw_polygon.points_count() {
            if self.selection[i].0 {
                let _err = self.raw_polygon.update_point(self.raw_polygon.points[i] + vec, i);
                self.selection[i].1.set_position(self.raw_polygon.points[i]);
            }
        }
    }

    pub fn draw(&self, target: &mut dyn RenderTarget) {
        if self.is_line_hovered {
            target.draw(&self.hover_quad);
        }

        if self.is_point_hovered {
            target.draw(&self.hover_circle);
        }

        for selection_circle in self.selection.iter() {
            if selection_circle.0 {
                target.draw(&selection_circle.1);
            }
        }
    }
}