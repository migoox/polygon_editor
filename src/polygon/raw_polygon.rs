use std::collections::HashMap;
use std::rc::Rc;
use geo::LineIntersection;
use sfml::graphics::{Drawable, RcFont, RcText, RcTexture, RenderTarget, Shape, Transformable};
use crate::my_math::{cross2, distance, vec_norm};
use super::sf;
use super::style;
use super::my_math;

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

    direction: sf::Vector2f,
    normal: sf::Vector2f,
    prev_normal: sf::Vector2f,
    offset_vec: sf::Vector2f,
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
            direction: sf::Vector2f::new(0., 0.),
            normal: sf::Vector2f::new(0., 0.),
            prev_normal: sf::Vector2f::new(0., 0.),
            offset_vec: sf::Vector2f::new(0., 0.),
        }
    }

    pub fn get_dir(&self) -> sf::Vector2f {
        return self.direction;
    }
    pub fn update_pos(&mut self, pos: sf::Vector2f) {
        self.pos = pos;
        self.selection_circle.set_position(pos);
        self.point_circle.set_position(pos);
    }

    pub fn update_normals(&mut self, prev: sf::Vector2f, next: sf::Vector2f) {
        let v01 = self.pos - prev;
        let v12 = next - self.pos;

        let v01_perp = sf::Vector2f::new(-v01.y, v01.x);
        let v12_perp = sf::Vector2f::new(-v12.y, v12.x);

        let v01_perp = my_math::vec_norm(&v01_perp);
        let v12_perp = my_math::vec_norm(&v12_perp);

        self.normal = v12_perp;
        self.prev_normal = v01_perp;
        self.offset_vec = my_math::vec_norm(&(v01_perp + v12_perp)) /
            ((1. + v01_perp.dot(v12_perp)) / 2.).sqrt();

        if cross2(&v01, &v12) < 0. {
            self.direction = my_math::vec_norm(&(v01_perp + v12_perp));
        } else {
            self.direction = -my_math::vec_norm(&(v01_perp + v12_perp));
        }
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
            direction: self.direction.clone(),
            normal: self.normal.clone(),
            prev_normal: self.prev_normal.clone(),
            offset_vec: self.offset_vec.clone(),
        }
    }
}

pub struct Polygon<'a> {
    points: Vec<Point<'a>>,
    lines_vb: sf::VertexBuffer,
    edges_color: sf::Color,
    show_last_line: bool,

    edge_constraint_sprites: Vec<sf::RcSprite>,
    points_labels: Vec<sf::RcText>,

    nametag: Option<sf::RcText>,

    name: String,
    // Resources references
    constraint_texture: Option<Rc<sf::RcTexture>>,
    font: Option<Rc<sf::RcFont>>,
}


impl<'a> Polygon<'a> {
    pub fn new() -> Polygon<'a> {
        Polygon {
            points: Vec::new(),
            lines_vb: sf::VertexBuffer::new(sf::PrimitiveType::LINE_STRIP, 0, sf::VertexBufferUsage::DYNAMIC),
            edges_color: style::LINES_COLOR,
            show_last_line: true,
            edge_constraint_sprites: Vec::new(),
            points_labels: Vec::new(),
            constraint_texture: None,
            font: None,
            nametag: None,
            name: "Polygon".to_string(),
        }
    }

    pub fn find_center(&self) -> sf::Vector2f {
        let mut result = sf::Vector2f::new(0., 0.);
        for point in self.points.iter() {
            result += point.pos;
        }
        return result / (self.points_count() as f32);
    }
    fn update_nametag(&mut self) {
        if self.font.is_some() {
            self.nametag = Some(sf::RcText::new(&self.name, self.font.as_ref().unwrap(), 20));
            let p = self.find_center();
            self.nametag.as_mut().unwrap().set_position(p);
            let center = self.nametag.as_ref().unwrap().global_bounds().size() / 2.;
            self.nametag.as_mut().unwrap().set_origin(center);
        }
    }
    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.update_nametag();
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn set_label_resources(&mut self, constraint_texture: &Rc<sf::RcTexture>, font: &Rc<sf::RcFont>) {
        self.constraint_texture = Some(Rc::clone(constraint_texture));
        self.font = Some(Rc::clone(font));
        self.update_nametag();
        self.update_normals();
        self.update_labels();
    }

    fn update_normals(&mut self) {
        for i in 0..self.points_count() {
            let prev = self.get_point_pos(i as isize - 1);
            let next = self.get_point_pos(i as isize + 1);
            self.points[i].update_normals(prev, next);
        }
    }
    fn update_labels(&mut self) {
        if self.constraint_texture.is_some() {
            self.edge_constraint_sprites.resize(self.points_count(), sf::RcSprite::with_texture(self.constraint_texture.as_ref().unwrap()));

            for id in 0..self.edge_constraint_sprites.len() {
                self.edge_constraint_sprites[id].set_origin(style::CONSTRAINT_SPRITE_SIZE / 2.);
                let pos = (self.get_point_pos(id as isize) + self.get_point_pos(id as isize + 1)) / 2.;
                self.edge_constraint_sprites[id].set_position(pos);
            }
        }

        if self.font.is_some() {
            if self.points_labels.len() < self.points_count() {
                for i in 0..(self.points_count() - self.points_labels.len()) {
                    self.points_labels.push(sf::RcText::new((&format!("{}", self.points_labels.len() + i)), self.font.as_ref().unwrap(), 20));
                    let id = self.points_labels.len() - 1;
                    let center = self.points_labels[id].global_bounds().size() / 2.;
                    self.points_labels[id].set_origin(center);
                }
            }
            self.points_labels.resize(self.points_count(), sf::RcText::new("0", self.font.as_ref().unwrap(), 20));

            for id in 0..self.points_count() {
                let pos = self.get_point_pos(id as isize);
                let vec = self.points[id].direction * 26.0;
                self.points_labels[id].set_position(pos + vec);
            }
            let p = self.find_center();
            self.nametag.as_mut().unwrap().set_position(p);
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
        result.update_labels();
        result.update_normals();
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
    pub fn get_offset_vec(&self, id: isize) -> sf::Vector2f { self.points[self.fix_index(id)].offset_vec }

    pub fn get_edge_constraint(&self, id: isize) -> EdgeConstraint {
        self.points[self.fix_index(id)].edge_constraint.clone()
    }
    pub fn set_edge_contsraint(&mut self, id: isize, constraint: EdgeConstraint) {
        let id = self.fix_index(id);
        self.points[id].edge_constraint = constraint;
    }
    pub fn push_point_with_pos(&mut self, point_pos: sf::Vector2f) {
        self.points.push(Point::new(point_pos));
        self.generate_lines_vb();
        self.update_normals();
        self.update_labels();
    }

    /// Inserts at "id" index. "id" is cyclic.
    pub fn insert_point_with_pos(&mut self, id: isize, point_pos: sf::Vector2f) {
        self.points.insert(self.fix_index(id), Point::new(point_pos));
        self.generate_lines_vb();
        self.update_normals();
        self.update_labels();
    }

    /// Removes a point with the given id
    pub fn remove_point(&mut self, id: isize) {
        self.points.remove(self.fix_index(id));
        self.generate_lines_vb();
        self.update_normals();
        self.update_labels();
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
        self.update_normals();
        self.update_labels();
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

    pub fn get_self_crossing_edges(&self) -> HashMap<usize, (usize, sf::Vector2f)> {
        let mut hash_map: HashMap<usize, Vec<(usize, sf::Vector2f)>> = HashMap::new();

        for i in 0..self.points_count() as isize {
            let line1 = geo::geometry::Line::new(
                geo::coord! {x: self.get_point_pos(i).x, y: self.get_point_pos(i).y},
                geo::coord! {x: self.get_point_pos(i + 1).x, y: self.get_point_pos(i + 1).y},
            );

            let mut end = self.points_count() as isize;
            if i == 0 {
                end -= 1;
            }
            // Do not check neighbor lines
            for j in (i + 2)..end {
                let line2 = geo::geometry::Line::new(
                    geo::coord! {x: self.get_point_pos(j).x, y: self.get_point_pos(j).y},
                    geo::coord! {x: self.get_point_pos(j + 1).x, y: self.get_point_pos(j + 1).y},
                );

                let result = geo::algorithm::line_intersection::line_intersection(
                    line1,
                    line2,
                );

                if result.is_some() {
                    match result.as_ref().unwrap() {
                        LineIntersection::SinglePoint { intersection, is_proper } => {
                            if *is_proper {
                                let id0 = self.fix_index(i);
                                let id1 = self.fix_index(j);
                                let point = sf::Vector2f::new(intersection.x, intersection.y);

                                let val = hash_map.entry(id0).or_insert(Vec::new());
                                val.push((id1, point));

                                let val = hash_map.entry(id1).or_insert(Vec::new());
                                val.push((id0, point));
                            }
                        }
                        LineIntersection::Collinear { intersection: _intersection } => ()
                    }
                }
            }
        }

        let mut result: HashMap<usize, (usize, sf::Vector2f)> = HashMap::new();
        for (id, vec) in hash_map {
            let start_point = self.get_point_pos(id as isize);

            let mut min_dist = my_math::distance2(&start_point, &vec[0].1);
            let mut min_i = 0;
            for i in 1..vec.len() {
                let curr_dist = my_math::distance2(&start_point, &vec[i].1);
                if curr_dist < min_dist {
                    min_dist = curr_dist;
                    min_i = i;
                }
            }

            result.insert(id, (vec[min_i].0, vec[min_i].1));
        }

        result
    }
    pub fn is_self_crossing(&self) -> bool {
        for i in 0..self.points_count() as isize {
            let line1 = geo::geometry::Line::new(
                geo::coord! {x: self.get_point_pos(i).x, y: self.get_point_pos(i).y},
                geo::coord! {x: self.get_point_pos(i + 1).x, y: self.get_point_pos(i + 1).y},
            );

            let mut end = self.points_count() as isize;
            if i == 0 {
                end -= 1;
            }
            // Do not check neighbor lines
            for j in (i + 2)..end {
                let line2 = geo::geometry::Line::new(
                    geo::coord! {x: self.get_point_pos(j).x, y: self.get_point_pos(j).y},
                    geo::coord! {x: self.get_point_pos(j + 1).x, y: self.get_point_pos(j + 1).y},
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
        for i in 0..self.points_count() as isize {
            sum += (self.get_point_pos(i + 1).x - self.get_point_pos(i).x)
                * (self.get_point_pos(i + 1).y + self.get_point_pos(i).y);
        }

        if sum <= 0. {
            self.points.reverse();
            // Remap constraints
            let constraints_cpy: Vec<EdgeConstraint> =
                self.points.iter().map(|p| p.edge_constraint.clone()).collect();
            for i in 0..self.points_count() as isize {
                self.set_edge_contsraint(i, EdgeConstraint::None);
                let next = self.fix_index(i + 1);
                self.set_edge_contsraint(i, constraints_cpy[next].clone());
            }


            self.generate_lines_vb();
            self.update_normals();
            self.update_labels();
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

    pub fn draw_labels(&self, target: &mut dyn sf::RenderTarget) {
        for (id, sprite) in self.edge_constraint_sprites.iter().enumerate() {
            if self.points[id].edge_constraint != EdgeConstraint::None {
                target.draw(sprite);
            }
        }

        for point in self.points_labels.iter() {
            target.draw(point);
        }

        if self.nametag.is_some() {
            target.draw(self.nametag.as_ref().unwrap());
        }
    }

    // d, incr_d_e, incr_d_ne, win_e, win_ne, steep
    fn get_bresenham_parameters(&self, slope: f32, delta: sf::Vector2i) -> (i32, i32, i32, sf::Vector2i, sf::Vector2i, bool) {
        if slope >= 0. && slope <= 1. {
            return (2 * delta.y - delta.x, 2 * delta.y, 2 * delta.y - 2 * delta.x, sf::Vector2i::new(1, 0), sf::Vector2i::new(1, 1), false);
        } else if slope >= -1. && slope < 0. {
            return (2 * delta.y + delta.x, 2 * delta.y + 2 * delta.x, 2 * delta.y, sf::Vector2i::new(1, -1), sf::Vector2i::new(1, 0), false);
        } else if slope > 1. {
            return (delta.y - 2 * delta.x, 2 * delta.y - 2 * delta.x, -2 * delta.x, sf::Vector2i::new(1, 1), sf::Vector2i::new(0, 1), true);
        }
        (delta.y + 2 * delta.x, 2 * delta.x, 2 * delta.x + 2 * delta.y, sf::Vector2i::new(0, -1), sf::Vector2i::new(1, -1), true)
    }

    fn bresenham_line(&self, mut p0: sf::Vector2f, mut p1: sf::Vector2f, img_target: &mut sf::Image) {
        if p1.x < p0.x {
            std::mem::swap(&mut p0, &mut p1);
        }

        let p0 = sf::Vector2i::new(p0.x as i32, p0.y as i32);
        let p1 = sf::Vector2i::new(p1.x as i32, p1.y as i32);
        let delta = p1 - p0;

        // Slope
        let m = ((p1.y - p0.y) as f32) / ((p1.x - p0.x) as f32);

        let (mut d, incr_d_e, incr_d_ne, e_win, ne_win, steep) =
            self.get_bresenham_parameters(m, delta);

        let mut p = p0;

        if steep {
            while (p1.y - p.y).abs() > 0 {
                if p.x < img_target.size().x as i32 && p.x >= 0 &&
                    p.y < img_target.size().y as i32 && p.y >= 0 {
                    unsafe { img_target.set_pixel(p.x as u32, p.y as u32, self.edges_color) }
                }

                if d < 0 {
                    d += incr_d_e;
                    p += e_win;
                } else {
                    d += incr_d_ne;
                    p += ne_win;
                }
            }
        } else {
            while p.x < p1.x {
                if p.x < img_target.size().x as i32 && p.x >= 0 &&
                    p.y < img_target.size().y as i32 && p.y >= 0 {
                    unsafe { img_target.set_pixel(p.x as u32, p.y as u32, self.edges_color) }
                }

                if d < 0 {
                    d += incr_d_e;
                    p += e_win;
                } else {
                    d += incr_d_ne;
                    p += ne_win;
                }
            }
        }
    }

    pub fn draw_edges_bresenham(&self, img_target: &mut sf::Image) {
        let mut end = self.points_count();
        if !self.show_last_line {
            end -= 1;
        }
        for i in 0..end as isize {
            self.bresenham_line(self.get_point_pos(i), self.get_point_pos(i + 1), img_target);
        }
    }
}

impl<'a> Clone for Polygon<'a> {
    fn clone(&self) -> Self {
        let mut new_txt: Option<Rc<RcTexture>> = None;
        let mut new_font: Option<Rc<RcFont>> = None;

        if self.constraint_texture.is_some() {
            new_txt = Some(Rc::clone(&self.constraint_texture.as_ref().unwrap()));
        }

        if self.font.is_some() {
            new_font = Some(Rc::clone(&self.font.as_ref().unwrap()));
        }

        Polygon {
            points: self.points.clone(),
            lines_vb: self.lines_vb.clone(),
            edges_color: self.edges_color.clone(),
            show_last_line: self.show_last_line.clone(),
            edge_constraint_sprites: self.edge_constraint_sprites.clone(),
            points_labels: self.points_labels.clone(),
            constraint_texture: new_txt,
            font: new_font,
            nametag: self.nametag.clone(),
            name: self.name.clone(),
        }
    }
}