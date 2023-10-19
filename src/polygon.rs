use std::io;
use std::collections::{HashMap, HashSet};
use egui_sfml::egui;
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

#[derive(Clone)]
#[derive(PartialEq)]
pub enum EdgeConstraint {
    None,
    Horizontal,
    Vertical,
}

pub struct Point<'a> {
    pos: sf::Vector2f,
    idle_circle: sf::CircleShape<'a>,
    selection_circle: sf::CircleShape<'a>,
    is_selected: bool,

    // Defines edge constraint of en edge created by this point and the next point in a proper
    // polygon points vector.
    // None by default, None means that a polygon that point is the part of is not proper or
    // that there is no constraint on that edge.
    edge_constraint: EdgeConstraint,
}

impl<'a> Point<'a> {
    pub fn new(pos: sf::Vector2f) -> Point<'a> {
        let mut idle_circle = sf::CircleShape::new(POINT_RADIUS, 20);
        idle_circle.set_position(pos);
        idle_circle.set_origin(sf::Vector2f::new(idle_circle.radius(), idle_circle.radius()));
        idle_circle.set_fill_color(POINTS_COLOR);

        let mut selection_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 20);
        selection_circle.set_position(pos);
        selection_circle.set_origin(sf::Vector2f::new(selection_circle.radius(), selection_circle.radius()));
        selection_circle.set_fill_color(POINT_SELECTED_COLOR);

        Point {
            idle_circle,
            pos,
            selection_circle,
            is_selected: false,
            edge_constraint: EdgeConstraint::None,
        }
    }

    pub fn update_pos(&mut self, pos: sf::Vector2f) {
        self.pos = pos;
        self.selection_circle.set_position(pos);
        self.idle_circle.set_position(pos);
    }

    pub fn draw_selection_circle(&self, target: &mut dyn sf::RenderTarget) {
        target.draw(&self.selection_circle);
    }
    pub fn draw_idle_circle(&self, target: &mut dyn sf::RenderTarget) {
        target.draw(&self.idle_circle);
    }
}

impl<'a> Clone for Point<'a> {
    fn clone(&self) -> Self {
        Point {
            pos: self.pos.clone(),
            idle_circle: self.idle_circle.clone(),
            selection_circle: self.selection_circle.clone(),
            is_selected: self.is_selected.clone(),
            edge_constraint: self.edge_constraint.clone(),
        }
    }
}

pub struct Polygon<'a> {
    points: Vec<Point<'a>>,
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
        let points = points.iter().map(|p| Point::new(p.clone())).collect();

        // Create lines vertex buffer
        let lines_vb = Self::generate_lines_vb(&points);

        // Create quads vertex buffer
        let quads_vb = Self::generate_quads_vb(&points);

        // Return the Polygon instance
        Polygon {
            points,
            quads_vb,
            lines_vb,
            edges_color: LINES_COLOR,
        }
    }

    pub fn points_count(&self) -> usize {
        self.points.len()
    }

    fn generate_lines_vb(points: &Vec<Point>) -> sf::VertexBuffer {
        let vertices: Vec<sf::Vertex> = points
            .iter()
            .map(|p| sf::Vertex::new(p.pos.clone(), LINES_COLOR, sf::Vector2f::new(0., 0.)))
            .collect();

        let mut lines_vb = sf::VertexBuffer::new(
            sf::PrimitiveType::LINE_STRIP,
            points.len() as u32,
            sf::VertexBufferUsage::DYNAMIC,
        );
        lines_vb.update(&vertices, 0);

        lines_vb
    }

    fn generate_quads_vb(points: &Vec<Point>) -> sf::VertexBuffer {
        let mut vertices: Vec<sf::Vertex> = Vec::with_capacity(points.len() * 4);

        for i in 0..(points.len() - 1) {
            let p0p1 = points[i + 1].pos - points[i].pos;
            let p0p1_len = (p0p1.x * p0p1.x + p0p1.y * p0p1.y).sqrt();
            let p0p1 = p0p1 / p0p1_len;

            let perp_cw = sf::Vector2f::new(p0p1.y, -p0p1.x);
            let perp_ccw = sf::Vector2f::new(-p0p1.y, p0p1.x);

            vertices.push(sf::Vertex::new(perp_cw * LINE_THICKNESS / 2. + points[i].pos, LINES_COLOR, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_cw * LINE_THICKNESS / 2. + points[i + 1].pos, LINES_COLOR, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_ccw * LINE_THICKNESS / 2. + points[i + 1].pos, LINES_COLOR, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_ccw * LINE_THICKNESS / 2. + points[i].pos, LINES_COLOR, sf::Vector2f::new(0., 0.)));
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

    pub fn push_point(&mut self, point_pos: sf::Vector2f) {
        self.points.push(Point::new(point_pos.clone()));
        self.lines_vb = Self::generate_lines_vb(&self.points);
        self.quads_vb = Self::generate_quads_vb(&self.points);
    }

    /// Inserts at "pos" index
    pub fn insert_point(&mut self, pos: usize, point_pos: sf::Vector2f) {
        self.points.insert(pos, Point::new(point_pos.clone()));
        self.lines_vb = Self::generate_lines_vb(&self.points);
        self.quads_vb = Self::generate_quads_vb(&self.points);
    }


    pub fn remove_point(&mut self, id: usize) {
        self.points.remove(id);
        self.lines_vb = Self::generate_lines_vb(&self.points);
        self.quads_vb = Self::generate_quads_vb(&self.points);
    }

    fn update_vertex(&mut self, point_pos: sf::Vector2f, color: sf::Color, index: usize) -> Result<(), io::Error> {
        if self.points_count() <= index {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        self.points[index].update_pos(point_pos);

        self.lines_vb.update(&[sf::Vertex::new(point_pos, color, sf::Vector2f::new(0.0, 0.0))], index as u32);

        // TODO: quads_vb

        Ok(())
    }

    fn update_last_vertex(&mut self, point_pos: sf::Vector2f, color: sf::Color) -> Result<(), io::Error> {
        self.update_vertex(point_pos, color, self.points_count() - 1)
    }

    pub fn update_point(&mut self, point_pos: sf::Vector2f, index: usize) -> Result<(), io::Error> {
        self.update_vertex(point_pos, self.edges_color, index)
    }

    pub fn update_last_point(&mut self, point_pos: sf::Vector2f) -> Result<(), io::Error> {
        self.update_point(point_pos, self.points_count() - 1)
    }

    pub fn set_edges_color(&mut self, edges_color: sf::Color) {
        if edges_color == self.edges_color {
            return;
        }

        self.edges_color = edges_color;

        for i in 0..self.points.len() {
            self.lines_vb.update(&[sf::Vertex::new(self.points[i].pos, self.edges_color, sf::Vector2f::new(0.0, 0.0))], i as u32);
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
        if self.points[0].pos == self.points[self.points.len() - 1].pos {
            return true;
        }

        return false;
    }

    pub fn is_self_crossing(&self) -> bool {
        for i in 0..(self.points_count() - 2) {
            let line1 = geo::geometry::Line::new(
                geo::coord! {x: self.points[i].pos.x, y: self.points[i].pos.y},
                geo::coord! {x: self.points[i + 1].pos.x, y: self.points[i + 1].pos.y},
            );

            let mut end = self.points_count() - 1;
            if i == 0 {
                end = self.points_count() - 2;
            }

            // Do not check neighbor lines
            for j in (i + 2)..end {
                let line2 = geo::geometry::Line::new(
                    geo::coord! {x: self.points[j].pos.x, y: self.points[j].pos.y},
                    geo::coord! {x: self.points[j + 1].pos.x, y: self.points[j + 1].pos.y},
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
            sum += (self.points[i + 1].pos.x - self.points[i].pos.x) * (self.points[i + 1].pos.y + self.points[i].pos.y);
        }

        if sum <= 0. {
            self.points.reverse();
            self.lines_vb = Self::generate_lines_vb(&self.points);
            self.quads_vb = Self::generate_quads_vb(&self.points);
            return true;
        }

        false
    }

    pub fn first_point_pos(&self) -> Option<sf::Vector2f> {
        if self.points_count() > 0 {
            return Some(self.points[0].pos);
        }
        None
    }

    pub fn clear(&mut self) {
        self.lines_vb = sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC);
        self.quads_vb = sf::VertexBuffer::new(sf::PrimitiveType::QUADS, 0, sf::VertexBufferUsage::DYNAMIC);
        self.points.clear();
    }

    pub fn draw_as_quads(&self, target: &mut dyn sf::RenderTarget) {
        self.quads_vb.draw(target, &Default::default());
    }

    pub fn draw_as_lines(&self, target: &mut dyn sf::RenderTarget) {
        self.lines_vb.draw(target, &Default::default());
    }

    pub fn draw_idle_circles(&self, target: &mut dyn sf::RenderTarget) {
        for point in &self.points {
            point.draw_idle_circle(target);
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
                    if distance(&add_pos, &self.raw_polygon.as_ref().unwrap().points[i].pos) <= POLY_EDGE_MIN_LEN {
                        return None;
                    }
                }
            }

            // If a polygon already exists, there must be at least 2 vertices inside
            let first = self.raw_polygon.as_ref().unwrap().first_point_pos().unwrap();

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
            let first = poly.first_point_pos().unwrap();

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
                geo::coord! {x: poly.points[poly.points_count() - 2].pos.x, y: poly.points[poly.points_count() - 2].pos.y},
                geo::coord! {x: poly.points[poly.points_count() - 1].pos.x, y: poly.points[poly.points_count() - 1].pos.y},
            );

            // Detect point intersections with the other lines
            if poly.points_count() > 3 && !is_magnet_set {
                for i in 0..(poly.points_count() - 3) {
                    let line2 = geo::geometry::Line::new(
                        geo::coord! {x: poly.points[i].pos.x, y: poly.points[i].pos.y},
                        geo::coord! {x: poly.points[i + 1].pos.x, y: poly.points[i + 1].pos.y},
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

static mut CURR_POLYGONOBJ_ID: usize = 0;

pub struct PolygonObject<'a> {
    raw_polygon: Polygon<'a>,

    id: usize,

    // Selection
    selection: HashSet<usize>,

    show_hover: bool,

    // Point hover
    is_point_hovered: bool,
    hovered_point_id: usize,
    hover_circle: sf::CircleShape<'a>,

    // Line hover
    is_line_hovered: bool,
    // First point of the line is considered to be line_id
    hovered_line_id: usize,
    hover_quad: sf::ConvexShape<'a>,

    // Insert/remove
    can_insert: bool,
    insert_circle: sf::CircleShape<'a>,
    insert_pos: sf::Vector2f,
}

impl<'a> PolygonObject<'a> {
    pub fn from(raw: Polygon<'a>) -> PolygonObject<'a> {
        let mut hover_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 20);
        hover_circle.set_fill_color(POINTS_COLOR);
        hover_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        let mut hover_quad = sf::ConvexShape::new(4);
        hover_quad.set_fill_color(POINTS_COLOR);

        let mut insert_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 20);
        insert_circle.set_fill_color(POINT_DETECTION_COLOR_CORRECT);
        insert_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        let mut remove_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 20);
        remove_circle.set_fill_color(POINT_DETECTION_COLOR_INCORRECT);
        remove_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        let mut id = 0;
        unsafe {
            id = CURR_POLYGONOBJ_ID;
            CURR_POLYGONOBJ_ID += 1;
        }

        PolygonObject {
            raw_polygon: raw,
            selection: HashSet::new(),
            show_hover: false,
            is_point_hovered: false,
            hovered_point_id: 0,
            id,
            hover_circle,
            insert_circle,
            can_insert: false,
            hover_quad,
            hovered_line_id: 0,
            is_line_hovered: false,
            insert_pos: sf::Vector2f::new(0.0, 0.0),
        }
    }
    pub fn raw_polygon(&self) -> &Polygon {
        &self.raw_polygon
    }

    pub fn can_insert(&self) -> bool {
        self.can_insert
    }
    pub fn get_insert_pos(&self) -> sf::Vector2f {
        self.insert_pos
    }
    pub fn insert_point(&mut self, id: usize, pos: sf::Vector2f) -> Result<(), io::Error> {
        if id > self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        self.raw_polygon.insert_point(id, pos);
        self.can_insert = false;
        Ok(())
    }

    pub fn remove_point(&mut self, id: usize) -> Result<(), io::Error> {
        if id > self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        if self.raw_polygon.points_count() <= 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, ""));
        }

        if id == 0 || id == self.raw_polygon.points_count() - 1 {
            self.raw_polygon.remove_point(0);
            self.raw_polygon.remove_point(self.raw_polygon.points_count() - 1);
            self.raw_polygon.push_point(self.raw_polygon.points[0].pos);

            self.selection.remove(&0);
            self.selection.remove(&(self.raw_polygon.points_count() - 1));
        } else {
            self.raw_polygon.remove_point(id);

            self.selection.remove(&id);
        }

        Ok(())
    }

    pub fn update_insertion(&mut self, pos: sf::Vector2f) {
        for i in 0..(self.raw_polygon.points_count() - 1) {
            if distance(&pos, &self.raw_polygon.points[i].pos) <= POINT_DETECTION_RADIUS ||
                distance(&pos, &self.raw_polygon.points[i + 1].pos) <= POINT_DETECTION_RADIUS {
                continue;
            }

            let v01 = self.raw_polygon.points[i + 1].pos - self.raw_polygon.points[i].pos;
            let v0m = pos - self.raw_polygon.points[i].pos;

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
                self.insert_pos = self.raw_polygon.points[i].pos + proj1;
                self.insert_circle.set_position(self.insert_pos);
                self.can_insert = true;
                return;
            }
        }
        self.can_insert = false;
    }

    fn update_on_point_hover(&mut self, pos: sf::Vector2f) {
        for (id, p) in self.raw_polygon.points.iter().enumerate() {
            if distance(&p.pos, &pos) <= POINT_DETECTION_RADIUS {
                self.hover_circle.set_position(p.pos.clone());
                self.hovered_point_id = id;
                self.is_point_hovered = true;
                return;
            }
        }
        self.is_point_hovered = false;
    }

    fn update_on_line_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..(self.raw_polygon.points_count() - 1) {
            let v01 = self.raw_polygon.points[i + 1].pos - self.raw_polygon.points[i].pos;
            let v0m = pos - self.raw_polygon.points[i].pos;

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

                self.hover_quad.set_point(0, self.raw_polygon.points[i].pos + proj_norm * LINE_THICKNESS / 2.);
                self.hover_quad.set_point(1, self.raw_polygon.points[i + 1].pos + proj_norm * LINE_THICKNESS / 2.);
                self.hover_quad.set_point(2, self.raw_polygon.points[i + 1].pos - proj_norm * LINE_THICKNESS / 2.);
                self.hover_quad.set_point(3, self.raw_polygon.points[i].pos - proj_norm * LINE_THICKNESS / 2.);
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

    pub fn is_hover_show_disabled(&self) -> bool {
        self.show_hover
    }
    pub fn disable_hover_show(&mut self) {
        self.show_hover = true;
    }
    pub fn enable_hover_show(&mut self) {
        self.show_hover = false;
    }
    pub fn is_point_hovered(&self) -> bool {
        self.is_point_hovered
    }

    pub fn is_line_hovered(&self) -> bool {
        self.is_line_hovered
    }

    pub fn assert_ccw(&mut self) {
        self.raw_polygon.assert_ccw();
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

        let last_id = self.raw_polygon.points_count() - 1;
        if id == 0 || id == last_id {
            self.raw_polygon.points[0].is_selected = true;
            self.raw_polygon.points[last_id].is_selected = true;

            self.selection.insert(0);
            self.selection.insert(last_id);
        } else {
            self.raw_polygon.points[id].is_selected = true;

            self.selection.insert(id);
        }

        Ok(())
    }

    pub fn is_point_selected(&self, id: usize) -> Result<bool, io::Error> {
        // selection.len() must always be equal to raw_polygon.points_count()
        if id >= self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        Ok(self.raw_polygon.points[id].is_selected)
    }

    pub fn is_line_selected(&self, first_id: usize) -> Result<bool, io::Error> {
        if first_id >= self.raw_polygon.points_count() - 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        Ok(self.raw_polygon.points[first_id].is_selected && self.raw_polygon.points[first_id + 1].is_selected)
    }

    pub fn deselect_all_points(&mut self) {
        for id in self.selection.iter() {
            self.raw_polygon.points[*id].is_selected = false;
        }
        self.selection.clear();
    }

    pub fn select_all_points(&mut self) {
        for id in 0..self.raw_polygon.points_count() {
            self.raw_polygon.points[id].is_selected = true;
            self.selection.insert(id);
        }
    }

    pub fn selected_points_count(&self) -> usize {
        self.selection.len()
    }

    pub fn deselect_point(&mut self, id: usize) -> Result<(), io::Error> {
        // selection.len() must always be equal to raw_polygon.points_count()
        if id >= self.raw_polygon.points_count() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        let last_id = self.raw_polygon.points_count() - 1;
        if id == 0 || id == last_id {
            self.raw_polygon.points[0].is_selected = false;
            self.raw_polygon.points[last_id].is_selected = false;

            self.selection.remove(&0);
            self.selection.remove(&(self.raw_polygon.points_count() - 1));
        } else {
            self.raw_polygon.points[id].is_selected = false;

            self.selection.remove(&id);
        }

        Ok(())
    }

    pub fn move_selected_points(&mut self, vec: sf::Vector2f) {
        for id in self.selection.iter() {
            let _err = self.raw_polygon.update_point(self.raw_polygon.points[*id].pos + vec, *id);
        }
    }

    pub fn draw(&self, target: &mut dyn RenderTarget) {
        if !self.show_hover {
            if self.is_line_hovered {
                target.draw(&self.hover_quad);
            }

            if self.is_point_hovered {
                target.draw(&self.hover_circle);
            }
        }

        if self.can_insert {
            target.draw(&self.insert_circle);
        }

        for id in self.selection.iter() {
            self.raw_polygon.points[*id].draw_selection_circle(target);
        }
    }

    pub fn draw_egui(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(format!("Polygon {}", self.id))
            .default_open(true)
            .show(ui, |ui| {
                for id in self.selection.iter() {
                    if *id == self.raw_polygon.points_count() - 1 {
                        continue;
                    }

                    if self.raw_polygon.points[*id + 1].is_selected {
                        // Pick the drawing method

                        egui::ComboBox::from_label(format!("({}, {}) Constraint", *id, *id + 1))
                            .selected_text(match self.raw_polygon.points[*id].edge_constraint {
                                EdgeConstraint::None => "None",
                                EdgeConstraint::Horizontal => "Horizontal",
                                EdgeConstraint::Vertical => "Vertical"
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.raw_polygon.points[*id].edge_constraint, EdgeConstraint::None, "None");
                                ui.selectable_value(&mut self.raw_polygon.points[*id].edge_constraint, EdgeConstraint::Horizontal, "Horizontal");
                                ui.selectable_value(&mut self.raw_polygon.points[*id].edge_constraint, EdgeConstraint::Vertical, "Vertical");
                            });
                    }
                }
            });
    }
}