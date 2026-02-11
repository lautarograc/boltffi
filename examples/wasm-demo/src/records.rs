use boltffi::*;

#[data]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[export]
pub fn echo_point(p: Point) -> Point {
    p
}

#[export]
pub fn make_point(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[export]
pub fn add_points(a: Point, b: Point) -> Point {
    Point {
        x: a.x + b.x,
        y: a.y + b.y,
    }
}

#[export]
pub fn point_distance(p: Point) -> f64 {
    (p.x * p.x + p.y * p.y).sqrt()
}
