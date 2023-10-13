
use sfml::graphics::RenderTarget;
use super::sf;
use super::AppContext;
pub trait State {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn update(&self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext);
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

    fn update(&self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) {}
}

impl State for AddPolygonState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        let poly_opt = app_ctx.polygon_builder.add_or_build(mouse_pos);
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

    fn update(&self, dt: f32, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) {
        app_ctx.polygon_builder.update(dt, mouse_pos);
    }
}