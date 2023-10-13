use std::io;
use line_intersection::LineInterval;
use sfml::graphics::{Drawable, Shape, Transformable};
use super::sf;

const LINE_THICKNESS: f32 = 6.0;
const POINT_RADIUS: f32 = 5.0;
const LINES_COLOR: sf::Color = sf::Color::WHITE;
const LINES_COLOR_INCORRECT: sf::Color = sf::Color::RED;

const POINTS_COLOR: sf::Color = sf::Color::BLUE;

const POINT_DETECTION_RADIUS: f32 = 10.0;
const POINT_DETECTION_COLOR_CORRECT: sf::Color = sf::Color::GREEN;
const POINT_DETECTION_COLOR_INCORRECT: sf::Color = sf::Color::RED;


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

    // Creates polygon from the given points, last point don't have to repeat the first one
    pub fn create(mut points: Vec<sf::Vector2f>) -> Polygon<'a> {
        if points.len() > 0 {
           points.push(points[0]);
        }

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
        let mut vertices: Vec<sf::Vertex> = Vec::with_capacity(points.len()*4);

        for i in 0..(points.len() - 1) {
            let p0p1 = points[i + 1] - points[i];
            let p0p1_len = (p0p1.x*p0p1.x + p0p1.y*p0p1.y).sqrt();
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
        if edges_color == self.edges_color  {
            return;
        }

        self.edges_color = edges_color;

        for i in 0..self.points.len() {
            self.lines_vb.update(&[sf::Vertex::new(self.points[i], self.edges_color, sf::Vector2f::new(0.0, 0.0))], i as u32);
        }

        // TODO: quads
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
    in_helper_circle: bool,
    is_line_intersecting: bool,
    left_btn_pressed: bool,
}

impl<'a> PolygonBuilder<'a> {
    pub fn new() -> PolygonBuilder<'a> {
        let mut helper_circle = sf::CircleShape::new(POINT_DETECTION_RADIUS, 30);
        helper_circle.set_fill_color(POINT_DETECTION_COLOR_CORRECT);
        helper_circle.set_origin(sf::Vector2f::new(POINT_DETECTION_RADIUS, POINT_DETECTION_RADIUS));

        PolygonBuilder {
            raw_polygon: None,
            active: false,
            left_btn_pressed: false,
            is_line_intersecting: false,
            in_helper_circle: false,
            helper_circle
        }
    }


    // If raw_polygon is None => creates a new one and adds starting point and the cursor point
    // Else just adds a new point
    pub fn add(&mut self, point: sf::Vector2f) {
        if self.raw_polygon.is_none() {
            // We need an additional point to attach it to the mouse cursor
            self.raw_polygon = Some(Polygon::new_with_start_point(point));
        }

        if let Some(ref mut polygon) = self.raw_polygon {
            polygon.push_point(point);
        }
    }

    fn clear_draw_flags(&mut self) {
        self.in_helper_circle = false;
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

    pub fn update_input_or_build(&mut self, ev: &sf::Event) -> Option<PolygonObject<'a>> {
        match ev {
            sf::Event::MouseButtonPressed { button: _, x, y } => {
                if !self.left_btn_pressed {
                    self.left_btn_pressed = true;

                    if !self.active || self.is_line_intersecting {
                        return None;
                    }

                    let add_pos = sf::Vector2f::new(*x as f32, *y as f32);

                    if let Some(poly) =  &mut self.raw_polygon {

                        // If a polygon already exists, there must be at least 2 vertices inside
                        let first = poly.first_point().unwrap();

                        if self.in_helper_circle  {
                            if poly.points_count() > 3 {
                                // If this condition is met, adding a new polygon is finished

                                // Change the position of the last vertex (cursor vertex)
                                poly.update_last_vertex(first, LINES_COLOR).unwrap();

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
                }
            },
            sf::Event::MouseButtonReleased { button: _, x: _, y: _ } => {
                self.left_btn_pressed = false;
            },
            _ => (),
        }

        None
    }
    pub fn update(&mut self, _dt: f32, window: &sf::RenderWindow) {
        if !self.active {
            return;
        }

        if let Some(poly) = &mut self.raw_polygon {
            // Polygon should contain at least 2 vertices here
            let first = poly.first_point().unwrap();

            let m_pos = window.mouse_position();
            let mut m_pos = sf::Vector2f::new(m_pos.x as f32, m_pos.y as f32);

            let mut is_magnet_set: bool = false;

            if distance(&first, &m_pos) <= POINT_DETECTION_RADIUS {
                if poly.points_count() > 3 {
                    // Show the circle helper to complete the polygon creation
                    self.helper_circle.set_fill_color(POINT_DETECTION_COLOR_CORRECT);
                } else {
                    // Show the circle indicating that the completion is impossible
                    self.helper_circle.set_fill_color(POINT_DETECTION_COLOR_INCORRECT);
                }

                self.in_helper_circle = true;
                self.helper_circle.set_position(first);

                // Magnet
                is_magnet_set = true;
                m_pos = first;
            } else {
                self.in_helper_circle = false;
            }

            // Detect new line intersections
            self.is_line_intersecting = false;

            let line1 = geo::geometry::Line::new(
                geo::coord!{x: poly.points[poly.points_count() - 2].x, y: poly.points[poly.points_count() - 2].y},
                geo::coord!{x: poly.points[poly.points_count() - 1].x, y: poly.points[poly.points_count() - 1].y},
            );

            // Detect point intersections with the other lines
            if poly.points_count() > 3 && !is_magnet_set {
                for i in 0..(poly.points_count() - 3) {
                    let line2 = geo::geometry::Line::new(
                        geo::coord!{x: poly.points[i].x, y: poly.points[i].y},
                        geo::coord!{x: poly.points[i + 1].x, y: poly.points[i + 1].y},
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
        if self.in_helper_circle {
            target.draw(&self.helper_circle);
        }
    }
}

pub struct PolygonObject<'a> {
    raw_polygon: Polygon<'a>,
}

impl<'a> PolygonObject<'a> {
    pub fn from(raw: Polygon<'a>) -> PolygonObject<'a> {
        PolygonObject {
            raw_polygon: raw.clone()
        }
    }
    pub fn raw_polygon(&self) -> &Polygon {
        &self.raw_polygon
    }
}