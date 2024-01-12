//! Different kind of pseudo-random number generator.

use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{UNIX_EPOCH, SystemTime};
use std::num::Wrapping;

use glam::{Vec3, DVec3};


const MULTIPLIER: Wrapping<i64> = Wrapping(0x5DEECE66D);
const ADDEND: Wrapping<i64> = Wrapping(0xB);
const MASK: Wrapping<i64> = Wrapping((1 << 48) - 1);

const FLOAT_DIV: f32 = (1u32 << 24) as f32;
const DOUBLE_DIV: f64 = (1u64 << 53) as f64;


#[inline]
fn initial_scramble(seed: i64) -> Wrapping<i64> {
    (Wrapping(seed) ^ MULTIPLIER) & MASK
}


/// Generate a new seed in the same way as `java.f.Random` (same constants).
fn gen_seed() -> i64 {
    static SEED: AtomicI64 = AtomicI64::new(8682522807148012);
    let mut current = SEED.load(Ordering::Relaxed);
    loop {
        let next = current.wrapping_mul(181783497276652981);
        match SEED.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => {
                // This is a bit different from Java implementation because the nano time
                // as an integer value is not available in Rust, even with Instant.
                // So we're using duration since unix epoch of the system time, maybe not
                // as safe as the Java implementation.
                return match SystemTime::now().duration_since(UNIX_EPOCH) {
                    Ok(d) => next ^ (d.as_nanos() as i64),
                    Err(_) => next
                };
            }
            Err(old) => current = old
        }
    }
}


/// A pseudo-random number generator ported from the Java standard *RNG* with additional
/// utility methods better suited for rust.
#[derive(Debug, Clone)]
pub struct JavaRandom {
    seed: Wrapping<i64>,
    next_gaussian: Option<f64>,
}

impl Default for JavaRandom {
    fn default() -> Self {
        Self::new_seeded()
    }
}

impl JavaRandom {

    #[inline]
    pub fn new(seed: i64) -> JavaRandom {
        JavaRandom { seed: initial_scramble(seed), next_gaussian: None }
    }

    #[inline]
    pub fn new_seeded() -> JavaRandom {
        Self::new(gen_seed())
    }

    #[inline]
    pub fn new_blank() -> JavaRandom {
        JavaRandom { seed: Wrapping(0), next_gaussian: None }
    }

    #[inline]
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = initial_scramble(seed);
    }

    #[inline]
    pub fn get_seed(&self) -> i64 {
        self.seed.0
    }

    pub fn next_blank(&mut self) {
        self.seed = (self.seed * MULTIPLIER + ADDEND) & MASK;
    }

    #[inline]
    fn next(&mut self, bits: u8) -> i32 {
        self.next_blank();
        (self.seed.0 as u64 >> (48 - bits)) as i32
    }

    #[inline]
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {

        debug_assert!(bound >= 0, "bound is negative");

        if (bound & -bound) == bound {
            // If the bound is a power of two, this is simpler.
            (((bound as i64).wrapping_mul(self.next(31) as i64)) >> 31) as i32
        } else {

            let mut bits;
            let mut val;

            loop {
                bits = self.next(31);
                val = bits.rem_euclid(bound);
                if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                    break;
                }
            }

            val

        }

    }

    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// Get the next pseudo-random single-precision float.
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / FLOAT_DIV
    }

    /// Get the next pseudo-random double-precision float.
    pub fn next_double(&mut self) -> f64 {
        let high = (self.next(26) as i64) << 27;
        let low = self.next(27) as i64;
        (high.wrapping_add(low) as f64) / DOUBLE_DIV
    }

    /// Get the next pseudo-random double-precision Gaussian random number.
    pub fn next_gaussian(&mut self) -> f64 {
        if let Some(next_gaussian) = self.next_gaussian.take() {
            next_gaussian
        } else {
            loop {
                let v1 = self.next_double() * 2.0 - 1.0;
                let v2 = self.next_double() * 2.0 - 1.0;
                let s = v1 * v1 + v2 * v2;
                if s < 1.0 && s != 0.0 {
                    let multiplier = (-2.0 * s.ln() / s).sqrt();
                    self.next_gaussian = Some(v2 * multiplier);
                    break v1 * multiplier;
                }
            }
        }
    }
    
    /// Get the next pseudo-random single-precision float vector, x, y and z.
    /// **This is not part of the standard Java class.**
    pub fn next_float_vec(&mut self) -> Vec3 {
        Vec3 { 
            x: self.next_float(), 
            y: self.next_float(),
            z: self.next_float(),
        }
    }

    /// Get the next pseudo-random double-precision float vector, x, y and z.
    /// **This is not part of the standard Java class.**
    pub fn next_double_vec(&mut self) -> DVec3 {
        DVec3 {
            x: self.next_double(), 
            y: self.next_double(),
            z: self.next_double(),
        }
    }

    /// Get the next pseudo-random double-precision double vector, x, y and z, 
    /// with Gaussian distribution.
    /// **This is not part of the standard Java class.**
    pub fn next_gaussian_vec(&mut self) -> DVec3 {
        DVec3 {
            x: self.next_gaussian(), 
            y: self.next_gaussian(),
            z: self.next_gaussian(),
        }
    }

    /// Randomly pick an item in the given slice.
    /// **This is not part of the standard Java class.**
    #[inline]
    pub fn next_choice<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.next_int_bounded(items.len() as i32) as usize]
    }

    /// Randomly pick an item in the given slice and return mutable reference to it.
    /// **This is not part of the standard Java class.**
    #[inline]
    pub fn next_choice_ref<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.next_int_bounded(items.len() as i32) as usize]
    }

    /// Randomly pick an item in the given slice and return mutable reference to it.
    /// **This is not part of the standard Java class.**
    #[inline]
    pub fn next_choice_mut<'a, T>(&mut self, items: &'a mut [T]) -> &'a mut T {
        &mut items[self.next_int_bounded(items.len() as i32) as usize]
    }

}
