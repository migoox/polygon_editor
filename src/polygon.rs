use sfml::graphics::{Drawable, RenderStates, Shape, Transformable, VertexBufferUsage};
use super::sf;

const LINE_THICKNESS: f32 = 6.0;
const POINT_RADIUS: f32 = 5.0;
const LINES_COLOR: sf::Color = sf::Color::GREEN;
const POINTS_COLOR: sf::Color = sf::Color::RED;

pub struct Polygon<'s> {
    points: Vec<sf::Vector2f>,
    points_circles: Vec<sf::CircleShape<'s>>,
    quads_vb: sf::VertexBuffer,
    lines_vb: sf::VertexBuffer,

    // Gadgets
}

impl Polygon<'_> {
    pub fn new() -> Polygon<'static> {
        Polygon {
            points: Vec::new(),
            points_circles: Vec::new(),
            quads_vb: sf::VertexBuffer::new(sf::PrimitiveType::QUADS, 0, sf::VertexBufferUsage::DYNAMIC),
            lines_vb: sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC),
        }
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

    pub fn create(mut points: Vec<sf::Vector2f>) -> Polygon<'static> {
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
        }
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

    pub fn draw_bresenham(&self, img_target: &mut sf::Image) {
        // TODO
    }
}

pub struct PolygonBuilder<'s> {
    raw_polygon: Option<Polygon<'s>>,
    left_btn_pressed: bool,
}

impl PolygonBuilder<'_> {
    pub fn new() -> PolygonBuilder<'static> {
        PolygonBuilder {
            raw_polygon: None,
            left_btn_pressed: false,
        }
    }


    // If raw_polygon is None => creates a new one and adds point
    // Else just adds point
    pub fn add(&mut self, point: sf::Vector2f) {
        if self.raw_polygon.is_none() {
            self.raw_polygon = Some(Polygon::new());
        }

        if let Some(ref mut polygon) = self.raw_polygon {
            polygon.push_point(point);
        }
    }

    pub fn clear(&mut self) {
        if let Some(poly) = &mut self.raw_polygon {
            poly.clear();
        }
    }

    pub fn update_input(&mut self, ev: &sf::Event) {
        match ev {
            sf::Event::MouseButtonPressed { button, x, y } => {
                if !self.left_btn_pressed {
                    self.left_btn_pressed = true;

                    self.add(sf::Vector2::new(*x as f32, *y as f32));
                }
            },
            sf::Event::MouseButtonReleased { button, x, y } => {
                self.left_btn_pressed = false;
            },
            _ => (),
        }

    }

    pub fn update(&mut self, _dt: f32) {

    }

    pub fn raw_polygon(&self) -> Option<&Polygon> {
        self.raw_polygon.as_ref()
    }

    pub fn build(&mut self) -> PolygonObject {

        PolygonObject {

        }
    }
}

pub struct PolygonObject {

}
