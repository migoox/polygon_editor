
use sfml::graphics::RenderTarget;
use super::sf;
use super::AppContext;
pub trait State {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;

    fn render(&self, target: &dyn sf::RenderTarget);
}


pub struct IdleState;
pub struct AddPolygonState;

impl State for IdleState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.start();
        Box::new(AddPolygonState)
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn render(&self, target: &dyn RenderTarget) {}
}

impl State for AddPolygonState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let poly_opt = app_ctx.polygon_builder.update_input_or_build(mouse_pos);
        if let Some(poly) = poly_opt {
            app_ctx.polygons.push(poly);
            return Box::new(IdleState);
        }
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        app_ctx.polygon_builder.cancel();
        Box::new(IdleState)
    }

    fn render(&self, target: &dyn RenderTarget) {}
}