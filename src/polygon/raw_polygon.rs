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
    point_circle: sf::CircleShape<'a>,
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
            point_circle: idle_circle,
            pos,
            selection_circle,
            is_selected: false,
            edge_constraint: EdgeConstraint::None,
        }
    }

    pub fn update_pos(&mut self, pos: sf::Vector2f) {
        self.pos = pos;
        self.selection_circle.set_position(pos);
        self.point_circle.set_position(pos);
    }

    pub fn draw_selection_circle(&self, target: &mut dyn sf::RenderTarget) {
        target.draw(&self.selection_circle);
    }
    pub fn draw_point_circle(&self, target: &mut dyn sf::RenderTarget) {
        target.draw(&self.point_circle);
    }
}

impl<'a> Clone for Point<'a> {
    fn clone(&self) -> Self {
        Point {
            pos: self.pos.clone(),
            point_circle: self.point_circle.clone(),
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
    show_last_line: bool,
}


impl<'a> Polygon<'a> {
    pub fn new() -> Polygon<'a> {
        Polygon {
            points: Vec::new(),
            lines_vb: sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC),
            edges_color: style::LINES_COLOR,
            show_last_line: true,
        }
    }

    pub fn new_with_start_point(point: sf::Vector2f) -> Polygon<'a> {
        let mut result = Self::new();
        result.push_point_with_pos(point);

        result
    }

    /// Creates polygon from the given points.
    pub fn create(mut points: Vec<sf::Vector2f>) -> Polygon<'a> {
        // Create points
        let points: Vec<Point> = points
            .iter()
            .map(|p| Point::new(p.clone()))
            .collect();

        // Return the Polygon instance
        let mut result = Polygon::new();
        result.points = points;
        result.generate_lines_vb();

        result
    }

    fn generate_lines_vb(&mut self) {
        if self.points_count() == 0 {
            return;
        }

        let mut vertices: Vec<sf::Vertex> = self.points
            .iter()
            .map(|p| sf::Vertex::new(
                p.pos.clone(),
                style::LINES_COLOR,
                sf::Vector2f::new(0., 0.),
            ))
            .collect();

        let mut len = self.points_count();
        if self.show_last_line {
            vertices.push(sf::Vertex::new(self.points[0].pos, self.edges_color, sf::Vector2f::new(0.0, 0.0)));
            len += 1;
        }

        self.lines_vb = sf::VertexBuffer::new(
            sf::PrimitiveType::LINE_STRIP,
            len as u32,
            sf::VertexBufferUsage::DYNAMIC,
        );
        self.lines_vb.update(&vertices, 0);
    }

    pub fn show_last_line(&mut self, flag: bool) {
        if self.show_last_line == flag {
            return;
        }
        self.show_last_line = flag;
        self.generate_lines_vb();
    }

    pub fn points_count(&self) -> usize {
        self.points.len()
    }

    /// Makes id cyclic.
    pub fn fix_index(&self, id: isize) -> usize {
        return (id.rem_euclid(self.points_count() as isize)) as usize;
    }

    /// Returns point's position, id is cyclic.
    pub fn get_point_pos(&self, id: isize) -> sf::Vector2f {
        self.points[self.fix_index(id)].pos
    }


    pub fn push_point_with_pos(&mut self, point_pos: sf::Vector2f) {
        self.points.push(Point::new(point_pos));
        self.generate_lines_vb();
    }

    /// Inserts at "id" index. "id" is cyclic.
    pub fn insert_point_with_pos(&mut self, id: isize, point_pos: sf::Vector2f) {
        self.points.insert(self.fix_index(id), Point::new(point_pos));
        self.generate_lines_vb();
    }

    /// Removes a point with the given id
    pub fn remove_point(&mut self, id: isize) {
        self.points.remove(self.fix_index(id));
        self.generate_lines_vb();
    }


    fn update_vertex(&mut self, point_pos: sf::Vector2f, color: sf::Color, index: isize) {
        let index = self.fix_index(index);

        if self.show_last_line && index == 0 {
            // Update points
            self.points[0].update_pos(point_pos);

            // Update first in vbo
            self.lines_vb.update(&[sf::Vertex::new(
                point_pos,
                color,
                sf::Vector2f::new(0.0, 0.0))], 0);

            // Update last in vbo
            self.lines_vb.update(&[sf::Vertex::new(
                point_pos,
                color,
                sf::Vector2f::new(0.0, 0.0))], self.points.len() as u32);
        } else {
            self.points[index].update_pos(point_pos);

            // Update in vbo
            self.lines_vb.update(&[sf::Vertex::new(
                point_pos,
                color,
                sf::Vector2f::new(0.0, 0.0))], index as u32);
        }
    }

    fn update_last_vertex(&mut self, point_pos: sf::Vector2f, color: sf::Color) {
        self.update_vertex(point_pos, color, self.points_count() as isize - 1)
    }

    pub fn update_point_pos(&mut self, point_pos: sf::Vector2f, index: isize) {
        self.update_vertex(point_pos, self.edges_color, index)
    }

    pub fn update_last_point_pos(&mut self, point_pos: sf::Vector2f) {
        self.update_point_pos(point_pos, self.points_count() as isize - 1)
    }

    pub fn set_edges_color(&mut self, edges_color: sf::Color) {
        if edges_color == self.edges_color {
            return;
        }

        self.edges_color = edges_color;

        for i in 0..self.points.len() {
            self.lines_vb.update(&[sf::Vertex::new(self.points[i].pos, self.edges_color, sf::Vector2f::new(0.0, 0.0))], i as u32);
        }

        if self.show_last_line {
            self.lines_vb.update(&[sf::Vertex::new(self.points[0].pos, self.edges_color, sf::Vector2f::new(0.0, 0.0))], self.points_count() as u32);
        }
    }

    pub fn is_proper(&self) -> bool {
        if self.points.len() < 3 {
            return false;
        }
        return true;
    }

    pub fn select_point(&mut self, id: isize) {
        let id = self.fix_index(id);
        self.points[id].is_selected = true;
    }
    pub fn deselect_point(&mut self, id: isize) {
        let id = self.fix_index(id);
        self.points[id].is_selected = false;
    }

    pub fn is_point_selected(&self, id: isize) -> bool {
        self.points[self.fix_index(id)].is_selected
    }

    pub fn is_self_crossing(&self) -> bool {
        // TODO: Fix
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

    pub fn assert_ccw(&mut self) -> bool {
        assert_eq!(self.is_proper(), true);

        let mut sum: f32 = 0.;
        for i in 0..(self.points.len() - 1) {
            sum += (self.points[i + 1].pos.x - self.points[i].pos.x) * (self.points[i + 1].pos.y + self.points[i].pos.y);
        }

        if sum <= 0. {
            self.points.reverse();
            self.generate_lines_vb();
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

    pub fn draw_edges(&self, target: &mut dyn sf::RenderTarget) {
        self.lines_vb.draw(target, &Default::default());
    }

    pub fn draw_points(&self, target: &mut dyn sf::RenderTarget) {
        for point in &self.points {
            point.draw_point_circle(target);
        }
    }
    pub fn draw_point_selection(&self, id: isize, target: &mut dyn sf::RenderTarget) {
        self.points[self.fix_index(id)].draw_selection_circle(target);
    }

    pub fn draw_edges_bresenham(&self, _img_target: &mut sf::Image) {
        // TODO
    }
}

impl<'a> Clone for Polygon<'a> {
    fn clone(&self) -> Self {
        Polygon {
            points: self.points.clone(),
            lines_vb: self.lines_vb.clone(),
            edges_color: self.edges_color.clone(),
            show_last_line: self.show_last_line.clone(),
        }
    }
}