use sfml::system::Vector2f;
use super::sf;

pub fn distance(point1: &sf::Vector2f, point2: &sf::Vector2f) -> f32 {
    let dx = point1.x - point2.x;
    let dy = point1.y - point2.y;
    (dx * dx + dy * dy).sqrt()
}

pub fn distance2(point1: &sf::Vector2f, point2: &sf::Vector2f) -> f32 {
    let dx = point1.x - point2.x;
    let dy = point1.y - point2.y;
    (dx * dx + dy * dy)
}

pub fn is_right_turn(p0: &sf::Vector2f, p1: &sf::Vector2f, p2: &sf::Vector2f) -> bool {
    let v0 = sf::Vector2f::new(p1.x - p0.x, p1.y - p0.y);
    let v1 = sf::Vector2f::new(p2.x - p1.x, p2.y - p1.y);
    cross2(&v0, &v1) < 0.
}

pub fn vec_len(vec: &sf::Vector2f) -> f32 {
    (vec.x * vec.x + vec.y * vec.y).sqrt()
}

pub fn vec_len2(vec: &sf::Vector2f) -> f32 {
    (vec.x * vec.x + vec.y * vec.y)
}

pub fn vec_norm(vec: &sf::Vector2f) -> sf::Vector2f {
    *vec / vec_len(vec)
}

pub fn dot_prod(vec1: &sf::Vector2f, vec2: &sf::Vector2f) -> f32 {
    vec1.x * vec2.x + vec1.y * vec2.y
}

pub fn cross2(vec1: &sf::Vector2f, vec2: &sf::Vector2f) -> f32 {
    return (vec1.x * vec2.y) - (vec1.y * vec2.x);
}