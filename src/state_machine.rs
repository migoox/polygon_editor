use sfml::graphics::RenderTarget;
use sfml::system::Vector2f;
use super::sf;
use super::AppContext;
pub trait State {
    fn left_mouse_click(self: Box<Self>, mouse_pos: sf::Vector2f, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn add_btn_click(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;
    fn cancel_btn_click(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State>;

    fn render(self: Box<Self>, target: &dyn sf::RenderTarget);
}


pub struct IdleState;
pub struct AddPolygonState;

impl State for IdleState {
    fn left_mouse_click(self: Box<Self>, mouse_pos: Vector2f, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn add_btn_click(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        todo!()
    }

    fn cancel_btn_click(self: Box<Self>, app_ctx: &mut AppContext) -> Box<dyn State> {
        self
    }

    fn render(self: Box<Self>, target: &dyn RenderTarget) {}
}