pub mod raw_polygon;

use std::io;
use std::collections::{HashMap, HashSet};
use egui_sfml::egui;
use sfml::graphics::{Drawable, RenderTarget, Shape, Transformable};
use super::sf;

use raw_polygon::Polygon;
use raw_polygon::EdgeConstraint;
use super::style;
use super::my_math;

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
        let mut helper_circle = sf::CircleShape::new(style::POINT_DETECTION_RADIUS, 30);
        helper_circle.set_fill_color(style::POINT_DETECTION_COLOR_CORRECT);
        helper_circle.set_origin(sf::Vector2f::new(style::POINT_DETECTION_RADIUS, style::POINT_DETECTION_RADIUS));

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
                    if my_math::distance(&add_pos, &self.raw_polygon.as_ref().unwrap().get_point_pos(i)) <= style::POLY_EDGE_MIN_LEN {
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
                    self.raw_polygon.as_mut().unwrap().move_last_to_make_proper();

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

            if my_math::distance(&first, &m_pos) <= style::POINT_DETECTION_RADIUS {
                if poly.points_count() > 3 {
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
                poly.set_edges_color(style::LINES_COLOR_INCORRECT);
            } else {
                poly.set_edges_color(style::LINES_COLOR);
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
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not enough points"));
        }

        self.raw_polygon.remove_point(id);
        self.selection.remove(&id);

        Ok(())
    }

    pub fn update_insertion(&mut self, pos: sf::Vector2f) {
        for i in 0..(self.raw_polygon.points_count() - 1) {
            if my_math::distance(&pos, &self.raw_polygon.get_point_pos(i)) <= style::POINT_DETECTION_RADIUS ||
                my_math::distance(&pos, &self.raw_polygon.get_point_pos(i)) <= style::POINT_DETECTION_RADIUS {
                continue;
            }

            let v01 = self.raw_polygon.get_point_pos(i) - self.raw_polygon.get_point_pos(i);
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
        for i in 0..self.raw_polygon.points_count() {
            if my_math::distance(&self.raw_polygon.get_point_pos(i), &pos) <= style::POINT_DETECTION_RADIUS {
                self.hover_circle.set_position(self.raw_polygon.get_point_pos(i).clone());
                self.hovered_point_id = i;
                self.is_point_hovered = true;
                return;
            }
        }
        self.is_point_hovered = false;
    }

    fn update_on_line_hover(&mut self, pos: sf::Vector2f) {
        for i in 0..self.raw_polygon.points_count() {
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

    // id must be less than points_count - 1

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
                        let mut old = self.raw_polygon.points[*id].edge_constraint.clone();

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

                        if old != self.raw_polygon.points[*id].edge_constraint {
                            println!("CHANGED");
                            match self.raw_polygon.points[*id].edge_constraint {
                                EdgeConstraint::Horizontal => {
                                    let old_p0 = self.raw_polygon.points[*id].pos;
                                    let old_p1 = self.raw_polygon.points[*id + 1].pos;

                                    let avg = (old_p0 + old_p1);
                                    let len = distance(&old_p0, &old_p1);

                                    self.raw_polygon.points[*id].pos = (avg + sf::Vector2f::new(-len, 0.)) / 2.;
                                    self.raw_polygon.points[*id + 1].pos = (avg + sf::Vector2f::new(len, 0.)) / 2.;

                                    let _err = self.raw_polygon.update_point(self.raw_polygon.points[*id].pos, *id);
                                    let _err = self.raw_polygon.update_point(self.raw_polygon.points[*id + 1].pos, *id + 1);

                                    if *id == 0 {
                                        let _err = self.raw_polygon.update_point(self.raw_polygon.points[0].pos, self.raw_polygon.points_count() - 1);
                                    }
                                }
                                EdgeConstraint::Vertical => {}
                                EdgeConstraint::None => (),
                            }
                        }
                    }
                }
            });
    }
}