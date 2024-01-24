//! Math utilities specialized for Minecraft, such as sin/cos precomputed tables, in order
//! to get the best parity with Minecraft generation.

#[allow(clippy::approx_constant)]
const JAVA_PI: f64 = 3.141592653589793;


/// We internally do not use a precomputed table as in Notchian implementation. For now
/// we recompute the table value on each access.
#[inline(always)]
fn mc_sin_table(index: u16) -> f32 {
    (index as f64 * JAVA_PI * 2.0 / 65536.0).sin() as f32
}

#[inline]
fn mc_sin(x: f32) -> f32 {
    mc_sin_table((x * 10430.378) as i32 as u16)
}

#[inline]
fn mc_cos(x: f32) -> f32 {
    mc_sin_table((x * 10430.378 + 16384.0) as i32 as u16)
}


/// An extension trait to numbers.
pub trait MinecraftMath: Copy {

    const MC_PI: Self;

    /// Computes the sine of a number (in radians) with parity with Notchian impl.
    fn mc_sin(self) -> Self;

    /// Computes the cosine of a number (in radians) with parity with Notchian impl.
    fn mc_cos(self) -> Self;

    /// Same as [`f32::sin_cos`] but for Notchian impl.
    #[inline]
    fn mc_sin_cos(self) -> (Self, Self) {
        (self.mc_sin(), self.mc_cos())
    }

}

impl MinecraftMath for f32 {

    const MC_PI: Self = JAVA_PI as f32;

    #[inline]
    fn mc_sin(self) -> Self {
        mc_sin(self)
    }

    #[inline]
    fn mc_cos(self) -> Self {
        mc_cos(self)
    }

}

impl MinecraftMath for f64 {

    const MC_PI: Self = JAVA_PI;

    #[inline]
    fn mc_sin(self) -> Self {
        mc_sin(self as f32) as f64
    }

    #[inline]
    fn mc_cos(self) -> Self {
        mc_cos(self as f32) as f64
    }

}
