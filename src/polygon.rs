pub mod raw_polygon;

use std::io;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use egui_sfml::egui;
use rand::Rng;
use sfml::graphics::{CircleShape, Drawable, RcTexture, RenderTarget, Shape, Transformable};
use super::sf;

use raw_polygon::Polygon;
use raw_polygon::EdgeConstraint;
use crate::my_math::{cross2, is_ccw, is_right_turn};
use crate::polygon::raw_polygon::EdgeConstraint::Horizontal;
use super::style;
use super::my_math;

pub struct PolygonBuilder<'s> {
    raw_polygon: Option<Polygon<'s>>,

    active: bool,

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

impl<'a> PolygonBuilder<'a> {
    pub fn get_resources(&self) -> (&Rc<sf::RcTexture>, &Rc<sf::RcFont>) {
        (&self.constraint_texture, &self.font)
    }
    pub fn new() -> PolygonBuilder<'a> {
        let mut helper_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 30);
        helper_circle.set_fill_color(style::POINT_DETECTION_COLOR_CORRECT);
        helper_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));

        let mut new_point_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 30);
        new_point_circle.set_fill_color(style::POINTS_COLOR);
        new_point_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));
        new_point_circle.set_position(sf::Vector2f::new(-100.0, -100.0));

        PolygonBuilder {
            raw_polygon: None,
            active: false,
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
        if self.raw_polygon.is_none() {
            self.raw_polygon = Some(Polygon::new_with_start_point(point));
            self.raw_polygon.as_mut().unwrap().set_label_resources(&self.constraint_texture, &self.font);
            self.raw_polygon.as_mut().unwrap().show_last_line(false);
            self.raw_polygon.as_mut().unwrap().set_name(format!("Polygon {}", self.curr_id));
            self.update_line(point, point);
            self.new_point_circle.set_position(point);

            self.curr_id += 1;
            return;
        }

        if let Some(ref mut polygon) = self.raw_polygon {
            polygon.push_point_with_pos(point);
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
                for i in 1..self.raw_polygon.as_ref().unwrap().points_count() {
                    if my_math::distance(&add_pos, &self.raw_polygon.as_ref().unwrap().get_point_pos(i as isize)) <= style::POLY_EDGE_MIN_LEN {
                        return None;
                    }
                }
            } else {
                if self.raw_polygon.as_ref().unwrap().points_count() >= 3 {
                    // If this condition is met, adding a new polygon is finished

                    self.update_line(sf::Vector2f::new(0.0, 0.0), sf::Vector2::new(0.0, 0.0));
                    self.new_point_circle.set_position(sf::Vector2f::new(-100.0, -100.0));

                    // Deactivate the builder
                    self.active = false;
                    self.clear_draw_flags();

                    // Build the PolygonObject
                    self.raw_polygon.as_mut().unwrap().assert_ccw();
                    self.raw_polygon.as_mut().unwrap().show_last_line(true);
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

    pub fn raw_polygon(&self) -> Option<&Polygon> {
        self.raw_polygon.as_ref()
    }
    pub fn draw_ctx(&self, target: &mut dyn sf::RenderTarget) {
        if let Some(poly) = self.raw_polygon.as_ref() {
            poly.draw_points(target);
        }

        if self.active {
            self.new_line.draw(target, &Default::default());
            target.draw(&self.new_point_circle);
        }

        if self.entered_correct_vertex_region {
            target.draw(&self.helper_circle);
        }

        target.draw(&self.new_line);
    }

    pub fn draw_edges(&self, target: &mut dyn sf::RenderTarget) {
        if let Some(poly) = self.raw_polygon.as_ref() {
            poly.draw_edges(target);
        }
    }

    pub fn draw_bresenham_edges(&self, target: &mut dyn sf::RenderTarget, img_target: &mut sf::Image) {
        if let Some(poly) = self.raw_polygon.as_ref() {
            poly.draw_edges_bresenham(img_target);
        }
    }
}

pub struct PolygonObject<'a> {
    raw_polygon: Polygon<'a>,

    // Selection
    selection: HashSet<usize>,

    show_hover: bool,

    // Draw Offset 
    show_offset: bool,
    naive_offset: bool,
    offset_size: f32,
    offset_polygon: Polygon<'a>,

    // Point hover
    hover_circle: sf::CircleShape<'a>,
    is_point_hovered: bool,
    hovered_point_id: usize,

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
            raw_polygon: raw,
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

    pub fn raw_polygon(&self) -> &Polygon {
        &self.raw_polygon
    }

    pub fn can_insert(&self) -> bool {
        self.can_insert
    }

    pub fn get_insert_pos(&self) -> sf::Vector2f {
        self.insert_pos
    }

    pub fn insert_point(&mut self, id: isize, pos: sf::Vector2f) {
        self.raw_polygon.set_edge_contsraint(id - 1, EdgeConstraint::None);
        self.raw_polygon.insert_point_with_pos(id, pos);
        self.update_offset();
        self.can_insert = false;
    }

    pub fn set_point_hover_color(&mut self, color: sf::Color) {
        self.hover_circle.set_fill_color(color);
    }

    pub fn remove_point(&mut self, id: isize) -> Result<(), io::Error> {
        if self.raw_polygon.points_count() <= 3 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not enough points"));
        }
        self.raw_polygon.set_edge_contsraint(id - 1, EdgeConstraint::None);
        self.raw_polygon.remove_point(id);
        self.selection.remove(&(id as usize));
        self.update_offset();
        Ok(())
    }

    pub fn update_insertion(&mut self, pos: sf::Vector2f) {
        for i in 0..self.raw_polygon.points_count() as isize {
            if my_math::distance(&pos, &self.raw_polygon.get_point_pos(i)) <= style::POINT_DETECTION_RADIUS ||
                my_math::distance(&pos, &self.raw_polygon.get_point_pos(i + 1)) <= style::POINT_DETECTION_RADIUS {
                continue;
            }

            let v01 = self.raw_polygon.get_point_pos(i + 1) - self.raw_polygon.get_point_pos(i);
            let v0m = pos - self.raw_polygon.get_point_pos(i);

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
                self.insert_pos = self.raw_polygon.get_point_pos(i) + proj1;
                self.insert_circle.set_position(self.insert_pos);
                self.can_insert = true;
                return;
            }
        }
        self.can_insert = false;
    }

    fn update_on_point_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..self.raw_polygon.points_count() as isize {
            if my_math::distance(&self.raw_polygon.get_point_pos(i), &pos) <= style::POINT_DETECTION_RADIUS {
                self.hover_circle.set_position(self.raw_polygon.get_point_pos(i).clone());
                self.hovered_point_id = self.raw_polygon.fix_index(i);
                self.is_point_hovered = true;
                return;
            }
        }
        self.is_point_hovered = false;
    }

    fn update_on_line_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..self.raw_polygon.points_count() as isize {
            let v01 = self.raw_polygon.get_point_pos(i + 1) - self.raw_polygon.get_point_pos(i);
            let v0m = pos - self.raw_polygon.get_point_pos(i);

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

                self.hover_quad.set_point(0, self.raw_polygon.get_point_pos(i) + proj_norm * style::LINE_THICKNESS / 2.);
                self.hover_quad.set_point(1, self.raw_polygon.get_point_pos(i + 1) + proj_norm * style::LINE_THICKNESS / 2.);
                self.hover_quad.set_point(2, self.raw_polygon.get_point_pos(i + 1) - proj_norm * style::LINE_THICKNESS / 2.);
                self.hover_quad.set_point(3, self.raw_polygon.get_point_pos(i) - proj_norm * style::LINE_THICKNESS / 2.);
                self.hovered_line_id = self.raw_polygon.fix_index(i);
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
        self.selection.clear();
        for i in 0..self.raw_polygon.points_count() {
            if self.raw_polygon.is_point_selected(i as isize) {
                self.selection.insert(i);
            }
        }
    }

    pub fn get_hovered_point_id(&self) -> usize {
        self.hovered_point_id
    }

    pub fn get_hovered_line_ids(&self) -> (usize, usize) {
        (self.hovered_line_id, self.raw_polygon.fix_index(self.hovered_line_id as isize + 1))
    }

    pub fn select_point(&mut self, id: isize) {
        self.raw_polygon.select_point(id);
        self.selection.insert(self.raw_polygon.fix_index(id));
    }

    pub fn deselect_point(&mut self, id: isize) {
        self.raw_polygon.deselect_point(id);
        self.selection.remove(&self.raw_polygon.fix_index(id));
    }

    pub fn deselect_all_points(&mut self) {
        for id in self.selection.iter() {
            self.raw_polygon.deselect_point(*(id) as isize);
        }
        self.selection.clear();
    }

    pub fn select_all_points(&mut self) {
        for id in 0..self.raw_polygon.points_count() as isize {
            self.raw_polygon.select_point(id);
            self.selection.insert(self.raw_polygon.fix_index(id));
        }
    }

    pub fn is_point_selected(&self, id: isize) -> bool {
        self.raw_polygon.is_point_selected(id)
    }

    pub fn is_line_selected(&self, first_id: isize) -> bool {
        self.raw_polygon.is_point_selected(first_id) && self.raw_polygon.is_point_selected(first_id + 1)
    }

    pub fn selected_points_count(&self) -> usize {
        self.selection.len()
    }

    pub fn move_selected_points(&mut self, vec: sf::Vector2f) {
        let mut moved = Vec::new();
        moved.resize(self.raw_polygon.points_count(), false);

        for id in self.selection.iter() {
            if moved[*id] {
                continue;
            }

            self.raw_polygon.update_point_pos(self.raw_polygon.get_point_pos(*id as isize) + vec, *id as isize);
            moved[*id] = true;

            let prev_id = self.raw_polygon.fix_index(*id as isize - 1) as isize;
            let mut prev_point = self.raw_polygon.get_point_pos(prev_id);
            let next_id = self.raw_polygon.fix_index(*id as isize + 1) as isize;
            let mut next_point = self.raw_polygon.get_point_pos(next_id);

            if self.raw_polygon.get_edge_constraint(prev_id) == EdgeConstraint::Vertical {
                prev_point.x += vec.x;
                self.raw_polygon.update_point_pos(prev_point, prev_id);
                moved[prev_id as usize] = true;
            } else if self.raw_polygon.get_edge_constraint(prev_id) == EdgeConstraint::Horizontal {
                prev_point.y += vec.y;
                self.raw_polygon.update_point_pos(prev_point, prev_id);
                moved[prev_id as usize] = true;
            }

            if self.raw_polygon.get_edge_constraint(*id as isize) == EdgeConstraint::Vertical {
                next_point.x += vec.x;
                self.raw_polygon.update_point_pos(next_point, next_id);
                moved[next_id as usize] = true;
            } else if self.raw_polygon.get_edge_constraint(*id as isize) == EdgeConstraint::Horizontal {
                next_point.y += vec.y;
                self.raw_polygon.update_point_pos(next_point, next_id);
                moved[next_id as usize] = true;
            }
        }

        self.update_offset();
    }

    pub fn draw_ctx(&self, target: &mut dyn RenderTarget) {
        self.raw_polygon.draw_points(target);

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
            self.raw_polygon.draw_point_selection(*id as isize, target);
        }

        self.raw_polygon.draw_labels(target);
    }

    pub fn draw_edges(&self, target: &mut dyn RenderTarget) {
        self.raw_polygon.draw_edges(target);

        if self.show_offset {
            self.offset_polygon.draw_edges(target);
        }
    }

    pub fn draw_bresenham_edges(&self, target: &mut dyn RenderTarget, img_target: &mut sf::Image) {
        self.raw_polygon.draw_edges_bresenham(img_target);

        if self.show_offset {
            self.offset_polygon.draw_edges_bresenham(img_target);
        }
    }

    pub fn update_offset(&mut self) {
        if !self.show_offset || self.raw_polygon.is_self_crossing() {
            return;
        }

        // Create a naive offset
        let mut naive_offset_polygon = self.raw_polygon.clone();
        for i in 0..naive_offset_polygon.points_count() as isize {
            let vec = self.raw_polygon.get_offset_vec(i);
            let pos = self.raw_polygon.get_point_pos(i);
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
        visited.resize(self.raw_polygon.points_count(), false);

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
        let line_prev = self.raw_polygon.fix_index(id - 1) as isize;
        let line0 = self.raw_polygon.fix_index(id) as isize;
        let line1 = self.raw_polygon.fix_index(id + 1) as isize;

        let p0 = self.raw_polygon.get_point_pos(line0);
        let p1 = self.raw_polygon.get_point_pos(line1);

        // Pick the drawing method
        let mut old = self.raw_polygon.get_edge_constraint(line0);
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
                    self.raw_polygon.get_edge_constraint(line_prev) != EdgeConstraint::Horizontal &&
                    self.raw_polygon.get_edge_constraint(line1) != EdgeConstraint::Horizontal {
                    ui.selectable_value(&mut new, EdgeConstraint::Horizontal, "Horizontal");
                }
                if (p1.y - p0.y).abs() > style::POINT_DETECTION_RADIUS &&
                    self.raw_polygon.get_edge_constraint(line_prev) != EdgeConstraint::Vertical &&
                    self.raw_polygon.get_edge_constraint(line1) != EdgeConstraint::Vertical {
                    ui.selectable_value(&mut new, EdgeConstraint::Vertical, "Vertical");
                }
            });

        if old != new {
            if new != EdgeConstraint::None &&
                (new == self.raw_polygon.get_edge_constraint(line0 - 1) ||
                    new == self.raw_polygon.get_edge_constraint(line1)) {
                return;
            }
            self.raw_polygon.set_edge_contsraint(line0, new.clone());

            match new {
                EdgeConstraint::Horizontal => {
                    let avg = (p0.y + p1.y) / 2.;

                    self.raw_polygon.update_point_pos(sf::Vector2f::new(p0.x, avg), line0);
                    self.raw_polygon.update_point_pos(sf::Vector2f::new(p1.x, avg), line1);
                }
                EdgeConstraint::Vertical => {
                    let avg = (p0.x + p1.x) / 2.;
                    self.raw_polygon.update_point_pos(sf::Vector2f::new(avg, p0.y), line0);
                    self.raw_polygon.update_point_pos(sf::Vector2f::new(avg, p1.y), line1);
                }
                EdgeConstraint::None => (),
            }
            if self.raw_polygon.is_self_crossing() {
                self.raw_polygon.update_point_pos(p0, line0);
                self.raw_polygon.update_point_pos(p1, line1);
                self.raw_polygon.set_edge_contsraint(line0, old);
            } else {
                self.update_offset();
            }
        }
    }

    pub fn draw_selection_egui(&mut self, ui: &mut egui::Ui) -> bool {
        if self.selection.len() != 2 {
            return false;
        }

        if let Some(id) = self.selection.iter().next() {
            let next_id = self.raw_polygon.fix_index(*id as isize + 1);
            let prev_id = self.raw_polygon.fix_index(*id as isize - 1);

            if self.selection.contains(&next_id) {
                self.draw_line_constraints_egui(*id as isize, ui);
            }
            if self.selection.contains(&prev_id) {
                self.draw_line_constraints_egui(prev_id as isize, ui);
            }
        }
        return false;
    }

    pub fn draw_egui(&mut self, ui: &mut egui::Ui) {
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

        egui::CollapsingHeader::new("Edges")
            .default_open(true)
            .show(ui, |ui| {
                for id in 0..self.raw_polygon.points_count() as isize {
                    self.draw_line_constraints_egui(id, ui);
                }
            });
    }
}