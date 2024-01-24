//! Various uncategorized utilities.


/// A function to better inline the default function call.
#[inline(always)]
pub(crate) fn default<T: Default>() -> T {
    T::default()
}


/// A fading average
#[derive(Debug, Clone, Default)]
pub struct FadingAverage {
    value: f32,
}

impl FadingAverage {

    #[inline]
    pub fn push(&mut self, value: f32, factor: f32) {
        self.value = (self.value * (1.0 - factor)) + value * factor;
    }

    #[inline]
    pub fn get(&self) -> f32 {
        self.value
    }

}
