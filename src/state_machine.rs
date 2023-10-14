use sfml::graphics::RenderTarget;
use sfml::system::Vector2f;
use super::sf;
use super::AppContext;
pub trait State {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_left_mouse_released(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn update(&self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext);
    fn state_name(&self) -> &'static str;
}

pub struct IdleState;
pub struct AddPolygonState;
pub struct SelectionState;
pub struct DraggingState;

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

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>{
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.cancel();
        Box::new(IdleState)
    }

    fn update(&self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) {
        app_ctx.polygon_builder.update(dt, mouse_pos);
    }

    fn state_name(&self) -> &'static str {
        "Add Polygon State"
    }
}

impl State for IdleState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_point_hovered() {
                let err = poly.select_point(poly.get_hovered_point_id());

                return Box::new(DraggingState);
            }
        }

        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>{
       for poly in app_ctx.polygons.iter_mut() {
            if poly.is_point_hovered() {
                let err = poly.select_point(poly.get_hovered_point_id());

                if err.is_ok() {
                    return Box::new(SelectionState);
                }
            }
        }

        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.start();
        Box::new(AddPolygonState)
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn update(&self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) {
        for poly in app_ctx.polygons.iter_mut() {
            poly.update_on_point_hover(mouse_pos);
        }
    }

    fn state_name(&self) -> &'static str {
        "Idle State"
    }
}

impl State for SelectionState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            if poly.is_point_hovered() {
                if let Ok(is_selected) = poly.is_point_selected(poly.get_hovered_point_id()) {
                    if !is_selected {
                        poly.deselect_all_points();
                        let _err = poly.select_point(poly.get_hovered_point_id());
                    }
                    return Box::new(DraggingState);
                }
            } else {
                poly.deselect_all_points();
                return Box::new(IdleState);
            }
        }

        self
    }

    fn on_left_mouse_released(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
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
                }
            } else {
                poly.deselect_all_points();
                return Box::new(IdleState);
            }
        }

        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        for poly in app_ctx.polygons.iter_mut() {
            poly.deselect_all_points();
        }

        return Box::new(AddPolygonState);
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn update(&self, dt: f32, mouse_pos: Vector2f, app_ctx: &mut AppContext) {
        for poly in app_ctx.polygons.iter_mut() {
            poly.update_on_point_hover(mouse_pos);
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
        Box::new(SelectionState)
    }

    fn on_ctrl_left_mouse_clicked(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn update(&self, dt: f32, mouse_pos: Vector2f, app_ctx: &mut AppContext) {
    }

    fn state_name(&self) -> &'static str {
        "Dragging State"
    }
}