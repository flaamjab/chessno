#[derive(Clone, Copy, Default, Debug)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone)]
pub struct Rectangle {
    top_left: Point2D,
    bottom_right: Point2D,
    width: f64,
    height: f64,
}

impl Rectangle {
    pub fn new(top_left: Point2D, bottom_right: Point2D) -> Self {
        assert!(top_left.x <= bottom_right.x && top_left.y <= bottom_right.y);
        Self {
            top_left,
            bottom_right,
            width: bottom_right.x - top_left.x,
            height: bottom_right.y - top_left.y,
        }
    }

    pub fn contains(&self, point: Point2D) -> bool {
        point.x > self.top_left.x
            && point.x < self.bottom_right.x
            && point.y > self.top_left.y
            && point.y < self.bottom_right.y
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }
}
