    use std::time::{Duration, Instant};
    use egui_sfml::{
        egui,
        SfEgui,
    };

    use sfml::graphics::RenderTarget;
    mod sf {
        pub use sfml::graphics::*;
        pub use sfml::system::*;
        pub use sfml::window::*;
    }

    pub struct Application {
        window: sf::RenderWindow,
        program_scale: f32,
    }

    impl Application {
        pub fn new() -> Application {
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
                program_scale
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

        fn handle_events(&mut self, ev: &sf::Event) {}

        fn update(&mut self, dt: f32) {}

        fn render(&mut self) {}

        fn render_egui(&mut self, ctx: &egui::Context) {
            egui::Window::new("Hello").show(ctx, |ui| {});
        }
    }