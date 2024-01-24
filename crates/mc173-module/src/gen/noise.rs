//! Perlin and octaves noise generators at parity with Minecraft world generation.
//! 
//! Note that in this module, every usage of the primitive cast operator `as` that
//! are narrowing the values are documented and justified to be at parity with the
//! Java conversion.
//! 
//! Reference for the narrowing casts in Java: 
//! <https://docs.oracle.com/javase/specs/jls/se7/html/jls-5.html>

use std::mem;
use std::fmt;

use glam::{DVec3, DVec2, IVec3};

use crate::rand::JavaRandom;


/// A cube of given size for storing noise values.
#[repr(transparent)]
#[derive(Clone, PartialEq)]
pub struct NoiseCube<const X: usize, const Y: usize, const Z: usize> {
    inner: [[[f64; Y]; Z]; X],
}

impl<const X: usize, const Y: usize, const Z: usize> NoiseCube<X, Y, Z> {

    #[inline]
    pub fn new() -> Self {
        Self {
            inner: [[[0.0; Y]; Z]; X],
        }
    }

    #[inline]
    pub fn fill(&mut self, value: f64) {
        self.inner = [[[value; Y]; Z]; X];
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> f64 {
        self.inner[x][z][y]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: f64) {
        self.inner[x][z][y] = value;
    }

    #[inline]
    pub fn add(&mut self, x: usize, y: usize, z: usize, value: f64) {
        self.inner[x][z][y] += value;
    }

}

impl<const X: usize, const Y: usize, const Z: usize> Default for NoiseCube<X, Y, Z> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const X: usize, const Y: usize, const Z: usize> fmt::Debug for NoiseCube<X, Y, Z> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NoiseCube").field(&self.inner).finish()
    }
}


impl NoiseCube<1, 1, 1> {

    /// Create a reference to a 1x1x1 noise cube from a reference to a single f64.
    #[inline(always)]
    pub fn from_ref(value: &f64) -> &Self {
        // We use this assert to ensure that size is correct, just to be sure.
        const __ASSERT: () = assert!(mem::size_of::<NoiseCube<1, 1, 1>>() == mem::size_of::<f64>());
        // SAFETY: both f64 and NoiseCube<1, 1, 1> have are guaranteed to have the same
        // layout because NoiseCube is a transparent struct around an array, and arrays
        // have a known layout where first element is at byte offset 0.
        // This is almost identical to std::array::from_ref
        unsafe { &*(value as *const f64 as *const Self) }
    }
    
    /// Create a mutable reference to a 1x1x1 noise cube from a mutable reference to a 
    /// single f64.
    #[inline(always)]
    pub fn from_mut(value: &mut f64) -> &mut Self {
        // SAFETY: Same as above.
        unsafe { &mut *(value as *mut f64 as *mut Self) }
    }

}


/// A 3D/2D Perlin noise generator.
#[derive(Debug, Clone)]
pub struct PerlinNoise {
    /// Offset applied to all position given to the generator.
    offset: DVec3,
    /// All permutations used by Perlin noise algorithm.
    permutations: Box<[u16; 512]>,
}

impl PerlinNoise {

    /// Create a new perlin noise initialized with the given RNG.
    pub fn new(rand: &mut JavaRandom) -> Self {

        // NOTE: The narrowing casts in this function are safe because with statically
        // know that we are in range (512 or 256) and i32 or u16 is enough.

        let offset = rand.next_double_vec() * 256.0;

        let mut permutations = Box::new(std::array::from_fn::<u16, 512, _>(|i| {
            if i <= 256 {
                i as u16
            } else {
                0
            }
        }));

        for index in 0usize..256 {
            let permutation_index = rand.next_int_bounded(256 - index as i32) as usize + index;
            permutations.swap(index, permutation_index);
            permutations[index + 256] = permutations[index];
        }

        Self {
            offset,
            permutations,
        }

    }

    /// Get the noise value at given 3D coordinates.
    pub fn gen_3d_point(&self, pos: DVec3) -> f64 {

        let mut pos = pos + self.offset;
        let pos_floor = pos.floor();
        pos -= pos_floor;
        let factor = pos * pos * pos * (pos * (pos * 6.0 - 15.0) + 10.0);
        
        let pos_int = pos_floor.as_ivec3();
        let x_index = (pos_int.x & 255) as usize;
        let y_index = (pos_int.y & 255) as usize;
        let z_index = (pos_int.z & 255) as usize;

        let a = self.permutations[x_index] as usize + y_index;
        let a0 = self.permutations[a] as usize + z_index;
        let a1 = self.permutations[a + 1] as usize + z_index;
        let b = self.permutations[x_index + 1] as usize + y_index;
        let b0 = self.permutations[b] as usize + z_index;
        let b1 = self.permutations[b + 1] as usize + z_index;

        let DVec3 { x, y, z } = pos;

        lerp(factor.z,
            lerp(factor.y, 
                lerp(factor.x, 
                    grad3(self.permutations[a0], x      , y, z), 
                    grad3(self.permutations[b0], x - 1.0, y, z)), 
                lerp(factor.x,
                    grad3(self.permutations[a1], x      , y - 1.0, z),
                    grad3(self.permutations[b1], x - 1.0, y - 1.0, z))),
            lerp(factor.y,
                lerp(factor.x,
                    grad3(self.permutations[a0 + 1], x      , y, z - 1.0),
                    grad3(self.permutations[b0 + 1], x - 1.0, y, z - 1.0)),
                lerp(factor.x,
                    grad3(self.permutations[a1 + 1], x      , y - 1.0, z - 1.0),
                    grad3(self.permutations[b1 + 1], x - 1.0, y - 1.0, z - 1.0))))

    }

    /// Get the noise value at given 2D coordinates.
    pub fn gen_2d_point(&self, pos: DVec2) -> f64 {
        self.gen_3d_point(pos.extend(0.0))
    }

    /// Generate a 3D noise cube at a given offset with the given scale and frequency.
    pub fn gen_3d<const X: usize, const Y: usize, const Z: usize>(&self, 
        cube: &mut NoiseCube<X, Y, Z>,
        offset: DVec3,
        scale: DVec3,
        amplitude: f64
    ) {
        
        let mut last_y_index = usize::MAX;

        let mut x0 = 0.0;
        let mut x1 = 0.0;
        let mut x2 = 0.0;
        let mut x3 = 0.0;

        for x_cube in 0..X {
            let (x, x_factor, x_index) = calc_pos((offset.x + x_cube as f64) * scale.x + self.offset.x);
            for z_cube in 0..Z {
                let (z, z_factor, z_index) = calc_pos((offset.z + z_cube as f64) * scale.z + self.offset.z);
                for y_cube in 0..Y {
                    let (y, y_factor, y_index) = calc_pos((offset.y + y_cube as f64) * scale.y + self.offset.y);

                    if y_cube == 0 || y_index != last_y_index {
                        
                        last_y_index = y_index;

                        let a = self.permutations[x_index] as usize + y_index;
                        let a0 = self.permutations[a] as usize + z_index;
                        let a1 = self.permutations[a + 1] as usize + z_index;
                        let b = self.permutations[x_index + 1] as usize + y_index;
                        let b0 = self.permutations[b] as usize + z_index;
                        let b1 = self.permutations[b + 1] as usize + z_index;

                        x0 = lerp(x_factor, 
                            grad3(self.permutations[a0], x      , y, z), 
                            grad3(self.permutations[b0], x - 1.0, y, z));
                        x1 = lerp(x_factor,
                            grad3(self.permutations[a1], x      , y - 1.0, z),
                            grad3(self.permutations[b1], x - 1.0, y - 1.0, z));
                        x2 = lerp(x_factor,
                            grad3(self.permutations[a0 + 1], x      , y, z - 1.0),
                            grad3(self.permutations[b0 + 1], x - 1.0, y, z - 1.0));
                        x3 = lerp(x_factor,
                            grad3(self.permutations[a1 + 1], x      , y - 1.0, z - 1.0),
                            grad3(self.permutations[b1 + 1], x - 1.0, y - 1.0, z - 1.0));

                    }

                    let noise = lerp(z_factor, lerp(y_factor, x0, x1), lerp(y_factor, x2, x3));
                    cube.add(x_cube, y_cube, z_cube, noise * amplitude);

                }
            }
        }

    }

    /// Generate a 2D noise cube at a given offset with the given scale and frequency.
    pub fn gen_2d<const X: usize, const Z: usize>(&self,
        cube: &mut NoiseCube<X, 1, Z>,
        offset: DVec2,
        scale: DVec2,
        amplitude: f64
    ) {

        for x_cube in 0..X {
            let (x, x_factor, x_index) = calc_pos((offset.x + x_cube as f64) * scale.x + self.offset.x);
            for z_cube in 0..Z {
                let (z, z_factor, z_index) = calc_pos((offset.y + z_cube as f64) * scale.y + self.offset.z);
                
                let a = self.permutations[x_index] as usize + 0;
                let a0 = self.permutations[a] as usize + z_index;
                let b = self.permutations[x_index + 1] as usize + 0;
                let b0 = self.permutations[b] as usize + z_index;

                let noise = lerp(z_factor,
                    lerp(x_factor,
                        grad2(self.permutations[a0], x, z),
                        grad3(self.permutations[b0], x - 1.0, 0.0, z)),
                    lerp(x_factor,
                        grad3(self.permutations[a0 + 1], x, 0.0, z - 1.0),
                        grad3(self.permutations[b0 + 1], x - 1.0, 0.0, z - 1.0)));
                
                cube.add(x_cube, 0, z_cube, noise * amplitude);

            }
        }

    }

    /// Weird noise generation (a handcrafted noise generator used by Notchian server 
    /// that uses the same type of permutations table and offset as the perlin noise, so
    /// we use the same structure).
    /// 
    /// The function is to be renamed if the algorithm name is found.
    pub fn gen_weird_2d<const X: usize, const Z: usize>(&self,
        cube: &mut NoiseCube<X, 1, Z>,
        offset: DVec2,
        scale: DVec2,
        amplitude: f64
    ) {

        let const_a: f64 = 0.5 * (f64::sqrt(3.0) - 1.0);
        let const_b: f64 = (3.0 - f64::sqrt(3.0)) / 6.0;
        
        for x_noise in 0..X {
            let x = (offset.x + x_noise as f64) * scale.x + self.offset.x;
            for z_noise in 0..Z {
                // NOTE: Using Y component of everything but this is interpreted as Z.
                let z = (offset.y + z_noise as f64) * scale.y + self.offset.y;

                let a = (x + z) * const_a;
                let x_wrap = wrap(x + a);
                let z_wrap = wrap(z + a);

                let b = i32::wrapping_add(x_wrap, z_wrap) as f64 * const_b;
                let x_wrap_b = x_wrap as f64 - b;
                let z_wrap_b = z_wrap as f64 - b;

                let x_delta = x - x_wrap_b;
                let z_delta = z - z_wrap_b;

                let (
                    x_offset, 
                    z_offset
                ) = if x_delta > z_delta { (1, 0) } else { (0, 1) };

                let x_delta0 = x_delta - x_offset as f64 + const_b;
                let z_delta0 = z_delta - z_offset as f64 + const_b;
                let x_delta1 = x_delta - 1.0 + 2.0 * const_b;
                let z_delta1 = z_delta - 1.0 + 2.0 * const_b;

                let x_index = (x_wrap & 255) as usize;
                let z_index = (z_wrap & 255) as usize;

                let v0_index = self.permutations[x_index + self.permutations[z_index] as usize] % 12;
                let v1_index = self.permutations[x_index + x_offset + self.permutations[z_index + z_offset] as usize] % 12;
                let v2_index = self.permutations[x_index + 1 + self.permutations[z_index + 1] as usize] % 12;

                let v0 = calc_weird_noise(x_delta, z_delta, v0_index as usize);
                let v1 = calc_weird_noise(x_delta0, z_delta0, v1_index as usize);
                let v2 = calc_weird_noise(x_delta1, z_delta1, v2_index as usize);

                cube.add(x_noise, 0, z_noise, 70.0 * (v0 + v1 + v2) * amplitude);

            }
        }

    }

}


/// A Perlin-based octave noise generator.
#[derive(Debug, Clone)]
pub struct PerlinOctaveNoise {
    /// Collection of generators for the different octaves.
    generators: Box<[PerlinNoise]>
}

impl PerlinOctaveNoise {

    /// Create a new Perlin-based octaves noise generator.
    pub fn new(rand: &mut JavaRandom, octaves: usize) -> Self {
        Self {
            generators: (0..octaves)
                .map(move |_| PerlinNoise::new(rand))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    /// Get the noise value at given 3D coordinates.
    pub fn gen_3d_point(&self, pos: DVec3) -> f64 {
        let mut ret = 0.0;
        let mut freq = 1.0;
        for gen in &self.generators[..] {
            ret += gen.gen_3d_point(pos * freq) / freq;
            freq /= 2.0;
        }
        ret
    }

    /// Get the noise value at given 3D coordinates.
    pub fn gen_2d_point(&self, pos: DVec2) -> f64 {
        let mut ret = 0.0;
        let mut freq = 1.0;
        for gen in &self.generators[..] {
            ret += gen.gen_2d_point(pos * freq) / freq;
            freq /= 2.0;
        }
        ret
    }

    /// Generate a 3D noise cube at a given offset with the given scale and frequency.
    pub fn gen_3d<const X: usize, const Y: usize, const Z: usize>(&self, 
        cube: &mut NoiseCube<X, Y, Z>,
        offset: DVec3,
        scale: DVec3
    ) {
        cube.fill(0.0);
        let mut freq = 1.0;
        for gen in &self.generators[..] {
            gen.gen_3d(cube, offset, scale * freq, 1.0 / freq);
            freq /= 2.0;
        }
    }

    /// Generate a 2D noise cube at a given offset with the given scale and frequency.
    pub fn gen_2d<const X: usize, const Z: usize>(&self,
        cube: &mut NoiseCube<X, 1, Z>,
        offset: DVec2,
        scale: DVec2,
    ) {
        cube.fill(0.0);
        let mut freq = 1.0;
        for gen in &self.generators[..] {
            gen.gen_2d(cube, offset, scale * freq, 1.0 / freq);
            freq /= 2.0;
        }
    }

    /// Weird noise generation (a handcrafted noise generator used by Notchian server 
    /// that uses the same type of permutations table and offset as the Perlin noise, so
    /// we use the same structure).
    /// 
    /// The function is to be renamed if the algorithm name is found.
    pub fn gen_weird_2d<const X: usize, const Z: usize>(&self,
        cube: &mut NoiseCube<X, 1, Z>,
        offset: DVec2,
        scale: DVec2,
        freq_factor: f64,
    ) {
        cube.fill(0.0);
        let scale = scale / 1.5;
        let mut freq = 1.0;
        let mut amplitude = 0.55;
        for gen in &self.generators[..] {
            gen.gen_weird_2d(cube, offset, scale * freq, amplitude);
            freq *= freq_factor;
            amplitude *= 2.0;
        }
    }

}


#[inline]
fn lerp(factor: f64, from: f64, to: f64) -> f64 {
    from + factor * (to - from)
}

#[inline]
fn grad3(value: u16, x: f64, y: f64, z: f64) -> f64 {
    let value = value & 15;
    let a = if value < 8 { x } else { y };
    let b = if value < 4 { y } else if value != 12 && value != 14 { z } else { x };
    (if value & 1 == 0 { a } else { -a }) + (if value & 2 == 0 { b } else { -b })
}

#[inline]
fn grad2(value: u16, x: f64, z: f64) -> f64 {
    let value = value & 15;
    let a = (1 - ((value & 8) >> 3)) as f64 * x;
    let b = if value < 4 { 0.0 } else if value != 12 && value != 14 { z } else { x };
    (if value & 1 == 0 { a } else { -a }) + (if value & 2 == 0 { b } else { -b })
}

#[inline]
fn calc_pos(mut pos: f64) -> (f64, f64, usize) {

    // PARITY: This function is really important to reproduce the infamous "Far Lands",
    // the manual floor implementation is really important to keep like this, and avoid
    // using the `.floor` function that just ignore the integer overflow rule that 
    // saturate the integer value if the floating point value is too high.

    let mut floor = pos as i32;
    if pos < floor as f64 {
        floor = floor.wrapping_sub(1);
    }

    pos -= floor as f64;
    let factor = pos * pos * pos * (pos * (pos * 6.0 - 15.0) + 10.0);
    let index = (floor & 255) as usize;

    (pos, factor, index)

}

#[inline]
fn wrap(value: f64) -> i32 {
    let ret = value as i32;
    if value > 0.0 { ret } else { ret.wrapping_sub(1) }
}

#[inline]
fn calc_weird_noise(x_delta: f64, z_delta: f64, index: usize) -> f64 {

    static WEIRD_TABLE: [IVec3; 12] = [
        IVec3::new(1, 1, 0),
        IVec3::new(-1, 1, 0),
        IVec3::new(1, -1, 0),
        IVec3::new(-1, -1, 0),
        IVec3::new(1, 0, 1),
        IVec3::new(-1, 0, 1),
        IVec3::new(1, 0, -1),
        IVec3::new(-1, 0, -1),
        IVec3::new(0, 1, 1),
        IVec3::new(0, -1, 1),
        IVec3::new(0, 1, -1),
        IVec3::new(0, -1, -1),
    ];

    let tmp = 0.5 - x_delta * x_delta - z_delta * z_delta;
    if tmp < 0.0 {
        0.0
    } else {
        let tmp = tmp * tmp;
        let weird = WEIRD_TABLE[index];
        tmp * tmp * (weird.x as f64 * x_delta + weird.y as f64 * z_delta)
    }

}
