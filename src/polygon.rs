use std::io;
use std::collections::HashSet;
use egui_sfml::egui;
use sfml::graphics::{CircleShape, Drawable, RcFont, RcTexture, RenderTarget, Shape, Transformable};
use std::collections::HashMap;
use std::rc::Rc;
use geo::LineIntersection;
use crate::my_math::{circle_vs_plane_frac, is_right_turn};
use crate::style;
use crate::my_math;
use crate::sf;
use crate::my_math::cross2;
use serde::{Serialize, Deserialize};
use sfml::window::Key::P;
use crate::line_alg::LinePainter;

#[derive(Serialize, Deserialize, Debug)]
pub struct RawCoord {
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawPolygonCoords {
    pub coords: Vec<RawCoord>,
}

impl RawPolygonCoords {
    pub fn new(coords: Vec<RawCoord>) -> RawPolygonCoords {
        RawPolygonCoords {
            coords,
        }
    }

    pub fn from_sf_points(points: Vec<sf::Vector2f>) -> RawPolygonCoords {
        let coords = points.iter().map(|p| RawCoord { x: p.x, y: p.y }).collect();
        RawPolygonCoords {
            coords,
        }
    }

    fn from_points(points: Vec<Point>) -> RawPolygonCoords {
        let coords = points.iter().map(|p| RawCoord { x: p.pos.x, y: p.pos.y }).collect();
        RawPolygonCoords {
            coords,
        }
    }
}

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

    pub fn draw_selection_circle(&self, target: &mut dyn RenderTarget) {
        target.draw(&self.selection_circle);
    }
    pub fn draw_point_circle(&self, target: &mut dyn RenderTarget) {
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
    constraint_texture: Option<Rc<RcTexture>>,
    font: Option<Rc<RcFont>>,
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

    pub fn set_points_from_raw(&mut self, raw_polygon: RawPolygonCoords) {
        self.points = raw_polygon.coords.iter().map(|coord| Point::new(sf::Vector2f::new(coord.x, coord.y))).collect();
        self.generate_lines_vb();
        self.update_normals();
        self.update_labels();
    }

    pub fn get_raw(&self) -> RawPolygonCoords {
        RawPolygonCoords {
            coords: self.points.iter().map(|p| RawCoord { x: p.pos.x, y: p.pos.y }).collect()
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
                self.edges_color,
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
        self.generate_lines_vb();
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

    pub fn get_self_crossing_edges(&self) -> HashMap<usize, Vec<(usize, sf::Vector2f)>> {
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
        hash_map
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

    pub fn draw_point_selection(&self, id: isize, target: &mut dyn RenderTarget) {
        self.points[self.fix_index(id)].draw_selection_circle(target);
    }

    pub fn draw_labels(&self, target: &mut dyn RenderTarget) {
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


    pub fn draw_edges_bresenham(&self, img_target: &mut sf::Image, line_painter: &LinePainter) {
        let mut end = self.points_count();
        if !self.show_last_line {
            end -= 1;
        }
        for i in 0..end as isize {
            line_painter.draw_line(self.get_point_pos(i), self.get_point_pos(i + 1), img_target);
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

pub struct PolygonObjectFactory<'s> {
    polygon: Option<Polygon<'s>>,

    curr_id: usize,
    helper_circle: sf::CircleShape<'s>,

    new_line: sf::VertexBuffer,
    new_point_circle: sf::CircleShape<'s>,

    // PolygonBuilder events
    is_line_intersecting: bool,
    entered_correct_vertex_region: bool,

    // Resources
    constraint_texture: Rc<sf::RcTexture>,
    font: Rc<sf::RcFont>,
}

impl<'a> PolygonObjectFactory<'a> {
    pub fn get_resources(&self) -> (&Rc<sf::RcTexture>, &Rc<sf::RcFont>) {
        (&self.constraint_texture, &self.font)
    }

    pub fn new() -> PolygonObjectFactory<'a> {
        let mut helper_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 30);
        helper_circle.set_fill_color(style::POINT_DETECTION_COLOR_CORRECT);
        helper_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));

        let mut new_point_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 30);
        new_point_circle.set_fill_color(style::POINTS_COLOR);
        new_point_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));
        new_point_circle.set_position(sf::Vector2f::new(-100.0, -100.0));

        PolygonObjectFactory {
            polygon: None,
            is_line_intersecting: false,
            curr_id: 0,
            entered_correct_vertex_region: false,
            helper_circle,
            new_line: sf::VertexBuffer::new(sf::PrimitiveType::LINES, 2, sf::VertexBufferUsage::DYNAMIC),
            new_point_circle,
            font: Rc::new(sf::RcFont::from_file("res/lato.ttf").expect("Couldn't load the font")),
            constraint_texture: Rc::new(sf::RcTexture::from_file("res/link2.png").expect("Couldn't load the texture")),
        }
    }

    fn update_line(&mut self, pos1: sf::Vector2f, pos2: sf::Vector2f) {
        self.new_line.update(
            &[
                sf::Vertex::new(
                    pos1,
                    style::POINTS_COLOR,
                    sf::Vector2f::new(0.0, 0.0),
                ),
                sf::Vertex::new(
                    pos2,
                    style::POINTS_COLOR,
                    sf::Vector2f::new(0.0, 0.0),
                )
            ],
            0,
        );
    }

    // If raw_polygon is None => creates a new one and adds starting point
    // Else just adds a new point
    fn add(&mut self, point: sf::Vector2f) {
        if self.polygon.is_none() {
            self.polygon = Some(Polygon::new_with_start_point(point));
            self.polygon.as_mut().unwrap().set_label_resources(&self.constraint_texture, &self.font);
            self.polygon.as_mut().unwrap().show_last_line(false);
            self.polygon.as_mut().unwrap().set_name(format!("Polygon #{}", self.curr_id));
            self.update_line(point, point);
            self.new_point_circle.set_position(point);

            self.curr_id += 1;
            return;
        }

        if let Some(ref mut polygon) = self.polygon {
            polygon.push_point_with_pos(point);
        }
    }

    fn clear_draw_flags(&mut self) {
        self.entered_correct_vertex_region = false;
        self.is_line_intersecting = false;
    }

    pub fn clear(&mut self) {
        let _poly = std::mem::replace(&mut self.polygon, None);
        self.update_line(sf::Vector2f::new(0., 0.), sf::Vector2f::new(0., 0.));
        self.clear_draw_flags();
    }

    pub fn add_or_build(&mut self, add_pos: sf::Vector2f) -> Option<PolygonObject<'a>> {
        if self.is_line_intersecting {
            return None;
        }

        if self.polygon.is_some() {
            // Assert minimal length of the new edge
            if !self.entered_correct_vertex_region {
                for i in 1..self.polygon.as_ref().unwrap().points_count() {
                    if my_math::distance(&add_pos, &self.polygon.as_ref().unwrap().get_point_pos(i as isize)) <= style::POLY_EDGE_MIN_LEN {
                        return None;
                    }
                }
            } else {
                if self.polygon.as_ref().unwrap().points_count() >= 3 {
                    // If this condition is met, adding a new polygon is finished

                    self.update_line(sf::Vector2f::new(0.0, 0.0), sf::Vector2::new(0.0, 0.0));
                    self.new_point_circle.set_position(sf::Vector2f::new(-100.0, -100.0));

                    // Deactivate the builder
                    // self.active = false;
                    self.clear_draw_flags();

                    // Build the PolygonObject
                    self.polygon.as_mut().unwrap().assert_ccw();
                    self.polygon.as_mut().unwrap().show_last_line(true);
                    let poly = std::mem::replace(&mut self.polygon, None);
                    return Some(PolygonObject::from(poly.unwrap().to_owned()));
                }

                // Prevent from putting all of the points in the same place
                return None;
            }
        }
        self.add(add_pos);

        None
    }

    pub fn build_from_raw(&mut self, raw_polygon: RawPolygonCoords) -> PolygonObject<'a> {
        let mut poly = Polygon::new();
        poly.set_points_from_raw(raw_polygon);
        poly.set_name(format!("Polygon #{}", self.curr_id));
        poly.set_label_resources(&self.constraint_texture, &self.font);
        poly.show_last_line(true);

        self.curr_id += 1;

        PolygonObject::from(poly)
    }

    pub fn update(&mut self, _dt: f32, mouse_pos: sf::Vector2f) {
        if let Some(poly) = &mut self.polygon {
            // Polygon should contain at least 2 vertices here
            let first = poly.first_point_pos().unwrap();

            let last = poly.points_count() - 1;
            let last = poly.get_point_pos(last as isize);

            let mut m_pos = mouse_pos;

            let mut is_magnet_set: bool = false;

            if my_math::distance(&first, &m_pos) <= style::POINT_DETECTION_RADIUS {
                if poly.points_count() >= 3 {
                    // Show the circle helper to complete the polygon creation
                    self.helper_circle.set_fill_color(style::POINT_DETECTION_COLOR_CORRECT);
                } else {
                    // Show the circle indicating that the completion is impossible
                    self.helper_circle.set_fill_color(style::POINT_DETECTION_COLOR_INCORRECT);
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
                geo::coord! {x: last.x, y: last.y},
                geo::coord! {x: m_pos.x, y: m_pos.y},
            );

            // Detect point intersections with the other lines
            if poly.points_count() >= 3 && !is_magnet_set {
                for i in 0..(poly.points_count() - 2) as isize {
                    let line2 = geo::geometry::Line::new(
                        geo::coord! {x: poly.get_point_pos(i).x, y: poly.get_point_pos(i).y},
                        geo::coord! {x: poly.get_point_pos(i + 1).x, y: poly.get_point_pos(i + 1).y},
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

            if self.is_line_intersecting {
                poly.set_edges_color(style::LINES_COLOR_INCORRECT);
            } else {
                poly.set_edges_color(style::LINES_COLOR);
            }

            // Update line helper
            self.update_line(last, m_pos);
            self.new_point_circle.set_position(m_pos);
        } else {
            self.new_point_circle.set_position(mouse_pos);
        }
    }

    pub fn polygon(&self) -> Option<&Polygon> {
        self.polygon.as_ref()
    }

    pub fn draw_ctx(&self, target: &mut dyn RenderTarget) {
        if let Some(poly) = self.polygon.as_ref() {
            poly.draw_points(target);
        }

        self.new_line.draw(target, &Default::default());
        target.draw(&self.new_point_circle);

        if self.entered_correct_vertex_region {
            target.draw(&self.helper_circle);
        }

        target.draw(&self.new_line);
    }

    pub fn draw_edges(&self, target: &mut dyn RenderTarget) {
        if let Some(poly) = self.polygon.as_ref() {
            poly.draw_edges(target);
        }
    }

    pub fn draw_bresenham_edges(&self, _target: &mut dyn RenderTarget, img_target: &mut sf::Image, line_painter: &LinePainter) {
        if let Some(poly) = self.polygon.as_ref() {
            poly.draw_edges_bresenham(img_target, line_painter);
        }
    }
}

pub struct PolygonObject<'a> {
    polygon: Polygon<'a>,

    // Selection
    selection: HashSet<usize>,

    show_hover: bool,

    // Draw Offset 
    show_offset: bool,
    naive_offset: bool,
    offset_size: f32,
    offset_polygon: Polygon<'a>,

    // Point hover
    hover_circle: CircleShape<'a>,
    is_point_hovered: bool,
    hovered_point_id: usize,

    // Line hover
    is_line_hovered: bool,
    // First point of the line is considered to be line_id
    hovered_line_id: usize,
    hover_quad: sf::ConvexShape<'a>,

    // Insert/remove
    can_insert: bool,
    insert_circle: CircleShape<'a>,
    insert_pos: sf::Vector2f,
}

impl<'a> PolygonObject<'a> {
    pub fn from(raw: Polygon<'a>) -> PolygonObject<'a> {
        let mut hover_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 20);
        hover_circle.set_fill_color(style::POINTS_COLOR);
        hover_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));

        let mut hover_quad = sf::ConvexShape::new(4);
        hover_quad.set_fill_color(style::POINTS_COLOR);

        let mut insert_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 20);
        insert_circle.set_fill_color(style::POINT_DETECTION_COLOR_CORRECT);
        insert_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));

        let mut remove_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 20);
        remove_circle.set_fill_color(style::POINT_DETECTION_COLOR_INCORRECT);
        remove_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));

        PolygonObject {
            polygon: raw,
            selection: HashSet::new(),
            show_hover: false,
            is_point_hovered: false,
            hovered_point_id: 0,
            hover_circle,
            insert_circle,
            can_insert: false,
            hover_quad,
            hovered_line_id: 0,
            is_line_hovered: false,
            insert_pos: sf::Vector2f::new(0.0, 0.0),
            show_offset: false,
            naive_offset: false,
            offset_size: 50.0,
            offset_polygon: Polygon::new(),
        }
    }

    pub fn get_raw(&self) -> RawPolygonCoords {
        self.polygon.get_raw()
    }
    pub fn polygon(&self) -> &Polygon {
        &self.polygon
    }

    pub fn can_insert(&self) -> bool {
        self.can_insert
    }

    pub fn get_insert_pos(&self) -> sf::Vector2f {
        self.insert_pos
    }

    pub fn insert_point(&mut self, id: isize, pos: sf::Vector2f) {
        self.polygon.set_edge_contsraint(id - 1, EdgeConstraint::None);
        self.polygon.insert_point_with_pos(id, pos);
        self.update_offset();
        self.can_insert = false;
    }

    pub fn set_point_hover_color(&mut self, color: sf::Color) {
        self.hover_circle.set_fill_color(color);
    }

    pub fn remove_point(&mut self, id: isize) -> Result<(), io::Error> {
        if self.polygon.points_count() <= 3 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not enough points"));
        }
        self.polygon.set_edge_contsraint(id - 1, EdgeConstraint::None);
        self.polygon.remove_point(id);
        self.selection.remove(&(id as usize));
        self.update_offset();
        Ok(())
    }

    pub fn update_insertion(&mut self, pos: sf::Vector2f) {
        for i in 0..self.polygon.points_count() as isize {
            if my_math::distance(&pos, &self.polygon.get_point_pos(i)) <= style::POINT_DETECTION_RADIUS ||
                my_math::distance(&pos, &self.polygon.get_point_pos(i + 1)) <= style::POINT_DETECTION_RADIUS {
                continue;
            }

            let v01 = self.polygon.get_point_pos(i + 1) - self.polygon.get_point_pos(i);
            let v0m = pos - self.polygon.get_point_pos(i);

            if my_math::dot_prod(&v01, &v0m) < 0.0 {
                continue;
            }

            let proj1 = v01 * (my_math::dot_prod(&v01, &v0m) / my_math::vec_len2(&v01));

            if my_math::vec_len2(&proj1) > my_math::vec_len2(&v01) {
                continue;
            }

            let proj2 = v0m - proj1;
            let dist = my_math::vec_len(&proj2);

            if dist < style::LINE_DETECTION_DISTANCE {
                self.insert_pos = self.polygon.get_point_pos(i) + proj1;
                self.insert_circle.set_position(self.insert_pos);
                self.can_insert = true;
                return;
            }
        }
        self.can_insert = false;
    }

    fn update_on_point_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..self.polygon.points_count() as isize {
            if my_math::distance(&self.polygon.get_point_pos(i), &pos) <= style::POINT_DETECTION_RADIUS {
                self.hover_circle.set_position(self.polygon.get_point_pos(i).clone());
                self.hovered_point_id = self.polygon.fix_index(i);
                self.is_point_hovered = true;
                return;
            }
        }
        self.is_point_hovered = false;
    }

    fn update_on_line_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..self.polygon.points_count() as isize {
            let v01 = self.polygon.get_point_pos(i + 1) - self.polygon.get_point_pos(i);
            let v0m = pos - self.polygon.get_point_pos(i);

            if my_math::dot_prod(&v01, &v0m) < 0.0 {
                continue;
            }

            let proj1 = v01 * (my_math::dot_prod(&v01, &v0m) / my_math::vec_len2(&v01));

            if my_math::vec_len2(&proj1) > my_math::vec_len2(&v01) {
                continue;
            }

            let proj2 = v0m - proj1;

            let dist = my_math::vec_len(&proj2);

            if dist < style::LINE_DETECTION_DISTANCE {
                let proj_norm = my_math::vec_norm(&proj2);

                self.hover_quad.set_point(0, self.polygon.get_point_pos(i) + proj_norm * style::LINE_THICKNESS / 2.);
                self.hover_quad.set_point(1, self.polygon.get_point_pos(i + 1) + proj_norm * style::LINE_THICKNESS / 2.);
                self.hover_quad.set_point(2, self.polygon.get_point_pos(i + 1) - proj_norm * style::LINE_THICKNESS / 2.);
                self.hover_quad.set_point(3, self.polygon.get_point_pos(i) - proj_norm * style::LINE_THICKNESS / 2.);
                self.hovered_line_id = self.polygon.fix_index(i);
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
        self.polygon.assert_ccw();
        self.selection.clear();
        for i in 0..self.polygon.points_count() {
            if self.polygon.is_point_selected(i as isize) {
                self.selection.insert(i);
            }
        }
    }

    pub fn get_hovered_point_id(&self) -> usize {
        self.hovered_point_id
    }

    pub fn get_hovered_line_ids(&self) -> (usize, usize) {
        (self.hovered_line_id, self.polygon.fix_index(self.hovered_line_id as isize + 1))
    }

    pub fn select_point(&mut self, id: isize) {
        self.polygon.select_point(id);
        self.selection.insert(self.polygon.fix_index(id));
    }

    pub fn deselect_point(&mut self, id: isize) {
        self.polygon.deselect_point(id);
        self.selection.remove(&self.polygon.fix_index(id));
    }

    pub fn deselect_all_points(&mut self) {
        for id in self.selection.iter() {
            self.polygon.deselect_point(*(id) as isize);
        }
        self.selection.clear();
    }

    pub fn select_all_points(&mut self) {
        for id in 0..self.polygon.points_count() as isize {
            self.polygon.select_point(id);
            self.selection.insert(self.polygon.fix_index(id));
        }
    }

    pub fn is_point_selected(&self, id: isize) -> bool {
        self.polygon.is_point_selected(id)
    }

    pub fn is_line_selected(&self, first_id: isize) -> bool {
        self.polygon.is_point_selected(first_id) && self.polygon.is_point_selected(first_id + 1)
    }

    pub fn selected_points_count(&self) -> usize {
        self.selection.len()
    }

    pub fn move_selected_points(&mut self, vec: sf::Vector2f) {
        // Move all selected points by the given vector
        for id in self.selection.iter() {
            self.polygon.update_point_pos(self.polygon.get_point_pos(*id as isize) + vec, *id as isize);
        }

        //
        for id in self.selection.iter() {
            let prev_id = self.polygon.fix_index(*id as isize - 1) as isize;
            let mut prev_point = self.polygon.get_point_pos(prev_id);
            let next_id = self.polygon.fix_index(*id as isize + 1) as isize;
            let mut next_point = self.polygon.get_point_pos(next_id);

            if !self.selection.contains(&(prev_id as usize)) {
                if self.polygon.get_edge_constraint(prev_id) == EdgeConstraint::Vertical {
                    prev_point.x += vec.x;
                    self.polygon.update_point_pos(prev_point, prev_id);
                } else if self.polygon.get_edge_constraint(prev_id) == EdgeConstraint::Horizontal {
                    prev_point.y += vec.y;
                    self.polygon.update_point_pos(prev_point, prev_id);
                }
            }

            if !self.selection.contains(&(next_id as usize)) {
                if self.polygon.get_edge_constraint(*id as isize) == EdgeConstraint::Vertical {
                    next_point.x += vec.x;
                    self.polygon.update_point_pos(next_point, next_id);
                } else if self.polygon.get_edge_constraint(*id as isize) == EdgeConstraint::Horizontal {
                    next_point.y += vec.y;
                    self.polygon.update_point_pos(next_point, next_id);
                }
            }
        }

        self.update_offset();
    }

    pub fn draw_ctx(&self, target: &mut dyn RenderTarget) {
        self.polygon.draw_points(target);

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
            self.polygon.draw_point_selection(*id as isize, target);
        }

        self.polygon.draw_labels(target);
    }

    pub fn draw_edges(&self, target: &mut dyn RenderTarget) {
        self.polygon.draw_edges(target);

        if self.show_offset {
            self.offset_polygon.draw_edges(target);
        }
    }

    pub fn draw_bresenham_edges(&self, target: &mut dyn RenderTarget, img_target: &mut sf::Image, line_painter: &LinePainter) {
        self.polygon.draw_edges_bresenham(img_target, line_painter);

        if self.show_offset {
            self.offset_polygon.draw_edges_bresenham(img_target, line_painter);
        }
    }

    pub fn update_offset(&mut self) {
        if !self.show_offset || self.polygon.is_self_crossing() {
            return;
        }

        // Create a naive offset
        let mut naive_offset_polygon = self.polygon.clone();
        for i in 0..naive_offset_polygon.points_count() as isize {
            let vec = self.polygon.get_offset_vec(i);
            let pos = self.polygon.get_point_pos(i);
            naive_offset_polygon.update_point_pos(pos + vec * self.offset_size, i);
        }

        // Find the crossing edges in the naive offset
        let mut crossings = naive_offset_polygon.get_self_crossing_edges();

        if crossings.is_empty() || self.naive_offset {
            // If there are no crossings, the naive offset is the solution
            self.offset_polygon = naive_offset_polygon;
            self.offset_polygon.set_edges_color(style::OFFSET_COLOR);
            return;
        }

        let mut visited: Vec<bool> = Vec::new();
        visited.resize(self.polygon.points_count(), false);

        let mut outside_offset_polygon_points: Vec<sf::Vector2f> = Vec::new();
        let mut outside_offset_polygon_points_ids: Vec<usize> = Vec::new();

        // Find min x point in order to find outside offset
        let mut start = 0;
        for index in 0..naive_offset_polygon.points_count() {
            if visited[index] {
                continue;
            }

            let pos = naive_offset_polygon.get_point_pos(index as isize);
            let pos_old = naive_offset_polygon.get_point_pos(start as isize);

            if pos.x < pos_old.x {
                start = index;
            }
        }

        // Make "start" an immutable and begin the outside offset algorithm
        let start = start;
        let mut i = start;

        // Safety break (prevents infinite loops in case the algorithm doesn't work)
        let mut iterations_inner = 0;

        loop {
            // Create a new polygon
            let curr_point = naive_offset_polygon.get_point_pos(i as isize);

            // Push the current point into the offset polygon
            outside_offset_polygon_points.push(curr_point);
            outside_offset_polygon_points_ids.push(i);

            // Find crossings of the line starting with the point "i"
            let mut curr_line_crossings = crossings.get(&i);
            if let Some(curr_line_crossings) = curr_line_crossings {
                // Find the closest intersection
                let mut min_dist = f32::INFINITY;
                let mut min_id: Option<usize> = None;
                for (id, curr_crossing) in curr_line_crossings.iter().enumerate() {
                    let curr_dist = my_math::distance2(&curr_point, &curr_crossing.1);
                    if curr_dist < min_dist {
                        min_dist = curr_dist;
                        min_id = Some(id);
                    }
                }

                let mut closest_intersection =
                    (curr_line_crossings[min_id.unwrap()].0, curr_line_crossings[min_id.unwrap()].1);

                // Push the closest intersection point
                outside_offset_polygon_points.push(closest_intersection.1);

                let mut new_line_crossings = crossings.get(&closest_intersection.0);
                let mut prev_line = i;
                while new_line_crossings.is_some() {
                    // Find the closest intersection that is on the proper side
                    let mut min_dist = f32::INFINITY;
                    let mut min_id: Option<usize> = None;
                    for (id, curr_crossing) in new_line_crossings.unwrap().iter().enumerate() {
                        if !is_right_turn(
                            &outside_offset_polygon_points[outside_offset_polygon_points.len() - 1],
                            &outside_offset_polygon_points[outside_offset_polygon_points.len() - 2],
                            &curr_crossing.1,
                        ) || prev_line == curr_crossing.0 {
                            continue;
                        }
                        let curr_dist = my_math::distance2(&outside_offset_polygon_points[outside_offset_polygon_points.len() - 1], &curr_crossing.1);
                        if curr_dist < min_dist {
                            min_dist = curr_dist;
                            min_id = Some(id);
                        }
                    }

                    if min_id.is_none() {
                        // All intersection are not on the proper side
                        break;
                    }

                    // Update prev_line
                    prev_line = closest_intersection.0;

                    closest_intersection = (new_line_crossings.unwrap()[min_id.unwrap()].0, new_line_crossings.unwrap()[min_id.unwrap()].1);
                    outside_offset_polygon_points.push(closest_intersection.1);
                    new_line_crossings = crossings.get(&closest_intersection.0);
                }

                if is_right_turn(
                    &outside_offset_polygon_points[outside_offset_polygon_points.len() - 1],
                    &outside_offset_polygon_points[outside_offset_polygon_points.len() - 2],
                    &closest_intersection.1,
                ) {
                    i = closest_intersection.0;
                } else {
                    i = naive_offset_polygon.fix_index(closest_intersection.0 as isize + 1);
                }
            } else {
                i = naive_offset_polygon.fix_index(i as isize + 1);
            }

            // Safety break
            iterations_inner += 1;
            if iterations_inner > naive_offset_polygon.points_count() {
                break;
            }

            if i == start {
                break;
            }
        }
        outside_offset_polygon_points.push(naive_offset_polygon.get_point_pos(start as isize));

        self.offset_polygon = Polygon::create(outside_offset_polygon_points);
        self.offset_polygon.set_edges_color(style::OFFSET_COLOR);
    }

    fn draw_line_constraints_egui(&mut self, id: isize, ui: &mut egui::Ui) {
        let line_prev = self.polygon.fix_index(id - 1) as isize;
        let line0 = self.polygon.fix_index(id) as isize;
        let line1 = self.polygon.fix_index(id + 1) as isize;

        let p0 = self.polygon.get_point_pos(line0);
        let p1 = self.polygon.get_point_pos(line1);

        // Pick the drawing method
        let mut old = self.polygon.get_edge_constraint(line0);
        let mut new = old.clone();

        egui::ComboBox::from_label(format!("({}, {}) Constraint", line0, line1))
            .selected_text(match new {
                EdgeConstraint::None => "None",
                EdgeConstraint::Horizontal => "Horizontal",
                EdgeConstraint::Vertical => "Vertical"
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut new, EdgeConstraint::None, "None");
                if (p1.x - p0.x).abs() > style::POINT_DETECTION_RADIUS &&
                    self.polygon.get_edge_constraint(line_prev) != EdgeConstraint::Horizontal &&
                    self.polygon.get_edge_constraint(line1) != EdgeConstraint::Horizontal {
                    ui.selectable_value(&mut new, EdgeConstraint::Horizontal, "Horizontal");
                }
                if (p1.y - p0.y).abs() > style::POINT_DETECTION_RADIUS &&
                    self.polygon.get_edge_constraint(line_prev) != EdgeConstraint::Vertical &&
                    self.polygon.get_edge_constraint(line1) != EdgeConstraint::Vertical {
                    ui.selectable_value(&mut new, EdgeConstraint::Vertical, "Vertical");
                }
            });

        if old != new {
            if new != EdgeConstraint::None &&
                (new == self.polygon.get_edge_constraint(line0 - 1) ||
                    new == self.polygon.get_edge_constraint(line1)) {
                return;
            }
            self.polygon.set_edge_contsraint(line0, new.clone());

            match new {
                EdgeConstraint::Horizontal => {
                    let avg = (p0.y + p1.y) / 2.;

                    self.polygon.update_point_pos(sf::Vector2f::new(p0.x, avg), line0);
                    self.polygon.update_point_pos(sf::Vector2f::new(p1.x, avg), line1);
                }
                EdgeConstraint::Vertical => {
                    let avg = (p0.x + p1.x) / 2.;
                    self.polygon.update_point_pos(sf::Vector2f::new(avg, p0.y), line0);
                    self.polygon.update_point_pos(sf::Vector2f::new(avg, p1.y), line1);
                }
                EdgeConstraint::None => (),
            }
            if self.polygon.is_self_crossing() {
                self.polygon.update_point_pos(p0, line0);
                self.polygon.update_point_pos(p1, line1);
                self.polygon.set_edge_contsraint(line0, old);
            } else {
                self.update_offset();
            }
        }
    }

    pub fn draw_selected_edge_egui(&mut self, ui: &mut egui::Ui) -> bool {
        if self.selection.len() != 2 {
            return false;
        }

        if let Some(id) = self.selection.iter().next() {
            let next_id = self.polygon.fix_index(*id as isize + 1);
            let prev_id = self.polygon.fix_index(*id as isize - 1);

            if self.selection.contains(&next_id) {
                self.draw_line_constraints_egui(*id as isize, ui);
                return true;
            }
            if self.selection.contains(&prev_id) {
                self.draw_line_constraints_egui(prev_id as isize, ui);
                return true;
            }
        }
        return false;
    }

    pub fn draw_polygon_options_egui(&mut self, ui: &mut egui::Ui) {
        let mut show_offset = self.show_offset;
        let mut offset = self.offset_size;
        let mut naive = self.naive_offset;

        ui.checkbox(&mut show_offset, "Show Offset");
        ui.checkbox(&mut naive, "Naive Offset");
        ui.add(egui::Slider::new(&mut offset, 0.0..=style::MAX_OFFSET).text("Offset"));

        if show_offset != self.show_offset || offset != self.offset_size || naive != self.naive_offset {
            self.offset_size = offset;
            self.naive_offset = naive;
            self.show_offset = show_offset;
            self.update_offset();
        }
    }

    pub fn draw_egui(&mut self, ui: &mut egui::Ui) {
        self.draw_polygon_options_egui(ui);

        egui::CollapsingHeader::new("Edges")
            .default_open(false)
            .show(ui, |ui| {
                for id in 0..self.polygon.points_count() as isize {
                    self.draw_line_constraints_egui(id, ui);
                }
            });
    }
}