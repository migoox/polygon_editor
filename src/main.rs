use egui_sfml::{
    egui::{self, color_picker::color_picker_color32},
    SfEgui,
};
use rand::Rng;
use sfml::{graphics::*, system::*, window::*};
fn set_egui_scale(ctx: &egui::Context, scale: f32) {
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

fn main() {
    // TODO: config file
    let program_scale: f32 = 1.0;

    let mut render_image = Image::new(800, 600);

    unsafe {
        for i in 0..100 {
            render_image.set_pixel(i, i, Color::rgb(255, 0, 0));
        }
    }

    let mut render_texture = Texture::new().unwrap();
    render_texture.load_from_image(
        &render_image,
        IntRect { left: 0, top: 0, width: 800, height: 600 },
    ).unwrap();

    let mut canvas = Sprite::new();
    canvas.set_texture(&render_texture, false);

    // Initialize Rng
    let mut rng = rand::thread_rng();

    // Create a render window
    let mut window = RenderWindow::new((800, 600), "Test app", Style::CLOSE, &Default::default());
    window.set_vertical_sync_enabled(true);

    // Create a circle to be drawn
    let mut circle = CircleShape::new(60., 30);
    circle.set_fill_color(Color::BLUE);
    circle.set_scale(Vector2f {
        x: program_scale,
        y: program_scale,
    });

    // Create the sfegui instance
    let mut sfegui = SfEgui::new(&window);

    // Color picker
    let mut rgb: [f32; 3] = [0.0, 0.0, 1.0];

    while window.is_open() {
        // Events
        while let Some(ev) = window.poll_event() {
            // Feed egui with the input here
            sfegui.add_event(&ev);
            match ev {
                Event::Closed => window.close(),
                _ => {}
            }
        }

        // Egui frame
        sfegui
            .do_frame(|ctx| {
                set_egui_scale(&ctx, program_scale);
                // Create an egui window
                egui::Window::new("Hello").show(ctx, |ui| {
                    // Add a button to the window
                    if ui.button("Random color").clicked() {
                        rgb[0] = rng.gen_range(0.0..1.0);
                        rgb[1] = rng.gen_range(0.0..1.0);
                        rgb[2] = rng.gen_range(0.0..1.0);
                        circle.set_fill_color(Color::rgb(
                            (rgb[0] * 255.0) as u8,
                            (rgb[1] * 255.0) as u8,
                            (rgb[2] * 255.0) as u8,
                        ));
                    }

                    // Add a color picker to the window
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        circle.set_fill_color(Color::rgb(
                            (rgb[0] * 255.0) as u8,
                            (rgb[1] * 255.0) as u8,
                            (rgb[2] * 255.0) as u8,
                        ));
                    }
                });
            })
            .unwrap();

        // Render
        window.clear(Color::BLACK);

        // window.draw(&circle);
        window.draw(&canvas);

        sfegui.draw(&mut window, None);
        window.display();
    }
}
