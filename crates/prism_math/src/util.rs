fn xyz_to_idx(i: i32, y_length: i32, x_length: i32) -> (i32, i32, i32) {
    let x = i % x_length;
    let y = (i / x_length) % y_length;
    let z = i / (x_length * y_length);
    (x, y, z)
}

#[macro_export]
macro_rules! xy_to_index {
    ($x:expr, $y:expr, $size:expr) => {
        $x + ($size * $y)
    };
}

#[macro_export]
macro_rules! xyz_to_index {
    ($x:expr, $y:expr, $z:expr, $x_length:expr, $y_length:expr) => {
        $x + $y * $x_length + $z * $x_length * $y_length
    };
}

/// iterate all points between point a and point b
pub struct Line2d {
    point_a: (i32, i32),
    point_b: (i32, i32),
    x: i32,
    y: i32,
    p: i32,
    dx: i32,
    dy: i32,
    flip: i32,
    m: i32,
    x1: i32,
    x2: i32,
    y1: i32,
    y2: i32,
}

impl Line2d {
    pub fn new(point_a: (i32, i32), point_b: (i32, i32)) -> Self {
        let mut flip = 0;
        let (mut x1, mut x2, mut y1, mut y2) = (point_a.0, point_b.0, point_a.1, point_b.1);
        let mut dx = 0;
        let mut dy = 0;
        let mut x = 0;
        let mut y = 0;
        let mut p = 0;

        let m = if (x2 - x1) == 0 {
            y2 - y1
        } else {
            (y2 - y1) / (x2 - x1)
        };

        if m.abs() < 1 {
            if x1 > x2 {
                // flip = 1;
                //  std::mem::swap(&mut x1, &mut x2);
                //  std::mem::swap(&mut y1, &mut y2);
            }

            dy = (y2 - y1).abs();
            dx = (x2 - x1).abs();

            p = 2 * dy - dx;

            y = y1;
            x = x1;
        }
        if m.abs() >= 1 {
            {
                if y1 > y2 {
                    //   flip = 1;
                    //    std::mem::swap(&mut x1, &mut x2);
                    //     std::mem::swap(&mut y1, &mut y2);
                }
                dy = (y2 - y1).abs();
                dx = (x2 - x1).abs();
                //  dy = (y2 - y1).abs();
                //        dx = (x2 - x1).abs();

                p = 2 * dx - dy;

                y = y1;
                x = x1;
            }
        }
        Self {
            point_a,
            point_b,
            x1,
            x2,
            y1,
            y2,
            dx,
            dy,
            x,
            y,
            p,
            m,
            flip,
        }
    }
}

impl Iterator for Line2d {
    type Item = (i32, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.m.abs() < 1 {
            let r_point = (self.x, self.y);
            if self.x1 > self.x2 {
                self.x = self.x - 1;
                self.p = if self.p >= 0 {
                    self.y = if self.m >= 1 || self.y1 < self.y2 {
                        self.y + 1
                    } else {
                        self.y - 1
                    };
                    self.p + 2 * self.dy - 2 * self.dx
                } else {
                    self.p + 2 * self.dy
                };
                if self.x >= self.x2 {
                    return Some(r_point);
                }
            } else if self.x1 < self.x2 {
                self.x = self.x + 1;
                self.p = if self.p >= 0 {
                    self.y = if self.m >= 1 || self.y1 < self.y2 {
                        self.y + 1
                    } else {
                        self.y - 1
                    };
                    self.p + 2 * self.dy - 2 * self.dx
                } else {
                    self.p + 2 * self.dy
                };
                if self.x <= self.x2 {
                    return Some(r_point);
                }
            }
        }
        if self.m.abs() >= 1 {
            let r_point = (self.x, self.y);
            if self.y1 > self.y2 {
                // down
                self.y -= 1;
                self.p = if self.p >= 0 {
                    self.x = if self.m >= 1 { self.x - 1 } else { self.x + 1 };
                    self.p + 2 * self.dx - 2 * self.dy
                } else {
                    self.p + 2 * self.dx
                };
                if self.y >= self.y2 {
                    return Some(r_point);
                }
            } else if self.y1 < self.y2 {
                // up
                self.y += 1;
                self.p = if self.p >= 0 {
                    self.x = if self.m >= 1 { self.x + 1 } else { self.x - 1 };
                    self.p + 2 * self.dx - 2 * self.dy
                } else {
                    self.p + 2 * self.dx
                };
                if self.y <= self.y2 {
                    return Some(r_point);
                }
            }
        }

        return None;
    }
}
