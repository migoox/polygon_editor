use sfml::system::Vector2f;
use super::sf;
use super::AppContext;

pub trait State {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_left_mouse_released(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn update(&mut self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext);
    fn state_name(&self) -> &'static str;
}

pub struct IdleState;

pub struct AddPolygonState;

pub struct SelectionState;

pub struct DraggingState {
    prev_mouse_point: sf::Vector2f,
    start_mouse_point: sf::Vector2f,
}

pub struct AddPointState;

impl State for AddPolygonState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let poly_opt = app_ctx.polygon_builder.add_or_build(mouse_pos);
        if let Some(poly) = poly_opt {
            app_ctx.polygons.push(poly);
            return Box::new(IdleState);
        }
        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.cancel();
        Box::new(AddPointState)
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.cancel();
        Box::new(IdleState)
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
        self.select_points_and_return_state(
            mouse_pos,
            app_ctx,
            Box::new(DraggingState {
                start_mouse_point: mouse_pos,
                prev_mouse_point: mouse_pos,
            }),
        )
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self.select_points_and_return_state(
            mouse_pos,
            app_ctx,
            Box::new(SelectionState),
        )
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.start();
        Box::new(AddPolygonState)
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        Box::new(AddPointState)
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
        let mut no_point_hovered = true;

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
                    return Box::new(DraggingState { prev_mouse_point: mouse_pos, start_mouse_point: mouse_pos });
                }
                no_point_hovered = false;
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
                    return Box::new(DraggingState { prev_mouse_point: mouse_pos, start_mouse_point: mouse_pos });
                }
                no_point_hovered = false;
            }
        }

        if no_point_hovered {
            for poly in app_ctx.polygons.iter_mut() {
                poly.deselect_all_points();
            }
            return Box::new(IdleState);
        }

        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let mut no_point_hovered = true;

        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_point_hovered() {
                if let Ok(is_selected) = poly.is_point_selected(poly.get_hovered_point_id()) {
                    if is_selected {
                        let _err = poly.deselect_point(poly.get_hovered_point_id());

                        if poly.selected_points_count() == 0 {
                            return Box::new(IdleState);
                        }
                    } else {
                        let _err = poly.select_point(poly.get_hovered_point_id());
                    }
                    no_point_hovered = false;
                }
            } else if poly.is_line_hovered() {
                let line = poly.get_hovered_line_ids();
                if let Ok(is_selected) = poly.is_line_selected(line.0) {
                    if is_selected {
                        let _err = poly.deselect_point(line.0);
                        let _err = poly.deselect_point(line.1);

                        if poly.selected_points_count() == 0 {
                            return Box::new(IdleState);
                        }
                    } else {
                        let _err = poly.select_point(line.0);
                        let _err = poly.select_point(line.1);
                    }
                    no_point_hovered = false;
                }
            }
        }

        if no_point_hovered {
            for poly in app_ctx.polygons.iter_mut() {
                poly.deselect_all_points();
            }
            return Box::new(IdleState);
        }

        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            poly.deselect_all_points();
        }

        app_ctx.polygon_builder.start();
        return Box::new(AddPolygonState);
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            poly.deselect_all_points();
        }

        return Box::new(AddPointState);
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
                println!("self_crossing");
            } else {
                poly.assert_ccw();
            }
        }
        Box::new(SelectionState)
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
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
        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.start();
        Box::new(AddPolygonState)
    }

    fn on_add_point_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        Box::new(IdleState)
    }

    fn update(&mut self, dt: f32, mouse_pos: Vector2f, app_ctx: &mut AppContext) {}

    fn state_name(&self) -> &'static str {
        "Add Point State"
    }
}