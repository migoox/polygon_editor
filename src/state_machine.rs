
use sfml::graphics::RenderTarget;
use super::sf;
use super::AppContext;
pub trait State {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;

    fn render(self: Box<Self>, target: &dyn sf::RenderTarget);
}


pub struct IdleState;
pub struct AddPolygonState;

impl State for IdleState {
    fn on_left_mouse_clicked(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn on_add_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
       self
    }

    fn on_cancel_btn(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn render(self: Box<Self>, target: &dyn RenderTarget) {}
}