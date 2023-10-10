use std::time::{Duration, Instant};
use egui_sfml::{
    egui,
    SfEgui,
};

use sfml::graphics::RenderTarget;

pub mod sf {
    pub use sfml::graphics::*;
    pub use sfml::system::*;
    pub use sfml::window::*;
}

pub mod polygon;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum DrawingMode {
    GPULines,
    GPUThickLines,
    CPUBresenhamLines,
}

pub struct Application<'a> {
    window: sf::RenderWindow,
    program_scale: f32,
    test: polygon::Polygon<'a>,
    test2: polygon::PolygonBuilder<'a>,
    drawing_mode: DrawingMode,
}

impl Application<'_> {
    pub fn new() -> Application<'static> {
        let mut window = sf::RenderWindow::new(
            (800, 600),
            "Polygon editor",
            sf::Style::CLOSE,
            &Default::default()
        );

        window.set_vertical_sync_enabled(true);

        let program_scale = 1.0;

        Application {
            window,
            program_scale,
            test: polygon::Polygon::create(vec![
                sf::Vector2f::new(100., 100.),
                sf::Vector2f::new(100., 50.),
                sf::Vector2f::new(0., 0.),
            ]),
            test2: polygon::PolygonBuilder::new(),
            drawing_mode: DrawingMode::GPULines,
        }
    }

    pub fn run(&mut self) {
        let mut sfegui = SfEgui::new(&self.window);
        let mut clock = Instant::now();

        while self.window.is_open() {
            while let Some(ev) = self.window.poll_event() {
                // Feed egui with the input detected by the sfml
                sfegui.add_event(&ev);

                // Handle events
                match ev {
                    sf::Event::Closed => self.window.close(),
                    _ => self.handle_events(&ev),
                }
            }

            // Update
            self.update(Instant::now().duration_since(clock).as_secs_f32());
            clock = Instant::now();

            // Egui frame
            sfegui
                .do_frame(|ctx| {
                    self.set_egui_scale(&ctx, self.program_scale);
                    self.render_egui(&ctx);
                })
                .unwrap();

            // Rendering
            self.window.clear(sf::Color::rgb(0, 0, 0));
            self.render();
            sfegui.draw(&mut self.window, None);
            self.window.display();
        }
    }
    fn set_egui_scale(&self, ctx: &egui::Context, scale: f32) {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                egui::style::TextStyle::Heading,
                egui::FontId::new(scale * 30.0, egui::FontFamily::Proportional),
            ),
            (
                egui::style::TextStyle::Body,
                egui::FontId::new(scale * 22.0, egui::FontFamily::Proportional),
            ),
            (
                egui::style::TextStyle::Monospace,
                egui::FontId::new(scale * 18.0, egui::FontFamily::Proportional),
            ),
            (
                egui::style::TextStyle::Button,
                egui::FontId::new(scale * 18.0, egui::FontFamily::Proportional),
            ),
            (
                egui::style::TextStyle::Small,
                egui::FontId::new(scale * 14.0, egui::FontFamily::Proportional),
            ),
        ]
            .into();
        ctx.set_style(style);
    }

    fn handle_events(&mut self, ev: &sf::Event) {
        self.test2.update_input(&ev);
    }

    fn update(&mut self, dt: f32) {
        self.test2.update(dt);
    }

    fn render(&mut self) {
        match self.drawing_mode {
            DrawingMode::GPULines => {
                self.test.draw_as_lines(&mut self.window);

                match self.test2.raw_polygon() {
                    Some(&ref poly) => poly.draw_as_lines(&mut self.window),
                    None => (),
                }
            },
            DrawingMode::GPUThickLines => {
                self.test.draw_as_quads(&mut self.window);

                match self.test2.raw_polygon() {
                    Some(&ref poly) => poly.draw_as_quads(&mut self.window),
                    None => (),
                }
            },
            DrawingMode::CPUBresenhamLines => () // TODO
        };

        self.test.draw_points_circles(&mut self.window);

        match self.test2.raw_polygon() {
            Some(&ref poly) => poly.draw_points_circles(&mut self.window),
            None => (),
        }
    }

    fn render_egui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Options").show(ctx, |ui| {
            // Pick the drawing method
            egui::ComboBox::from_label("Drawing method")
                .selected_text(match self.drawing_mode {
                   DrawingMode::GPULines => "Lines [GPU]",
                    DrawingMode::GPUThickLines => "Thick Lines [GPU]",
                    DrawingMode::CPUBresenhamLines => "Bresenham Lines [CPU]"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.drawing_mode, DrawingMode::GPULines, "Lines [GPU]");
                    ui.selectable_value(&mut self.drawing_mode, DrawingMode::GPUThickLines, "Thick Lines [GPU]");
                    ui.selectable_value(&mut self.drawing_mode, DrawingMode::CPUBresenhamLines, "Bresenham Lines [CPU]");
                });
        });
    }
}