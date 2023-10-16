use std::ops::Add;
use sfml::system::Vector2f;
use super::sf;
use super::AppContext;

pub trait State {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_left_mouse_released(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_ctrl_a_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn update(&mut self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext);
    fn state_name(&self) -> &'static str;
}

pub struct IdleState;

impl IdleState {
    pub fn new(app_ctx: &mut AppContext) -> IdleState {
        for poly in app_ctx.polygons.iter_mut() {
            poly.enable_hover_show()
        }

        IdleState
    }
}

pub struct AddPolygonState;

impl AddPolygonState {
    pub fn new(app_ctx: &mut AppContext) -> AddPolygonState {
        for poly in app_ctx.polygons.iter_mut() {
            poly.disable_hover_show()
        }
        app_ctx.polygon_builder.start();

        AddPolygonState
    }
}

pub struct SelectionState;

impl SelectionState {
    pub fn new(app_ctx: &mut AppContext) -> SelectionState {
        for poly in app_ctx.polygons.iter_mut() {
            poly.enable_hover_show()
        }

        SelectionState
    }
}

pub struct DraggingState {
    prev_mouse_point: sf::Vector2f,
    start_mouse_point: sf::Vector2f,
}

impl DraggingState {
    pub fn new(mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> DraggingState {
        for poly in app_ctx.polygons.iter_mut() {
            poly.disable_hover_show()
        }

        DraggingState {
            prev_mouse_point: mouse_pos,
            start_mouse_point: mouse_pos,
        }
    }
}


pub struct AddPointState;

impl AddPointState {
    pub fn new(app_ctx: &mut AppContext) -> AddPointState {
        for poly in app_ctx.polygons.iter_mut() {
            poly.disable_hover_show()
        }

        AddPointState
    }
}

impl State for AddPolygonState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let poly_opt = app_ctx.polygon_builder.add_or_build(mouse_pos);
        if let Some(poly) = poly_opt {
            app_ctx.polygons.push(poly);
            return Box::new(IdleState::new(app_ctx));
        }
        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_a_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.cancel();
        Box::new(AddPointState::new(app_ctx))
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.cancel();
        Box::new(IdleState::new(app_ctx))
    }

    fn update(&mut self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) {
        app_ctx.polygon_builder.update(dt, mouse_pos);
    }

    fn state_name(&self) -> &'static str {
        "Add Polygon State"
    }
}

impl IdleState {
    fn select_points_and_return_state(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext, success_result: Box<dyn State>) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_point_hovered() {
                let err = poly.select_point(poly.get_hovered_point_id());

                if err.is_ok() {
                    return success_result;
                }
            } else if poly.is_line_hovered() {
                let line = poly.get_hovered_line_ids();
                let err = poly.select_point(line.0);

                if err.is_err() {
                    continue;
                }

                let err = poly.select_point(line.1);

                if err.is_ok() {
                    return success_result;
                }
            }
        }

        self
    }
}

impl State for IdleState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let result = Box::new(DraggingState::new(mouse_pos, app_ctx));
        self.select_points_and_return_state(
            mouse_pos,
            app_ctx,
            result,
        )
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let result = Box::new(SelectionState::new(app_ctx));
        self.select_points_and_return_state(
            mouse_pos,
            app_ctx,
            result,
        )
    }

    fn on_ctrl_a_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_line_hovered() || poly.is_point_hovered() {
                poly.select_all_points();
                return Box::new(SelectionState::new(app_ctx));
            }
        }
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        Box::new(AddPolygonState::new(app_ctx))
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        Box::new(AddPointState::new(app_ctx))
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn update(&mut self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) {
        for poly in app_ctx.polygons.iter_mut() {
            poly.update_hover(mouse_pos);
        }
    }

    fn state_name(&self) -> &'static str {
        "Idle State"
    }
}

impl State for SelectionState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let mut nothing_hovered = true;

        for i in 0..app_ctx.polygons.len() {
            if app_ctx.polygons[i].is_point_hovered() {
                if let Ok(is_selected) = app_ctx.polygons[i].is_point_selected(app_ctx.polygons[i].get_hovered_point_id()) {
                    if !is_selected {
                        for j in 0..app_ctx.polygons.len() {
                            app_ctx.polygons[j].deselect_all_points();
                        }
                        let id = app_ctx.polygons[i].get_hovered_point_id();
                        let _err = app_ctx.polygons[i].select_point(id);
                    }
                    return Box::new(DraggingState::new(mouse_pos, app_ctx));
                }
                nothing_hovered = false;
            } else if app_ctx.polygons[i].is_line_hovered() {
                if let Ok(is_selected) = app_ctx.polygons[i].is_line_selected(app_ctx.polygons[i].get_hovered_line_ids().0) {
                    if !is_selected {
                        for j in 0..app_ctx.polygons.len() {
                            app_ctx.polygons[j].deselect_all_points();
                        }
                        let line = app_ctx.polygons[i].get_hovered_line_ids();
                        let _err = app_ctx.polygons[i].select_point(line.0);
                        let _err = app_ctx.polygons[i].select_point(line.1);
                    }
                    return Box::new(DraggingState::new(mouse_pos, app_ctx));
                }
                nothing_hovered = false;
            }
        }

        if nothing_hovered {
            for poly in app_ctx.polygons.iter_mut() {
                poly.deselect_all_points();
            }
            return Box::new(IdleState::new(app_ctx));
        }

        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let mut nothing_hovered = true;

        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_point_hovered() {
                if let Ok(is_selected) = poly.is_point_selected(poly.get_hovered_point_id()) {
                    if is_selected {
                        let _err = poly.deselect_point(poly.get_hovered_point_id());

                        if poly.selected_points_count() == 0 {
                            return Box::new(IdleState::new(app_ctx));
                        }
                    } else {
                        let _err = poly.select_point(poly.get_hovered_point_id());
                    }
                    nothing_hovered = false;
                }
            } else if poly.is_line_hovered() {
                let line = poly.get_hovered_line_ids();
                if let Ok(is_selected) = poly.is_line_selected(line.0) {
                    if is_selected {
                        let _err = poly.deselect_point(line.0);
                        let _err = poly.deselect_point(line.1);

                        if poly.selected_points_count() == 0 {
                            return Box::new(IdleState::new(app_ctx));
                        }
                    } else {
                        let _err = poly.select_point(line.0);
                        let _err = poly.select_point(line.1);
                    }
                    nothing_hovered = false;
                }
            }
        }

        if nothing_hovered {
            for poly in app_ctx.polygons.iter_mut() {
                poly.deselect_all_points();
            }
            return Box::new(IdleState::new(app_ctx));
        }

        self
    }

    fn on_ctrl_a_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let mut nothing_hovered = true;

        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_line_hovered() || poly.is_point_hovered() {
                poly.select_all_points();
                nothing_hovered = false;
            }
        }

        if nothing_hovered {
            for poly in app_ctx.polygons.iter_mut() {
                poly.deselect_all_points();
            }
        }
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            poly.deselect_all_points();
        }

        return Box::new(AddPolygonState::new(app_ctx));
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            poly.deselect_all_points();
        }

        return Box::new(AddPointState::new(app_ctx));
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn update(&mut self, dt: f32, mouse_pos: Vector2f, app_ctx: &mut AppContext) {
        for poly in app_ctx.polygons.iter_mut() {
            poly.update_hover(mouse_pos);
        }
    }

    fn state_name(&self) -> &'static str {
        "Selection State"
    }
}

impl State for DraggingState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            if poly.raw_polygon().is_self_crossing() {
                // Revert changes
                poly.move_selected_points(self.start_mouse_point - mouse_pos);
            } else {
                poly.assert_ccw();
            }
        }
        Box::new(SelectionState::new(app_ctx))
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_a_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn update(&mut self, dt: f32, mouse_pos: Vector2f, app_ctx: &mut AppContext) {
        for poly in app_ctx.polygons.iter_mut() {
            poly.move_selected_points(mouse_pos - self.prev_mouse_point);
        }
        self.prev_mouse_point = mouse_pos;
    }

    fn state_name(&self) -> &'static str {
        "Dragging State"
    }
}

impl State for AddPointState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            if poly.can_insert() {
                let line = poly.get_hovered_line_ids();
                let _err = poly.insert_point(line.1, poly.get_insert_pos());
                return Box::new(IdleState::new(app_ctx));
            }
        }
        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_a_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        Box::new(AddPolygonState::new(app_ctx))
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        Box::new(IdleState::new(app_ctx))
    }

    fn update(&mut self, dt: f32, mouse_pos: Vector2f, app_ctx: &mut AppContext) {
        for poly in app_ctx.polygons.iter_mut() {
            poly.update_insertion(mouse_pos);
            poly.update_hover(mouse_pos);
        }
    }

    fn state_name(&self) -> &'static str {
        "Add Point State"
    }
}