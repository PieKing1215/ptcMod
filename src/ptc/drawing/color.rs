use colorsys::ColorTransform;

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub trait IntoColorNum {
    fn col_num(self) -> u8;
}

impl IntoColorNum for u8 {
    #[inline]
    fn col_num(self) -> u8 {
        self
    }
}

impl IntoColorNum for f32 {
    #[inline]
    fn col_num(self) -> u8 {
        (self * f32::from(u8::MAX)).clamp(f32::from(u8::MIN), f32::from(u8::MAX)) as u8
    }
}

impl Color {
    #[inline]
    pub const fn rgba_const(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn rgb_const(r: u8, g: u8, b: u8) -> Self {
        Self::rgba_const(r, g, b, u8::MAX)
    }

    #[inline]
    pub fn rgba(
        r: impl IntoColorNum,
        g: impl IntoColorNum,
        b: impl IntoColorNum,
        a: impl IntoColorNum,
    ) -> Self {
        Self::rgba_const(r.col_num(), g.col_num(), b.col_num(), a.col_num())
    }

    #[inline]
    pub fn rgb(r: impl IntoColorNum, g: impl IntoColorNum, b: impl IntoColorNum) -> Self {
        Self::rgba(r, g, b, u8::MAX)
    }

    #[inline]
    #[must_use]
    pub fn with_a(self, a: impl IntoColorNum) -> Self {
        Self::rgba_const(self.r, self.g, self.b, a.col_num())
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn r_f32(&self) -> f32 {
        f32::from(self.r) / f32::from(u8::MAX)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn g_f32(&self) -> f32 {
        f32::from(self.g) / f32::from(u8::MAX)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn b_f32(&self) -> f32 {
        f32::from(self.b) / f32::from(u8::MAX)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn a_f32(&self) -> f32 {
        f32::from(self.a) / f32::from(u8::MAX)
    }

    pub fn into_argb(self) -> u32 {
        u32::from_be_bytes([self.a, self.r, self.g, self.b])
    }

    pub fn from_argb(argb: u32) -> Self {
        let [a, r, g, b] = u32::to_be_bytes(argb);
        Self { r, g, b, a }
    }

    pub fn rotate_hue(self, rotate: f64) -> Self {
        let mut rgb = colorsys::Rgb::from((self.r, self.g, self.b));
        rgb.adjust_hue(rotate);
        let rgb_arr: (u8, u8, u8) = rgb.into();
        Self {
            r: rgb_arr.0,
            g: rgb_arr.1,
            b: rgb_arr.2,
            a: self.a,
        }
    }

    pub fn blend(self, other: Self, factor: f32) -> Self {
        let r = self.r_f32() + (other.r_f32() - self.r_f32()) * factor;
        let g = self.g_f32() + (other.g_f32() - self.g_f32()) * factor;
        let b = self.b_f32() + (other.b_f32() - self.b_f32()) * factor;
        let a = self.a_f32() + (other.a_f32() - self.a_f32()) * factor;
        Self::rgba(r, g, b, a)
    }

    pub const BLACK: Color = Color::rgb_const(0, 0, 0);
    pub const WHITE: Color = Color::rgb_const(0xff, 0xff, 0xff);
    pub const GRAY: Color = Color::rgb_const(0x7f, 0x7f, 0x7f);

    pub const RED: Color = Color::rgb_const(0xff, 0, 0);
    pub const GREEN: Color = Color::rgb_const(0, 0xff, 0);
    pub const BLUE: Color = Color::rgb_const(0, 0, 0xff);

    pub const YELLOW: Color = Color::rgb_const(0xff, 0xff, 0);
    pub const CYAN: Color = Color::rgb_const(0, 0xff, 0xff);
    pub const MAGENTA: Color = Color::rgb_const(0xff, 0, 0xff);

    pub const ORANGE: Color = Color::rgb_const(0xff, 0x7f, 0);
    pub const CHARTREUSE_GREEN: Color = Color::rgb_const(0x7f, 0xff, 0);
    pub const ROSE: Color = Color::rgb_const(0xff, 0, 0x7f);
    pub const VIOLET: Color = Color::rgb_const(0x7f, 0, 0xff);
    pub const SPRING_GREEN: Color = Color::rgb_const(0, 0xff, 0x7f);
    pub const AZURE: Color = Color::rgb_const(0, 0x7f, 0xff);
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> [f32; 4] {
        [color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32()]
    }
}

impl From<Color> for (f32, f32, f32, f32) {
    fn from(color: Color) -> (f32, f32, f32, f32) {
        (color.r_f32(), color.g_f32(), color.b_f32(), color.a_f32())
    }
}
impl From<Color> for [f32; 3] {
    fn from(color: Color) -> [f32; 3] {
        [color.r_f32(), color.g_f32(), color.b_f32()]
    }
}

impl From<Color> for (f32, f32, f32) {
    fn from(color: Color) -> (f32, f32, f32) {
        (color.r_f32(), color.g_f32(), color.b_f32())
    }
}
