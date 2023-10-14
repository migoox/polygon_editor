use std::any::Any;
use std::ops::Deref;
use std::time::Instant;
use egui_sfml::{
    egui,
    SfEgui,
};

use sfml::graphics::RenderTarget;
use crate::state_machine::{IdleState, State};

pub mod sf {
    pub use sfml::graphics::*;
    pub use sfml::system::*;
    pub use sfml::window::*;
}

pub mod polygon;
pub mod state_machine;

const BACKGROUND_COLOR: sf::Color = sf::Color::rgb(37, 43, 72);

#[derive(Debug)]
#[derive(PartialEq)]
pub enum DrawingMode {
    GPULines,
    GPUThickLines,
    CPUBresenhamLines,
}

pub struct AppContext<'a> {
    polygon_builder: polygon::PolygonBuilder<'a>,
    polygons: Vec<polygon::PolygonObject<'a>>,
}

pub struct Application<'a> {
    window: sf::RenderWindow,
    program_scale: f32,

    // Option is required, since we are temporary taking ownership
    // of the State, each time the transition function is called.
    // In this application curr_state is always Some.
    curr_state: Option<Box<dyn State>>,
    app_ctx: AppContext<'a>,
    drawing_mode: DrawingMode,
    egui_rect: egui::Rect,


    // Input
    ctrl_pressed: bool,
    left_mouse_pressed: bool,
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
            curr_state: Some(Box::new(IdleState)),
            app_ctx: AppContext {
                polygons: Vec::new(),
                polygon_builder: polygon::PolygonBuilder::new(),
            },
            drawing_mode: DrawingMode::GPULines,
            egui_rect: egui::Rect::EVERYTHING,
            ctrl_pressed: false,
            left_mouse_pressed: false,
        }
    }

    pub fn run(&mut self) {
        let mut sfegui = SfEgui::new(&self.window);
        let mut clock = Instant::now();

        while self.window.is_open() {
            while let Some(ev) = self.window.poll_event() {
                // Feed egui with the input detected by the sfml
                sfegui.add_event(&ev);

                // Close the program
                if ev == sf::Event::Closed {
                    self.window.close()
                }

                // If mouse has been clicked do not react when it's inside of the egui window bounds
                match ev {
                    sf::Event::MouseButtonPressed { button: _, x, y } =>  {
                        if !self.egui_rect.contains(egui::Pos2::new(x as f32, y as f32)) {
                            self.handle_input(&ev);
                        }
                    },
                    _ => self.handle_input(&ev),
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
            self.window.clear(BACKGROUND_COLOR);
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

    fn handle_input(&mut self, ev: &sf::Event) {
        match ev {
            sf::Event::KeyPressed { code: key, .. } => {
                if *key == sfml::window::Key::LControl {
                    self.ctrl_pressed = true;
                }
            },
            sf::Event::KeyReleased { code: key, .. } => {
                if *key == sfml::window::Key::LControl {
                    self.ctrl_pressed = false;
                }
            },
            sf::Event::MouseButtonPressed { button: btn, x, y } => {
                if *btn == sfml::window::mouse::Button::Left {
                    self.left_mouse_pressed = true;
                    if self.ctrl_pressed {
                        // CTRL + LM
                        self.curr_state = Some(self.curr_state.take().unwrap().on_ctrl_left_mouse_clicked(
                            sf::Vector2f::new(*x as f32, *y as f32),
                            &mut self.app_ctx
                        ));
                        println!("Ctrl + LM clicked");
                    } else {
                        // LM
                        self.curr_state = Some(self.curr_state.take().unwrap().on_left_mouse_clicked(
                            sf::Vector2f::new(*x as f32, *y as f32),
                            &mut self.app_ctx
                        ));
                        println!("LM clicked");
                    }
                }
            },
            sf::Event::MouseButtonReleased { button: btn, x, y } => {
                if *btn == sfml::window::mouse::Button::Left {
                    self.left_mouse_pressed = false;
                    self.curr_state = Some(self.curr_state.take().unwrap().on_left_mouse_released(
                        sf::Vector2f::new(self.window.mouse_position().x as f32, self.window.mouse_position().y as f32),
                        &mut self.app_ctx
                    ));
                    println!("LM released");
                }

            },
            _ => (),
        }
    }

    fn update(&mut self, dt: f32) {
        self.curr_state.as_mut().unwrap().update(
            dt,
            sf::Vector2f::new(
                self.window.mouse_position().x as f32,
                self.window.mouse_position().y as f32,
            ),
            &mut self.app_ctx,
        );
    }

    fn render(&mut self) {
        // Draw edges of the polygons
        match self.drawing_mode {
            DrawingMode::GPULines => {
                for poly in &self.app_ctx.polygons {
                    poly.raw_polygon().draw_as_lines(&mut self.window);
                }

                match self.app_ctx.polygon_builder.raw_polygon() {
                    Some(&ref poly) => poly.draw_as_lines(&mut self.window),
                    None => (),
                }
            },
            DrawingMode::GPUThickLines => {
                for poly in &self.app_ctx.polygons {
                    poly.raw_polygon().draw_as_quads(&mut self.window);
                }

                match self.app_ctx.polygon_builder.raw_polygon() {
                    Some(&ref poly) => poly.draw_as_quads(&mut self.window),
                    None => (),
                }
            },
            DrawingMode::CPUBresenhamLines => () // TODO
        };

        // Draw points of the polygons
        for poly in &self.app_ctx.polygons {
            poly.raw_polygon().draw_points_circles(&mut self.window);
        }

        match self.app_ctx.polygon_builder.raw_polygon() {
            Some(&ref poly) => poly.draw_points_circles(&mut self.window),
            None => (),
        }

        // Draw ui elements
        self.app_ctx.polygon_builder.draw(&mut self.window);
        for poly in &self.app_ctx.polygons {
            poly.draw(&mut self.window);
        }
    }

    fn render_egui(&mut self, ctx: &egui::Context) {
        egui::Window::new("Options").show(ctx, |ui| {
            self.egui_rect = ctx.used_rect();

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

            if ui.button("Add a polygon").clicked() {
                self.curr_state = Some(self.curr_state.take().unwrap().on_add_btn(&mut self.app_ctx));
            }

            if ui.button("Cancel").clicked() {
                self.curr_state = Some(self.curr_state.take().unwrap().on_cancel_btn(&mut self.app_ctx));
            }

            ui.label(format!("State: {}",  self.curr_state.as_ref().unwrap().state_name()));
        });
    }
}
