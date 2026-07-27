use windows::Win32::Foundation::RECT;

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
    pub fn as_lprect(&mut self) -> *mut RECT {
        static_assertions::assert_eq_size!(Rect<i32>, RECT);
        static_assertions::assert_eq_align!(Rect<i32>, RECT);

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

impl<T: std::ops::Add<Output = T> + Copy> Rect<T> {
    pub fn offset(&self, x: T, y: T) -> Self {
        Self {
            left: self.left + x,
            top: self.top + y,
            right: self.right + x,
            bottom: self.bottom + y,
        }
    }
}

pub trait Draw {
    unsafe fn fill_rect(&self, rect: &Rect<i32>, color: Color);

    #[deprecated = "ddraw doesn't actually implement this so it's useless"]
    #[expect(unused)]
    unsafe fn fill_rect_batch(&self, rects: Vec<Rect<i32>>, color: Color);

    fn set_pixels(&self, modify: impl FnOnce(&mut dyn FnMut(i32, i32, Color)));

    fn offset(&self, x: i32, y: i32) -> impl Draw
    where
        Self: Sized,
    {
        OffsetDraw { inner: self, x, y }
    }
}

struct OffsetDraw<'a, T> {
    inner: &'a T,
    x: i32,
    y: i32,
}

impl<T: Draw> Draw for OffsetDraw<'_, T> {
    unsafe fn fill_rect(&self, rect: &Rect<i32>, color: Color) {
        let ofs_rect = rect.offset(self.x, self.y);
        unsafe { self.inner.fill_rect(&ofs_rect, color) };
    }

    unsafe fn fill_rect_batch(&self, rects: Vec<Rect<i32>>, color: Color) {
        let ofs_rects = rects
            .into_iter()
            .map(|rect| rect.offset(self.x, self.y))
            .collect();
        unsafe {
            #[expect(deprecated)]
            self.inner.fill_rect_batch(ofs_rects, color);
        };
    }

    fn set_pixels(&self, modify: impl FnOnce(&mut dyn FnMut(i32, i32, Color))) {
        self.inner
            .set_pixels(|f| modify(&mut |x, y, color| f(x + self.x, y + self.y, color)));
    }
}
