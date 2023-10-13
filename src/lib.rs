use std::time::Instant;
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

const BACKGROUND_COLOR: sf::Color = sf::Color::rgb(37, 43, 72);

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
    polygon_builder: polygon::PolygonBuilder<'a>,
    polygons: Vec<polygon::PolygonObject<'a>>,
    drawing_mode: DrawingMode,
    egui_rect: egui::Rect,
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
            polygon_builder: polygon::PolygonBuilder::new(),
            polygons: Vec::new(),
            drawing_mode: DrawingMode::GPULines,
            egui_rect: egui::Rect::EVERYTHING,
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
                            self.handle_events(&ev);
                        }
                    },
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


    fn handle_events(&mut self, ev: &sf::Event) {
        let poly_opt = self.polygon_builder.update_input_or_build(ev);
        if let Some(poly) = poly_opt {
            self.polygons.push(poly);
        }
    }

    fn update(&mut self, dt: f32) {
        self.polygon_builder.update(dt, &self.window);
    }

    fn render(&mut self) {
        match self.drawing_mode {
            DrawingMode::GPULines => {
                for poly in &self.polygons {
                    poly.raw_polygon().draw_as_lines(&mut self.window);
                }

                match self.polygon_builder.raw_polygon() {
                    Some(&ref poly) => poly.draw_as_lines(&mut self.window),
                    None => (),
                }
            },
            DrawingMode::GPUThickLines => {
                for poly in &self.polygons {
                    poly.raw_polygon().draw_as_quads(&mut self.window);
                }

                match self.polygon_builder.raw_polygon() {
                    Some(&ref poly) => poly.draw_as_quads(&mut self.window),
                    None => (),
                }
            },
            DrawingMode::CPUBresenhamLines => () // TODO
        };

        for poly in &self.polygons {
            poly.raw_polygon().draw_points_circles(&mut self.window);
        }

        match self.polygon_builder.raw_polygon() {
            Some(&ref poly) => poly.draw_points_circles(&mut self.window),
            None => (),
        }

        self.polygon_builder.draw(&mut self.window);
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
                self.polygon_builder.start();
            }

            if !self.polygon_builder.is_active() {
                ui.add(
                    egui::Button::new("Cancel")
                        .fill(egui::Color32::from_rgb(36,36,36)),
                );
            } else {
                if ui.button("Cancel").clicked() {
                    self.polygon_builder.start();
                }
            }
        });
    }
}