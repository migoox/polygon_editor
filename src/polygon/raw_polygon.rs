use std::io;
use sfml::graphics::{Drawable, RenderTarget, Shape, Transformable};
use super::sf;
use super::style;

#[derive(Clone)]
#[derive(PartialEq)]
pub enum EdgeConstraint {
    None,
    Horizontal,
    Vertical,
}

struct Point<'a> {
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
        let mut idle_circle = sf::CircleShape::new(style::POINT_RADIUS, 20);
        idle_circle.set_position(pos);
        idle_circle.set_origin(sf::Vector2f::new(idle_circle.radius(), idle_circle.radius()));
        idle_circle.set_fill_color(style::POINTS_COLOR);

        let mut selection_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 20);
        selection_circle.set_position(pos);
        selection_circle.set_origin(sf::Vector2f::new(selection_circle.radius(), selection_circle.radius()));
        selection_circle.set_fill_color(style::POINT_SELECTED_COLOR);

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
    lines_vb: sf::VertexBuffer,
    edges_color: sf::Color,
}


impl<'a> Polygon<'a> {
    pub fn new() -> Polygon<'a> {
        Polygon {
            points: Vec::new(),
            lines_vb: sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC),
            edges_color: style::LINES_COLOR,
        }
    }

    pub fn new_with_start_point(point: sf::Vector2f) -> Polygon<'a> {
        let mut result = Self::new();
        result.push_point(point);

        result
    }
    pub fn get_point_pos(&self, id: usize) -> sf::Vector2f {
        self.points[id].pos
    }

    /// Creates polygon from the given points. This function doesn't assert that the returned
    /// Polygon is proper.
    pub fn create(mut points: Vec<sf::Vector2f>) -> Polygon<'a> {
        // Create points circles
        let points = points.iter().map(|p| Point::new(p.clone())).collect();

        // Create lines vertex buffer
        let lines_vb = Self::generate_lines_vb(&points);

        // Return the Polygon instance
        Polygon {
            points,
            lines_vb,
            edges_color: style::LINES_COLOR,
        }
    }

    pub fn points_count(&self) -> usize {
        if self.is_proper() {
            return self.points.len() - 1;
        }
        self.points.len()
    }

    fn generate_lines_vb(points: &Vec<Point>) -> sf::VertexBuffer {
        let vertices: Vec<sf::Vertex> = points
            .iter()
            .map(|p| sf::Vertex::new(p.pos.clone(), style::LINES_COLOR, sf::Vector2f::new(0., 0.)))
            .collect();

        let mut lines_vb = sf::VertexBuffer::new(
            sf::PrimitiveType::LINE_STRIP,
            points.len() as u32,
            sf::VertexBufferUsage::DYNAMIC,
        );
        lines_vb.update(&vertices, 0);

        lines_vb
    }

    pub fn push_point(&mut self, point_pos: sf::Vector2f) {
        self.points.push(Point::new(point_pos.clone()));
        self.lines_vb = Self::generate_lines_vb(&self.points);
    }

    /// Inserts at "pos" index
    pub fn insert_point(&mut self, pos: usize, point_pos: sf::Vector2f) {
        self.points.insert(pos, Point::new(point_pos.clone()));
        self.lines_vb = Self::generate_lines_vb(&self.points);
    }

    // Takes the last point in the points vector and moves it onto first element
    pub fn move_last_to_make_proper(&mut self) {
        if self.is_proper() {
            return;
        }
        self.points[self.points.len() - 1] = self.points[0].clone();
    }

    // Clones the first element of the points vector and pushes it at the end
    pub fn make_proper(&mut self) {
        if self.is_proper() {
            return;
        }
        self.points.push(self.points[0].clone())
    }

    pub fn remove_point(&mut self, id: usize) {
        self.points.remove(id);

        if self.is_proper() {
            if id == 0 {
                self.points.remove(self.points_count() - 1);
                self.points.push(self.points[0].clone());
            } else if id == self.points_count() - 1 {
                self.points.remove(0);
                self.points.push(self.points[0].clone());
            }
        }

        self.lines_vb = Self::generate_lines_vb(&self.points);
    }

    fn update_vertex(&mut self, point_pos: sf::Vector2f, color: sf::Color, index: usize) -> Result<(), io::Error> {
        if self.points_count() <= index {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Index out of range"));
        }

        self.points[index].update_pos(point_pos);

        self.lines_vb.update(&[sf::Vertex::new(point_pos, color, sf::Vector2f::new(0.0, 0.0))], index as u32);

        if self.is_proper() {
            if index == 0 {
                self.points[self.points.len() - 1].update_pos(point_pos);

                self.lines_vb.update(&[sf::Vertex::new(point_pos, color, sf::Vector2f::new(0.0, 0.0))], self.points.len() as u32 - 1);
            } else if index == self.points.len() - 1 {
                self.points[0].update_pos(point_pos);

                self.lines_vb.update(&[sf::Vertex::new(point_pos, color, sf::Vector2f::new(0.0, 0.0))], 0);
            }
        }


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

    pub fn select_point(&mut self, id: usize) {
        if self.is_proper() {
            assert_ne!(id, self.points.len() - 1);
        }
        self.points[id].is_selected = true;
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
    pub fn assert_ccw(&mut self) -> bool {
        let mut sum: f32 = 0.;
        for i in 0..(self.points.len() - 1) {
            sum += (self.points[i + 1].pos.x - self.points[i].pos.x) * (self.points[i + 1].pos.y + self.points[i].pos.y);
        }

        if sum <= 0. {
            self.points.reverse();
            self.lines_vb = Self::generate_lines_vb(&self.points);
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
        self.points.clear();
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
            lines_vb: self.lines_vb.clone(),
            edges_color: self.edges_color.clone(),
        }
    }
}