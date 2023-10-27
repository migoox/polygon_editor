use std::io::WriterPanicked;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::{Mutex, Once, OnceLock, RwLock};
use sfml::graphics::Texture;
use sfml::LoadResult;
use super::sf;

pub const LINE_THICKNESS: f32 = 2.0;
pub const LINE_DETECTION_DISTANCE: f32 = 10.0;
pub const POINT_RADIUS: f32 = 5.0;
pub const LINES_COLOR: sf::Color = sf::Color::rgb(180, 180, 179);
pub const LINES_COLOR_INCORRECT: sf::Color = sf::Color::rgb(237, 123, 123);
pub const POLY_EDGE_MIN_LEN: f32 = 5.;
pub const POINTS_COLOR: sf::Color = sf::Color::rgb(247, 233, 135);
pub const POINT_DETECTION_RADIUS: f32 = 10.0;
pub const POINT_DETECTION_COLOR_CORRECT: sf::Color = sf::Color::rgb(100, 204, 197);
pub const POINT_DETECTION_COLOR_INCORRECT: sf::Color = sf::Color::rgb(237, 123, 123);
pub const POINT_SELECTED_COLOR: sf::Color = sf::Color::rgb(167, 187, 236);

pub const BACKGROUND_COLOR: sf::Color = sf::Color::rgb(37, 43, 72);


pub const CONSTRAINT_SPRITE_SIZE: sf::Vector2f = sf::Vector2f::new(32., 32.);

pub const WIN_SIZE_X: u32 = 1280;
pub const WIN_SIZE_Y: u32 = 720;

pub const MAX_OFFSET: f32 = 50.;

pub const OFFSET_COLOR: sf::Color = sf::Color::rgb(167, 187, 236);
