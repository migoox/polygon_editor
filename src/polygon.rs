use sfml::graphics::{Drawable, RenderStates, Shape, Transformable};
use super::sf;
pub struct Polygon<'s> {
    points: Vec<sf::Vector2f>,
    color: sf::Color,
    points_circles: Vec<sf::CircleShape<'s>>,
    quads_vb: sf::VertexBuffer,
    lines_vb: sf::VertexBuffer,
}

impl Polygon<'_> {
    pub fn create(mut points: Vec<sf::Vector2f>, color: sf::Color) -> Polygon<'static> {
        if points.len() > 0 {
           points.push(points[0]);
        }

        // Create points circles
        let points_circles: Vec<sf::CircleShape> = points
            .iter()
            .map(|&p| {
                let mut c = sf::CircleShape::new(5.0, 20);
                c.set_position(p);
                c.set_origin(sf::Vector2f::new(c.radius(), c.radius()));
                c.set_fill_color(sf::Color::rgb(255 - color.r, 255 - color.g, 255 - color.b));
                c
            })
            .collect();

        // Create lines vertex buffer
        let vertices: Vec<sf::Vertex> = points
            .iter()
            .map(|&p| sf::Vertex::new(p, color, sf::Vector2f::new(0., 0.)))
            .collect();

        let mut lines_vb = sf::VertexBuffer::new(
                sf::PrimitiveType::LINE_STRIP,
                (points.len() as u32) * 2,
                sf::VertexBufferUsage::DYNAMIC,
        );
        lines_vb.update(&vertices, 0);

        // Create quads vertex buffer
        let thickness = 4.0;
        let mut vertices: Vec<sf::Vertex> = Vec::with_capacity(points.len()*4);

        for i in 0..(points.len() - 1) {
            let p0p1 = points[i + 1] - points[i];
            let p0p1_len = (p0p1.x*p0p1.x + p0p1.y*p0p1.y).sqrt();
            let p0p1 = p0p1 / p0p1_len;

            let perp_cw = sf::Vector2f::new(p0p1.y, -p0p1.x);
            let perp_ccw = sf::Vector2f::new(-p0p1.y, p0p1.x);

            vertices.push(sf::Vertex::new(perp_cw * thickness / 2. + points[i], color, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_cw * thickness / 2. + points[i + 1], color, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_ccw * thickness / 2. + points[i + 1], color, sf::Vector2f::new(0., 0.)));
            vertices.push(sf::Vertex::new(perp_ccw * thickness / 2. + points[i], color, sf::Vector2f::new(0., 0.)));
        }

        // Create the vertex buffer and fill it with the vertices
        let mut quads_vb = sf::VertexBuffer::new(
                sf::PrimitiveType::QUADS,
                (points.len() as u32) * 4,
                sf::VertexBufferUsage::DYNAMIC,
        );
        quads_vb.update(&vertices, 0);

        // Return the Polygon instance
        Polygon {
            points,
            color,
            points_circles,
            quads_vb,
            lines_vb,
        }
    }

    pub fn update_input(&mut self, ev: &sf::Event) {
        //

    }

    pub fn update(&mut self, _dt: f32) {
        //

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
