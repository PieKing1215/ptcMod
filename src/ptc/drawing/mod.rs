use self::color::Color;

pub mod color;
pub mod ddraw;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect<T> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T,
}

impl<T> Rect<T> {
    pub fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self { left, top, right, bottom }
    }
}

impl Rect<i32> {
    pub fn as_lprect(&mut self) -> winapi::shared::windef::LPRECT {
        static_assertions::assert_eq_size!(Rect<i32>, winapi::shared::windef::RECT);
        static_assertions::assert_eq_align!(Rect<i32>, winapi::shared::windef::RECT);

        std::ptr::from_mut(self).cast()
    }
}

impl<T: std::ops::Sub<Output = T> + Copy> Rect<T> {
    pub fn width(&self) -> T {
        self.right - self.left
    }

    pub fn height(&self) -> T {
        self.bottom - self.top
    }
}

pub trait Draw {
    unsafe fn fill_rect(&mut self, rect: &Rect<i32>, color: Color);

    #[deprecated = "ddraw doesn't actually implement this so it's useless"]
    #[expect(unused)]
    unsafe fn fill_rect_batch(&mut self, rects: Vec<Rect<i32>>, color: Color);
}
