use std::mem;
use crate::my_math::circle_vs_plane_frac;
use super::sf;

#[derive(Clone, PartialEq, Debug)]
pub enum LinePainterAlgorithm {
    MidPointLine,
    SymmetricMidPointLine,
    GuptaDoubleStepMidPointLine,
    WULine,
}

pub struct LinePainter {
    color: sf::Color,
    thickness: f32,
    alg: LinePainterAlgorithm,
}

impl LinePainter {
    pub fn new(color: sf::Color, thickness: f32) -> LinePainter {
        LinePainter {
            color,
            thickness,
            alg: LinePainterAlgorithm::MidPointLine,
        }
    }
    pub fn set_thickness(&mut self, thickness: f32) {
        self.thickness = thickness;
    }
    pub fn thickness(&self) -> f32 {
        self.thickness
    }
    pub fn set_alg(&mut self, alg: LinePainterAlgorithm) {
        self.alg = alg;
    }
    pub fn alg(&self) -> LinePainterAlgorithm {
        self.alg.clone()
    }

    fn put_pixel(&self, x: i32, y: i32, img_target: &mut sf::Image) {
        if x < img_target.size().x as i32 && x >= 0 &&
            y < img_target.size().y as i32 && y >= 0 {
            unsafe { img_target.set_pixel(x as u32, y as u32, self.color) }
        }
    }

    fn intensify_pixel_with_circle_vs_half_plain_frac(&self, x: i32, y: i32, thickness: f32, distance: f32, img_target: &mut sf::Image) -> bool {
        if !(x < img_target.size().x as i32 && x >= 0 &&
            y < img_target.size().y as i32 && y >= 0) {
            return false;
        }

        // Find an alpha
        let mut alpha = 0.0;
        let w = thickness / 2.;
        let r = 0.5;
        if w >= 1. {
            if w <= distance {
                alpha = circle_vs_plane_frac(distance - w, r);
            } else if 0. <= distance && distance <= w {
                alpha = 1. - circle_vs_plane_frac(w - distance, r);
            }
        } else {
            if 0. <= distance && distance <= w {
                alpha = 1. - circle_vs_plane_frac(w - distance, r) - circle_vs_plane_frac(w + distance, r);
            } else if w <= distance && distance <= r - w {
                alpha = circle_vs_plane_frac(distance - w, r) - circle_vs_plane_frac(distance + w, r);
            } else {
                alpha = circle_vs_plane_frac(distance - w, r);
            }
        }


        unsafe {
            let color = img_target.pixel_at(x as u32, y as u32);
            let premultiplied = sf::Color::rgb(
                ((self.color.r as f32) * alpha) as u8,
                ((self.color.g as f32) * alpha) as u8,
                ((self.color.b as f32) * alpha) as u8,
            );

            if premultiplied.r == 0 && premultiplied.g == 0 && premultiplied.b == 0 {
                return false;
            }

            let new_color = premultiplied + sf::Color::rgb(
                ((color.r as f32) * (1. - alpha)) as u8,
                ((color.g as f32) * (1. - alpha)) as u8,
                ((color.b as f32) * (1. - alpha)) as u8,
            );


            img_target.set_pixel(x as u32, y as u32, new_color);
        }
        return true;
    }

    fn intensify_pixel(&self, x: i32, y: i32, intensity: f32, img_target: &mut sf::Image) {
        if !(x < img_target.size().x as i32 && x >= 0 &&
            y < img_target.size().y as i32 && y >= 0) {
            return;
        }
        unsafe {
            let color = img_target.pixel_at(x as u32, y as u32);
            let premultiplied = sf::Color::rgb(
                ((self.color.r as f32) * intensity) as u8,
                ((self.color.g as f32) * intensity) as u8,
                ((self.color.b as f32) * intensity) as u8,
            );

            let new_color = premultiplied + sf::Color::rgb(
                ((color.r as f32) * (1. - intensity)) as u8,
                ((color.g as f32) * (1. - intensity)) as u8,
                ((color.b as f32) * (1. - intensity)) as u8,
            );

            img_target.set_pixel(x as u32, y as u32, new_color);
        }
    }

    pub fn draw_line(&mut self, mut p0: sf::Vector2f, mut p1: sf::Vector2f, color: sf::Color, img_target: &mut sf::Image) {
        self.color = color;
        if self.alg == LinePainterAlgorithm::WULine {
            self.xiaolin_wu_antialiased_line(p0, p1, |x, y, i| self.intensify_pixel(x, y, i, img_target));
            return;
        }
        let mut p0 = sf::Vector2i::new(p0.x as i32, p0.y as i32);
        let mut p1 = sf::Vector2i::new(p1.x as i32, p1.y as i32);

        // This simplification skips 4 cases out of 8
        if p1.x < p0.x {
            std::mem::swap(&mut p0, &mut p1);
        }

        let mut d = p1 - p0;

        if d.y <= 0 {
            if d.x.abs() >= d.y.abs() {
                self.run_bresenham_alg18(p0.x, p0.y, p1.x, p1.y, d.x, -d.y, 1, -1, false, img_target);
            } else {
                self.run_bresenham_alg18(p0.y, p0.x, p1.y, p1.x, -d.y, d.x, -1, 1, true, img_target);
            }
        } else {
            if d.x.abs() >= d.y.abs() {
                self.run_bresenham_alg18(p0.x, p0.y, p1.x, p1.y, d.x, d.y, 1, 1, false, img_target);
            } else {
                self.run_bresenham_alg18(p0.y, p0.x, p1.y, p1.x, d.y, d.x, 1, 1, true, img_target);
            }
        }
    }
    fn run_bresenham_alg18(&self,
                           x0: i32, y0: i32,
                           x1: i32, y1: i32,
                           dx: i32, dy: i32,
                           incr_x: i32, incr_y: i32,
                           rev_func_input: bool,
                           img_target: &mut sf::Image)
    {
        if rev_func_input {
            match self.alg {
                LinePainterAlgorithm::MidPointLine => self.mid_point_line18(x0, y0, x1, y1, dx, dy, incr_x, incr_y, |x, y| self.put_pixel(y, x, img_target)),
                LinePainterAlgorithm::SymmetricMidPointLine => self.symmetric_mid_point_line18(x0, y0, x1, y1, dx, dy, incr_x, incr_y, |x, y| self.put_pixel(y, x, img_target)),
                LinePainterAlgorithm::GuptaDoubleStepMidPointLine => self.gupta_sproull_antialiased_thick_line18(x0, y0, x1, y1, dx, dy, incr_x, incr_y, |x, y, d| self.intensify_pixel_with_circle_vs_half_plain_frac(y, x, self.thickness, d, img_target)),
                _ => ()
            }
            return;
        }
        match self.alg {
            LinePainterAlgorithm::MidPointLine => self.mid_point_line18(x0, y0, x1, y1, dx, dy, incr_x, incr_y, |x, y| self.put_pixel(x, y, img_target)),
            LinePainterAlgorithm::SymmetricMidPointLine => self.symmetric_mid_point_line18(x0, y0, x1, y1, dx, dy, incr_x, incr_y, |x, y| self.put_pixel(x, y, img_target)),
            LinePainterAlgorithm::GuptaDoubleStepMidPointLine => self.gupta_sproull_antialiased_thick_line18(x0, y0, x1, y1, dx, dy, incr_x, incr_y, |x, y, d| self.intensify_pixel_with_circle_vs_half_plain_frac(x, y, self.thickness, d, img_target)),
            _ => ()
        }
    }

    // Works only for 1/8 quarter
    fn mid_point_line18<F>(&self,
                           mut x0: i32, mut y0: i32,
                           x1: i32, _y1: i32,
                           dx: i32, dy: i32,
                           incr_x: i32, incr_y: i32,
                           mut put_pixel_func: F,
    ) where
        F: FnMut(i32, i32),
    {
        let mut d = 2 * dy - dx;
        let incrd_e = 2 * dy;
        let incrd_ne = 2 * dy - 2 * dx;

        let mut distance = (x1 - x0).abs();

        while distance.abs() > 0 {
            for i in 0..(self.thickness as i32) {
                put_pixel_func(x0, y0 + i);
                put_pixel_func(x0, y0 - i)
            }

            if d < 0 {
                d += incrd_e;
            } else {
                d += incrd_ne;
                y0 += incr_y;
            }
            x0 += incr_x;
            distance -= 1;
        }
    }

    // Works only for 1/8 quarter
    fn symmetric_mid_point_line18<F>(&self,
                                     mut x0: i32, mut y0: i32,
                                     mut x1: i32, mut y1: i32,
                                     dx: i32, dy: i32,
                                     incr_x: i32, incr_y: i32,
                                     mut put_pixel_func: F,
    ) where
        F: FnMut(i32, i32),
    {
        let mut d = 2 * dy - dx;
        let incrd_e = 2 * dy;
        let incrd_ne = 2 * dy - 2 * dx;

        let mut distance = (x1 - x0).abs();

        while distance > 0 {
            for i in 0..(self.thickness as i32) {
                put_pixel_func(x0, y0 + i);
                put_pixel_func(x0, y0 - i)
            }
            if d < 0 {
                d += incrd_e;
            } else {
                d += incrd_ne;
                y0 += incr_y;
                y1 -= incr_y;
            }
            x0 += incr_x;
            x1 -= incr_x;
            distance -= 2;
        }
        for i in 0..(self.thickness as i32) {
            put_pixel_func(x0, y0 + i);
            put_pixel_func(x0, y0 - i)
        }
    }

    // TODO: fix this function
    fn symmetric_double_step_mid_point_line18<F>(&self,
                                                 mut x0: i32, mut y0: i32,
                                                 mut x1: i32, mut y1: i32,
                                                 dx: i32, dy: i32,
                                                 incr_x: i32, incr_y: i32,
                                                 mut put_pixel_func: F,
    ) where
        F: FnMut(i32, i32),
    {
        let xend = (dx - 1) / 4;
        let pix_left = (dx - 1) % 4;
        let incr1 = 4 * dy;
        let incr2 = 4 * dy - 2 * dx;
        let mut d = 4 * dy - dx;

        put_pixel_func(x0, y0);
        put_pixel_func(x1, y1);
        for _i in 0..(xend - 1) {
            x0 += incr_x;
            x1 -= incr_x;
            if d < 0 {
                put_pixel_func(x0 + 1, y0);
                put_pixel_func(x0 + 2, y0);
                put_pixel_func(x1 - 1, y1);
                put_pixel_func(x1 - 2, y1);
                d += incr1;
            } else if (d - 2 * dy) < 0 {
                put_pixel_func(x0 + 1, y0);
                put_pixel_func(x0 + 2, y0 + 1);
                put_pixel_func(x1 - 1, y1);
                put_pixel_func(x1 - 2, y1 - 1);
                y0 += incr_y;
                y1 -= incr_y;
                d += incr2;
            } else {
                put_pixel_func(x0 + 1, y0 + 1);
                put_pixel_func(x0 + 2, y0 + 2);
                put_pixel_func(x1 - 1, y1 - 1);
                put_pixel_func(x1 - 2, y1 - 2);
                y0 += 2 * incr_y;
                y1 -= 2 * incr_y;
                d += incr2;
            }
        }
        x0 += 2 * incr_x;
        x1 -= 2 * incr_y;
    }

    fn gupta_sproull_antialiased_thick_line18<F>(&self,
                                                 mut x0: i32, mut y0: i32,
                                                 mut x1: i32, mut y1: i32,
                                                 dx: i32, dy: i32,
                                                 incr_x: i32, incr_y: i32,
                                                 mut intensify_pixel_func: F,
    ) where
        F: FnMut(i32, i32, f32) -> bool,
    {
        // Bresenham
        let mut d = 2 * dy - dx;
        let incrd_e = 2 * dy;
        let incrd_ne = 2 * dy - 2 * dx;

        // Antialiasing
        let mut two_v_dx = 0;
        let inv_denom: f32 = 1. / (2. * ((dx * dx + dy * dy) as f32).sqrt());
        let two_dx_inv_denom = 2. * (dx as f32) * inv_denom;

        let mut distance = (x1 - x0).abs();

        while distance.abs() > 0 {
            let mut i = 0;
            loop {
                let valid = intensify_pixel_func(
                    x0,
                    y0 + i,
                    (i as f32) * two_dx_inv_denom - (incr_y as f32) * (two_v_dx as f32) * inv_denom,
                );
                if !valid && i > 0 { break; }
                i += 1;
            }

            i = 0;
            loop {
                let valid = intensify_pixel_func(
                    x0,
                    y0 - i,
                    (i as f32) * two_dx_inv_denom + (incr_y as f32) * (two_v_dx as f32) * inv_denom,
                );
                if !valid && i > 0 { break; }
                i += 1;
            }

            if d < 0 {
                two_v_dx = d + dx;
                d += incrd_e;
            } else {
                two_v_dx = d - dx;
                d += incrd_ne;
                y0 += incr_y;
            }
            x0 += incr_x;
            distance -= 1;
        }
    }

    fn xiaolin_wu_antialiased_line<F>(&self,
                                      mut p0: sf::Vector2f,
                                      mut p1: sf::Vector2f,
                                      mut intensify_pixel_func: F,
    ) where
        F: FnMut(i32, i32, f32),
    {
        // Check steepness
        let mut steep = false;
        if (p0.y - p1.y).abs() >= (p0.x - p1.x).abs() {
            mem::swap(&mut p0.x, &mut p0.y);
            mem::swap(&mut p1.x, &mut p1.y);
            steep = true;
        }

        // Skip 4 cases
        if p0.x > p1.x {
            mem::swap(&mut p0, &mut p1);
        }

        // Find the slope
        let delta = p1 - p0;
        let mut m: f32 = 0.0;
        if delta.x != 0.0 {
            m = delta.y / delta.x;
        }

        let mut y: f32 = p0.y;
        if steep {
            for x in (p0.x as i32)..(p1.x as i32) {
                intensify_pixel_func(y as i32, x, (1. - y.fract()));
                intensify_pixel_func(y as i32 + 1, x, (y.fract()));
                y += m;
            }
        } else {
            for x in (p0.x as i32)..(p1.x as i32) {
                intensify_pixel_func(x, y as i32, (1. - y.fract()));
                intensify_pixel_func(x, y as i32 + 1, (y.fract()));
                y += m;
            }
        }
    }
}