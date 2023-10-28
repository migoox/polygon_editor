use std::fs;
use std::time::Instant;
use egui_file::DialogType;

use egui_sfml::{
    egui,
    SfEgui,
};
use egui_sfml::egui::Widget;
use serde_json::{from_str, to_string};

use sfml::graphics::RenderTarget;
use crate::polygon::{PolygonObject, RawPolygonCoords};
use crate::state_machine::{IdleState, State};

use super::sf;
use super::polygon;
use super::style;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum DrawingMode {
    GPULines,
    CPUBresenhamLines,
}

pub struct AppContext<'a> {
    pub polygon_obj_factory: polygon::PolygonObjectFactory<'a>,
    pub polygon_objs: Vec<polygon::PolygonObject<'a>>,
}

pub struct Application<'a> {
    window: sf::RenderWindow,
    cpu_drawing_image: sf::Image,
    ui_scale: f32,

    // Option is required, since we are temporary taking ownership
    // of the State, each time the transition function is called.
    // In this application curr_state is always Some.
    curr_state: Option<Box<dyn State>>,
    app_ctx: AppContext<'a>,
    drawing_mode: DrawingMode,

    // Egui
    egui_rects: Vec<egui::Rect>,
    opened_file: Option<std::path::PathBuf>,
    file_dialog: Option<egui_file::FileDialog>,

    // Input
    a_pressed: bool,
    ctrl_pressed: bool,
    left_mouse_pressed: bool,
}

impl Application<'_> {
    pub fn new() -> Application<'static> {
        let mut window = sf::RenderWindow::new(
            (style::WIN_SIZE_X, style::WIN_SIZE_Y),
            "Polygon editor",
            sf::Style::CLOSE,
            &Default::default(),
        );

        window.set_vertical_sync_enabled(true);

        let mut result = Application {
            window,
            ui_scale: 0.8,
            cpu_drawing_image: sf::Image::new(style::WIN_SIZE_X, style::WIN_SIZE_Y),
            curr_state: Some(Box::new(IdleState)),
            app_ctx: AppContext {
                polygon_objs: Vec::new(),
                polygon_obj_factory: polygon::PolygonObjectFactory::new(),
            },
            drawing_mode: DrawingMode::GPULines,
            egui_rects: Vec::new(),
            a_pressed: false,
            ctrl_pressed: false,
            left_mouse_pressed: false,
            opened_file: None,
            file_dialog: None,
        };


        // let mut points: Vec<sf::Vector2f> = Vec::with_capacity(10);
        // points.push(sf::Vector2f::new(422., 131.));
        // points.push(sf::Vector2f::new(408., 640.));
        // points.push(sf::Vector2f::new(1008., 645.));
        // points.push(sf::Vector2f::new(979., 120.));
        // points.push(sf::Vector2f::new(740., 119.));
        // points.push(sf::Vector2f::new(741., 490.));
        // points.push(sf::Vector2f::new(509., 489.));
        // points.push(sf::Vector2f::new(510., 248.));
        // points.push(sf::Vector2f::new(678., 192.));
        // points.push(sf::Vector2f::new(563., 139.));
        //
        // result.app_ctx.polygons.push(PolygonObject::from(Polygon::create(points)));

        //
        // let mut points: Vec<sf::Vector2f> = Vec::with_capacity(10);
        // points.push(sf::Vector2f::new(722., 255.));
        // points.push(sf::Vector2f::new(801., 256.));
        // points.push(sf::Vector2f::new(797., 114.));
        // points.push(sf::Vector2f::new(438., 118.));
        // points.push(sf::Vector2f::new(446., 463.));
        // points.push(sf::Vector2f::new(893., 451.));
        // points.push(sf::Vector2f::new(887., 307.));
        // points.push(sf::Vector2f::new(661., 305.));
        // points.push(sf::Vector2f::new(652., 373.));
        // points.push(sf::Vector2f::new(503., 363.));
        // points.push(sf::Vector2f::new(516., 167.));
        // points.push(sf::Vector2f::new(726., 163.));
        //
        // result.app_ctx.polygons.push(PolygonObject::from(Polygon::create(points)));
        //
        // let mut points: Vec<sf::Vector2f> = Vec::with_capacity(10);
        // points.push(sf::Vector2f::new(347., 228.));
        // points.push(sf::Vector2f::new(825., 216.));
        // points.push(sf::Vector2f::new(816., 552.));
        // points.push(sf::Vector2f::new(974., 560.));
        // points.push(sf::Vector2f::new(962., 108.));
        // points.push(sf::Vector2f::new(204., 108.));
        // points.push(sf::Vector2f::new(187., 624.));
        // points.push(sf::Vector2f::new(597., 628.));
        // points.push(sf::Vector2f::new(595., 452.));
        // points.push(sf::Vector2f::new(505., 453.));
        // points.push(sf::Vector2f::new(508., 571.));
        // points.push(sf::Vector2f::new(349., 575.));
        // points.push(sf::Vector2f::new(351., 430.));
        // points.push(sf::Vector2f::new(746., 433.));
        // points.push(sf::Vector2f::new(749., 351.));
        // points.push(sf::Vector2f::new(348., 351.));
        // result.app_ctx.polygons.push(PolygonObject::from(Polygon::create(points)));
        result
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
                    sf::Event::MouseButtonPressed { button: _, x, y } => {
                        for rect_id in 0..self.egui_rects.len() {
                            if !self.egui_rects[rect_id].contains(egui::Pos2::new(x as f32, y as f32)) {
                                self.handle_input(&ev);
                            }
                        }
                    }
                    _ => self.handle_input(&ev),
                }
            }

            // Update
            self.update(Instant::now().duration_since(clock).as_secs_f32());
            clock = Instant::now();

            // Egui frame
            sfegui
                .do_frame(|ctx| {
                    self.set_egui_scale(&ctx, self.ui_scale);
                    self.render_egui(&ctx);
                })
                .unwrap();

            // Rendering
            self.window.clear(style::BACKGROUND_COLOR);
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

    fn save(&mut self) {
        if !self.opened_file.is_some() {
            return;
        }

        let raw_polygons: Vec<RawPolygonCoords> = self.app_ctx.polygon_objs
            .iter()
            .map(|pobj| pobj.get_raw())
            .collect();

        let json_string = to_string(&raw_polygons).unwrap();
        if let Err(err) = fs::write(self.opened_file.clone().unwrap().as_path(), json_string) {
            eprintln!("Error writing to file: {}", err);
        } else {
            println!("String successfully saved");
        }
    }

    fn load(&mut self) {
        if !self.opened_file.is_some() {
            return;
        }

        match fs::read_to_string(self.opened_file.clone().unwrap().as_path()) {
            Ok(contents) => {
                let raw_polygons: Vec<RawPolygonCoords> = from_str(&contents).unwrap();
                self.app_ctx.polygon_objs.clear();
                self.app_ctx.polygon_obj_factory.clear();

                for raw in raw_polygons {
                    self.app_ctx.polygon_objs.push(self.app_ctx.polygon_obj_factory.build_from_raw(raw));
                }
            }
            Err(err) => {
                eprintln!("Error reading from the file: {}", err);
                self.opened_file = None;
            }
        }
    }

    fn handle_input(&mut self, ev: &sf::Event) {
        match ev {
            sf::Event::KeyPressed { code: key, .. } => {
                match *key {
                    sfml::window::Key::LControl => self.ctrl_pressed = true,
                    sfml::window::Key::A => self.a_pressed = true,
                    _ => (),
                };
            }
            sf::Event::KeyReleased { code: key, .. } => {
                match *key {
                    sfml::window::Key::LControl => self.ctrl_pressed = false,
                    sfml::window::Key::A => self.a_pressed = false,
                    _ => (),
                };
            }
            sf::Event::MouseButtonPressed { button: btn, x, y } => {
                if *btn == sfml::window::mouse::Button::Left {
                    self.left_mouse_pressed = true;
                    if self.ctrl_pressed {
                        if self.a_pressed {
                            // CTRL + A + LM
                            self.curr_state = Some(self.curr_state.take().unwrap().on_ctrl_a_left_mouse_clicked(
                                sf::Vector2f::new(*x as f32, *y as f32),
                                &mut self.app_ctx,
                            ));
                            println!("Ctrl + A + LM clicked");
                        } else {
                            // CTRL + LM
                            self.curr_state = Some(self.curr_state.take().unwrap().on_ctrl_left_mouse_clicked(
                                sf::Vector2f::new(*x as f32, *y as f32),
                                &mut self.app_ctx,
                            ));
                            println!("Ctrl + LM clicked");
                        }
                    } else {
                        // LM
                        self.curr_state = Some(self.curr_state.take().unwrap().on_left_mouse_clicked(
                            sf::Vector2f::new(*x as f32, *y as f32),
                            &mut self.app_ctx,
                        ));
                        println!("LM clicked");
                    }
                }
            }
            sf::Event::MouseButtonReleased { button: btn, x, y } => {
                if *btn == sfml::window::mouse::Button::Left {
                    self.left_mouse_pressed = false;
                    self.curr_state = Some(self.curr_state.take().unwrap().on_left_mouse_released(
                        sf::Vector2f::new(self.window.mouse_position().x as f32, self.window.mouse_position().y as f32),
                        &mut self.app_ctx,
                    ));
                    println!("LM released");
                }
            }
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
                for poly in &self.app_ctx.polygon_objs {
                    poly.draw_edges(&mut self.window);
                    poly.draw_ctx(&mut self.window);
                }

                self.app_ctx.polygon_obj_factory.draw_edges(&mut self.window);
                self.app_ctx.polygon_obj_factory.draw_ctx(&mut self.window);
            }
            DrawingMode::CPUBresenhamLines => {
                // Clear the framebuffer
                for y in 0..style::WIN_SIZE_Y {
                    for x in 0..style::WIN_SIZE_X {
                        unsafe { self.cpu_drawing_image.set_pixel(x, y, style::BACKGROUND_COLOR); }
                    }
                }

                for poly in &self.app_ctx.polygon_objs {
                    poly.draw_bresenham_edges(&mut self.window, &mut self.cpu_drawing_image);
                }
                self.app_ctx.polygon_obj_factory.draw_bresenham_edges(&mut self.window, &mut self.cpu_drawing_image);

                // Draw the framebuffer
                let mut texture = sf::Texture::new();
                let _err = texture.as_mut().unwrap().load_from_image(
                    &self.cpu_drawing_image,
                    sf::IntRect::new(
                        0,
                        0,
                        style::WIN_SIZE_X as i32,
                        style::WIN_SIZE_Y as i32,
                    ),
                );

                let sprite = sf::Sprite::with_texture(texture.as_ref().unwrap());
                self.window.draw(&sprite);

                for poly in &self.app_ctx.polygon_objs {
                    poly.draw_ctx(&mut self.window);
                }
                self.app_ctx.polygon_obj_factory.draw_ctx(&mut self.window);
            }
        };
    }

    fn render_egui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("Top").show(&ctx, |ui| {
            ui.menu_button("File", |ui| {
                {
                    if egui::Button::new("Save").sense(egui::Sense {
                        click: self.opened_file.is_some(),
                        drag: self.opened_file.is_some(),
                        focusable: self.opened_file.is_some(),
                    }).ui(ui).clicked() {
                        self.save();
                    };

                    if ui.button("Save as...").clicked() {
                        let mut dialog = egui_file::FileDialog::save_file(self.opened_file.clone());
                        dialog.open();
                        self.file_dialog = Some(dialog);
                    }
                }
                ui.separator();
                {
                    if ui.button("Load...").clicked() {
                        let mut dialog = egui_file::FileDialog::open_file(self.opened_file.clone());
                        dialog.open();
                        self.file_dialog = Some(dialog);
                    }
                }
            });
        });
        // Handle dialog
        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                if dialog.path().is_some() {
                    self.opened_file = Some(dialog.path().unwrap().to_path_buf());
                    if dialog.dialog_type() == DialogType::OpenFile {
                        self.load();
                    } else if dialog.dialog_type() == DialogType::SaveFile {
                        self.save();
                    }
                }
            }
        }
        egui::Window::new("Options")
            .default_width(300.)
            .show(ctx, |ui| {
                ui.label("Polygons:");
                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        self.app_ctx.polygon_objs.retain_mut(|poly| {
                            let mut remove_flag = true;
                            egui::CollapsingHeader::new(poly.polygon().get_name())
                                .default_open(true)
                                .show(ui, |ui| {
                                    // Delete button
                                    if ui.button("Delete").clicked() {
                                        remove_flag = false;
                                    }

                                    // Polygon options
                                    poly.draw_egui(ui);
                                });
                            remove_flag
                        });
                    });


                ui.separator();
                // Pick the drawing method
                egui::ComboBox::from_label("Drawing method")
                    .selected_text(match self.drawing_mode {
                        DrawingMode::GPULines => "Lines [GPU]",
                        DrawingMode::CPUBresenhamLines => "Bresenham Lines [CPU]"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.drawing_mode, DrawingMode::GPULines, "Lines [GPU]");
                        ui.selectable_value(&mut self.drawing_mode, DrawingMode::CPUBresenhamLines, "Bresenham Lines [CPU]");
                    });

                ui.separator();

                let mut polygon_flag = false;
                let mut polygon_with_selected_points = 0;
                for (id, poly) in self.app_ctx.polygon_objs.iter().enumerate() {
                    if poly.selected_points_count() > 0 {
                        polygon_with_selected_points = id;
                        if polygon_flag {
                            polygon_flag = false;
                            break;
                        }
                        polygon_flag = true;
                    }
                }

                ui.label("Selected edge:");
                if polygon_flag {
                    if !self.app_ctx.polygon_objs[polygon_with_selected_points].draw_selected_edge_egui(ui) {
                        ui.label("None");
                    }
                } else {
                    ui.label("None");
                }

                ui.label("Selected polygon:");
                if polygon_flag {
                    self.app_ctx.polygon_objs[polygon_with_selected_points].draw_polygon_options_egui(ui);
                } else {
                    ui.label("None");
                }

                ui.separator();

                if ui.button("Add a polygon").clicked() {
                    self.curr_state = Some(self.curr_state.take().unwrap().on_add_btn(&mut self.app_ctx));
                }

                if ui.button("Edit points").clicked() {
                    self.curr_state = Some(self.curr_state.take().unwrap().on_edit_points_btn(&mut self.app_ctx));
                }

                ui.separator();

                ui.label(format!("State: {}", self.curr_state.as_ref().unwrap().state_name()));

                if ui.button("Cancel").clicked() {
                    self.curr_state = Some(self.curr_state.take().unwrap().on_cancel_btn(&mut self.app_ctx));
                }
            });

        self.egui_rects.clear();
        ctx.memory(|mem| {
            if let Some(rect) = mem.area_rect("Options") {
                self.egui_rects.push(rect);
            }
            if let Some(rect) = mem.area_rect("Top") {
                self.egui_rects.push(rect);
            }
        });
    }
}
