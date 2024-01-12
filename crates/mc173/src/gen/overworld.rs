//! Overworld chunk generator.
//! 
//! The overworld generator is fully featured and should produces the same chunk terrain
//! and randomly the same features.
//! 
//! The overworld generator is an heavy piece of algorithm, and the runtime duration of
//! generation and population depends on optimization level:
//! - Release: populate take around 75% of time to generate
//! - Debug: populate take around 150% of time to generate
//! 
//! However, it's important to note that to fully generate a chunk in middle of nowhere,
//! it's required to generate terrain for 9 chunks and populate only one, so populating
//! in this case will always be faster that terrain generation. Even in the worst case
//! of no optimization, populate only represent around 16% of terrain generation time, 
//! for each fully populated chunk.
//! 
//! If we take a more realistic approach of loading a chunk near already-existing chunks,
//! we only need to generate 2 to 4 chunks, in the worst case, populate represent 75% of
//! terrain generation time, for each fully populated chunk.
//! 
//! We see that in general, we will have more terrain generation than populating to run.

use glam::{DVec2, IVec3, Vec3Swizzles, DVec3};

use crate::chunk::{Chunk, CHUNK_WIDTH, CHUNK_HEIGHT};
use crate::block::material::Material;
use crate::rand::JavaRandom;
use crate::biome::Biome;
use crate::world::World;
use crate::block;

use super::noise::{PerlinOctaveNoise, NoiseCube};
use super::{ChunkGenerator, FeatureGenerator};
use super::plant::{PlantGenerator, SugarCanesGenerator, PumpkinGenerator, CactusGenerator};
use super::liquid::{LakeGenerator, LiquidGenerator};
use super::dungeon::DungeonGenerator;
use super::cave::CaveGenerator;
use super::vein::VeinGenerator;
use super::tree::TreeGenerator;


const NOISE_WIDTH: usize = 5;
const NOISE_HEIGHT: usize = 17;

const TEMPERATURE_SCALE: DVec2 = DVec2::splat(0.025f32 as f64);
const TEMPERATURE_FREQ_FACTOR: f64 = 0.25;
const HUMIDITY_SCALE: DVec2 = DVec2::splat(0.05f32 as f64);
const HUMIDITY_FREQ_FACTOR: f64 = 1.0 / 3.0;
const BIOME_SCALE: DVec2 = DVec2::splat(0.25);
const BIOME_FREQ_FACTOR: f64 = 0.5882352941176471;


/// A chunk generator for the overworld dimension. This structure can be shared between
/// workers.
pub struct OverworldGenerator {
    /// The world seed.
    seed: i64,
    /// The noise used for generating biome temperature.
    temperature_noise: PerlinOctaveNoise,
    /// The noise used for generating biome humidity.
    humidity_noise: PerlinOctaveNoise,
    /// The noise used to alter both temperature and humidity for biome.
    biome_noise: PerlinOctaveNoise,
    terrain_noise0: PerlinOctaveNoise,
    terrain_noise1: PerlinOctaveNoise,
    terrain_noise2: PerlinOctaveNoise,
    terrain_noise3: PerlinOctaveNoise,
    terrain_noise4: PerlinOctaveNoise,
    sand_gravel_noise: PerlinOctaveNoise,
    thickness_noise: PerlinOctaveNoise,
    feature_noise: PerlinOctaveNoise,
    biome_table: Box<[Biome; 4096]>,
}

/// This structure stores huge structures that should not be shared between workers.
#[derive(Default, Clone)]
pub struct OverworldState {
    temperature: NoiseCube<CHUNK_WIDTH, 1, CHUNK_WIDTH>,
    humidity: NoiseCube<CHUNK_WIDTH, 1, CHUNK_WIDTH>,
    biome: NoiseCube<CHUNK_WIDTH, 1, CHUNK_WIDTH>,
    terrain:  NoiseCube<NOISE_WIDTH, NOISE_HEIGHT, NOISE_WIDTH>,
    terrain0: NoiseCube<NOISE_WIDTH, NOISE_HEIGHT, NOISE_WIDTH>,
    terrain1: NoiseCube<NOISE_WIDTH, NOISE_HEIGHT, NOISE_WIDTH>,
    terrain2: NoiseCube<NOISE_WIDTH, NOISE_HEIGHT, NOISE_WIDTH>,
    terrain3: NoiseCube<NOISE_WIDTH, 1, NOISE_WIDTH>,
    terrain4: NoiseCube<NOISE_WIDTH, 1, NOISE_WIDTH>,
    sand: NoiseCube<CHUNK_WIDTH, CHUNK_WIDTH, 1>,
    gravel: NoiseCube<CHUNK_WIDTH, 1, CHUNK_WIDTH>,
    thickness: NoiseCube<CHUNK_WIDTH, CHUNK_WIDTH, 1>,
}

impl OverworldGenerator {

    /// Create a new overworld generator given a seed.
    pub fn new(seed: i64) -> Self {

        let biome_lookup = Box::new(std::array::from_fn(|i| {
            
            let t = (i % 64) as f32 / 63.0;
            let h = (i / 64) as f32 / 63.0;
            let h = h * t;

            if t < 0.1 {
                Biome::Tundra
            } else if h < 0.2 {
                if t < 0.5 {
                    Biome::Tundra
                } else if t < 0.95 {
                    Biome::Savanna
                } else {
                    Biome::Desert
                }
            } else if h > 0.5 && t < 0.7 {
                Biome::Swampland
            } else if t < 0.5 {
                Biome::Taiga
            } else if t < 0.97 {
                if h < 0.35 {
                    Biome::ShrubLand
                } else {
                    Biome::Forest
                }
            } else if h < 0.45 {
                Biome::Plains
            } else if h < 0.9 {
                Biome::SeasonalForest
            } else {
                Biome::RainForest
            }

        }));

        let mut rand = JavaRandom::new(seed);

        Self {
            seed,
            temperature_noise: PerlinOctaveNoise::new(&mut JavaRandom::new(seed.wrapping_mul(9871)), 4),
            humidity_noise: PerlinOctaveNoise::new(&mut JavaRandom::new(seed.wrapping_mul(39811)), 4),
            biome_noise: PerlinOctaveNoise::new(&mut JavaRandom::new(seed.wrapping_mul(543321)), 2),
            terrain_noise0: PerlinOctaveNoise::new(&mut rand, 16),
            terrain_noise1: PerlinOctaveNoise::new(&mut rand, 16),
            terrain_noise2: PerlinOctaveNoise::new(&mut rand, 8),
            sand_gravel_noise: PerlinOctaveNoise::new(&mut rand, 4),
            thickness_noise: PerlinOctaveNoise::new(&mut rand, 4),
            terrain_noise3: PerlinOctaveNoise::new(&mut rand, 10),
            terrain_noise4: PerlinOctaveNoise::new(&mut rand, 16),
            feature_noise: PerlinOctaveNoise::new(&mut rand, 8),
            biome_table: biome_lookup,
        }

    }

    /// Internal function to calculate the biome from given random variables.
    #[inline]
    fn calc_biome(&self, temperature: f64, humidity: f64, biome: f64) -> (f64, f64, Biome) {

        let a = biome * 1.1 + 0.5;
        let t = (temperature * 0.15 + 0.7) * 0.99 + a * 0.01;
        let t = 1.0 - (1.0 - t).powi(2);
        let h = (humidity * 0.15 + 0.5) * 0.998 + a * 0.002;

        let t = t.clamp(0.0, 1.0);
        let h = h.clamp(0.0, 1.0);

        let pos_biome = self.biome_table[(t * 63.0) as usize + (h * 63.0) as usize * 64];
        (t, h, pos_biome)

    }

    /// Get a single biome at given position.
    fn get_biome(&self, x: i32, z: i32) -> Biome {

        let offset = DVec2::new(x as f64, z as f64);
        let mut temperature = 0.0;
        let mut humidity = 0.0;
        let mut biome = 0.0;

        self.temperature_noise.gen_weird_2d(NoiseCube::from_mut(&mut temperature), offset, TEMPERATURE_SCALE, TEMPERATURE_FREQ_FACTOR);
        self.humidity_noise.gen_weird_2d(NoiseCube::from_mut(&mut humidity), offset, HUMIDITY_SCALE, HUMIDITY_FREQ_FACTOR);
        self.biome_noise.gen_weird_2d(NoiseCube::from_mut(&mut biome), offset, BIOME_SCALE, BIOME_FREQ_FACTOR);

        self.calc_biome(temperature, humidity, biome).2

    }

    /// Generate a biome map for the chunk and store it in the chunk data.
    fn gen_biomes(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut OverworldState) {

        let offset = DVec2::new((cx * 16) as f64, (cz * 16) as f64);

        let temperature = &mut state.temperature;
        let humidity = &mut state.humidity;
        let biome = &mut state.biome;
        
        self.temperature_noise.gen_weird_2d(temperature, offset,TEMPERATURE_SCALE, TEMPERATURE_FREQ_FACTOR);
        self.humidity_noise.gen_weird_2d(humidity, offset, HUMIDITY_SCALE, HUMIDITY_FREQ_FACTOR);
        self.biome_noise.gen_weird_2d(biome, offset, BIOME_SCALE, BIOME_FREQ_FACTOR);

        for x in 0usize..16 {
            for z in 0usize..16 {

                let (t, h, pos_biome) = self.calc_biome(
                    temperature.get(x, 0, z), 
                    humidity.get(x, 0, z),
                    biome.get(x, 0, z));
                
                // The value may be used afterward for generation, so we update the value.
                temperature.set(x, 0, z, t);
                humidity.set(x, 0, z, h);

                chunk.set_biome(IVec3::new(x as i32, 0, z as i32), pos_biome);

            }
        }

    }

    /// Generate the primitive terrain of the chunk.
    fn gen_terrain(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut OverworldState) {

        const NOISE_STRIDE: usize = CHUNK_WIDTH / NOISE_WIDTH;
        const NOISE_REAL_WIDTH: usize = NOISE_WIDTH - 1;
        const NOISE_REAL_HEIGHT: usize = NOISE_HEIGHT - 1;
        const NOISE_REAL_WIDTH_STRIDE: usize = CHUNK_WIDTH / NOISE_REAL_WIDTH;
        const NOISE_REAL_HEIGHT_STRIDE: usize = CHUNK_HEIGHT / NOISE_REAL_HEIGHT;

        let offset = IVec3::new(cx * NOISE_REAL_WIDTH as i32, 0, cz * NOISE_REAL_WIDTH as i32);

        let terrain = &mut state.terrain;
        let terrain0 = &mut state.terrain0;
        let terrain1 = &mut state.terrain1;
        let terrain2 = &mut state.terrain2;
        let terrain3 = &mut state.terrain3;
        let terrain4 = &mut state.terrain4;
        let temperature = &state.temperature;
        let humidity = &state.humidity;

        let offset_2d = offset.xz().as_dvec2();
        let offset_3d = offset.as_dvec3();

        self.terrain_noise3.gen_2d(terrain3, offset_2d, DVec2::splat(1.121));
        self.terrain_noise4.gen_2d(terrain4, offset_2d, DVec2::splat(200.0));
        self.terrain_noise2.gen_3d(terrain2, offset_3d, DVec3::new(684.412 / 80.0, 684.412 / 160.0, 684.412 / 80.0));
        self.terrain_noise0.gen_3d(terrain0, offset_3d, DVec3::splat(684.412));
        self.terrain_noise1.gen_3d(terrain1, offset_3d, DVec3::splat(684.412));

        // Start by generating a 5x17x5 density map for the terrain.
        for x_noise in 0..NOISE_WIDTH {
            let x_block = x_noise * NOISE_STRIDE + (NOISE_STRIDE / 2);
            for z_noise in 0..NOISE_WIDTH {
                let z_block = z_noise * NOISE_STRIDE + (NOISE_STRIDE / 2);

                let t = temperature.get(x_block, 0, z_block);
                let h = humidity.get(x_block, 0, z_block) * t;
                let h_inv = (1.0 - h).powi(4);
                let h = 1.0 - h_inv;

                let mut v0 = (terrain3.get(x_noise, 0, z_noise) + 256.0) / 512.0 * h;
                v0 = v0.min(1.0);

                let mut v1 = terrain4.get(x_noise, 0, z_noise) / 8000.0;
                if v1 < 0.0 {
                    v1 = -v1 * 0.3;
                }

                v1 = v1 * 3.0 - 2.0;

                if v1 < 0.0 {
                    v1 /= 2.0;
                    v1 = v1.max(-1.0);
                    v1 /= 1.4;
                    v1 /= 2.0;
                    v0 = 0.0;
                } else {
                    v1 = v1.min(1.0);
                    v1 /= 8.0;
                }

                v0 = v0.max(0.0);
                v0 += 0.5;

                v1 = v1 * NOISE_HEIGHT as f64 / 16.0;
                let v2 = NOISE_HEIGHT as f64 / 2.0 + v1 * 4.0;

                for y_noise in 0..NOISE_HEIGHT {

                    let mut v3 = (y_noise as f64 - v2) * 12.0 / v0;
                    if v3 < 0.0 {
                        v3 *= 4.0;
                    }

                    let v4 = terrain0.get(x_noise, y_noise, z_noise) / 512.0;
                    let v5 = terrain1.get(x_noise, y_noise, z_noise) / 512.0;
                    let v6 = (terrain2.get(x_noise, y_noise, z_noise) / 10.0 + 1.0) / 2.0;

                    // NOTE: Basically a clamped linear interpolation.
                    let mut final_value = if v6 < 0.0 {
                        v4
                    } else if v6 > 1.0 {
                        v5
                    } else {
                        v4 + (v5 - v4) * v6
                    };

                    final_value -= v3;
                    if y_noise > NOISE_HEIGHT - 4 {
                        let v7 = ((y_noise - (NOISE_HEIGHT - 4)) as f32 / 3.0) as f64;
                        final_value = final_value * (1.0 - v7) + (-10.0 * v7);
                    }

                    terrain.set(x_noise, y_noise, z_noise, final_value);

                }

            }
        }

        // Then we read the generated density map and place blocks.
        for x_noise in 0..NOISE_REAL_WIDTH {
            for z_noise in 0..NOISE_REAL_WIDTH {
                for y_noise in 0..NOISE_REAL_HEIGHT {

                    let mut a = terrain.get(x_noise + 0, y_noise + 0, z_noise + 0);
                    let mut b = terrain.get(x_noise + 0, y_noise + 0, z_noise + 1);
                    let mut c = terrain.get(x_noise + 1, y_noise + 0, z_noise + 0);
                    let mut d = terrain.get(x_noise + 1, y_noise + 0, z_noise + 1);
                    let e = (terrain.get(x_noise + 0, y_noise + 1, z_noise + 0) - a) * 0.125; // Should be vectorized.
                    let f = (terrain.get(x_noise + 0, y_noise + 1, z_noise + 1) - b) * 0.125;
                    let g = (terrain.get(x_noise + 1, y_noise + 1, z_noise + 0) - c) * 0.125;
                    let h = (terrain.get(x_noise + 1, y_noise + 1, z_noise + 1) - d) * 0.125;

                    for y_index in 0..NOISE_REAL_HEIGHT_STRIDE {

                        let y = y_noise * NOISE_REAL_HEIGHT_STRIDE + y_index;
                        
                        let ca = (c - a) * 0.25;
                        let db = (d - b) * 0.25;

                        let mut a0 = a;
                        let mut b0 = b;

                        for x_index in 0..NOISE_REAL_WIDTH_STRIDE {

                            let x = x_noise * NOISE_REAL_WIDTH_STRIDE + x_index;

                            let b0a0 = (b0 - a0) * 0.25;
                            let mut a00 = a0;

                            for z_index in 0..NOISE_REAL_WIDTH_STRIDE {

                                let z = z_noise * NOISE_REAL_WIDTH_STRIDE + z_index;
                                let t = temperature.get(x, 0, z);

                                let mut id = block::AIR;

                                if y < 64 {
                                    id = if t < 0.5 && y == 63 {
                                        block::ICE
                                    } else {
                                        block::WATER_STILL
                                    };
                                }
                                
                                if a00 > 0.0 {
                                    id = block::STONE;
                                }

                                // Chunk should be empty by default, so we ignore if air.
                                if id != block::AIR {
                                    chunk.set_block(IVec3::new(x as i32, y as i32, z as i32), id, 0);
                                }

                                a00 += b0a0;

                            }

                            a0 += ca;
                            b0 += db;

                        }

                        a += e;
                        b += f;
                        c += g;
                        d += h;

                    }

                }
            }
        }

    }

    /// Generate the primitive terrain of the chunk.
    fn gen_surface(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut OverworldState, rand: &mut JavaRandom) {

        let sand = &mut state.sand;
        let gravel = &mut state.gravel;
        let thickness = &mut state.thickness;

        let offset = DVec3::new((cx * 16) as f64, (cz * 16) as f64, 0.0);
        let scale = 1.0 / 32.0;
        let sea_level = 64;

        self.sand_gravel_noise.gen_3d(sand, offset, DVec3::new(scale, scale, 1.0));
        self.sand_gravel_noise.gen_2d(gravel, offset.truncate(), DVec2::new(scale, scale));
        self.thickness_noise.gen_3d(thickness, offset, DVec3::splat(scale * 2.0));

        // NOTE: Order of iteration is really important for random parity.
        for z in 0usize..16 {
            for x in 0usize..16 {

                let mut pos = IVec3::new(x as i32, 0, z as i32);

                let biome = chunk.get_biome(pos);
                let have_sand = sand.get(x, z, 0) + rand.next_double() * 0.2 > 0.0;
                let have_gravel = gravel.get(x, 0, z) + rand.next_double() * 0.2 > 3.0;
                let thickness = (thickness.get(x, z, 0) / 3.0 + 3.0 + rand.next_double() * 0.25) as i32;

                let (
                    biome_top_id, 
                    biome_filler_id
                ) = match biome {
                    Biome::Desert |
                    Biome::IceDesert => (block::SAND, block::SAND),
                    _ => (block::GRASS, block::DIRT),
                };

                let mut top_id = biome_top_id;
                let mut filler_id = biome_filler_id;
                let mut remaining_thickness = -1;

                for y in (0..128).rev() {

                    pos.y = y;

                    if y <= rand.next_int_bounded(5) {
                        chunk.set_block(pos, block::BEDROCK, 0);
                        continue;
                    }

                    let (prev_id, _) = chunk.get_block(pos);

                    if prev_id == block::AIR {
                        remaining_thickness = -1;
                    } else if prev_id == block::STONE {

                        if remaining_thickness == -1 {
                            
                            // No surface yet, initialize it.
                            if thickness <= 0 {
                                top_id = block::AIR;
                                filler_id = block::STONE;
                            } else if y >= sea_level - 4 && y <= sea_level + 1 {
                                
                                top_id = biome_top_id;
                                filler_id = biome_filler_id;

                                if have_sand {
                                    top_id = block::SAND;
                                    filler_id = block::SAND;
                                } else if have_gravel {
                                    top_id = block::AIR;
                                    filler_id = block::GRAVEL;
                                }

                            }

                            if y < sea_level && top_id == block::AIR {
                                top_id = block::WATER_STILL;
                            }

                            remaining_thickness = thickness;

                            if y >= sea_level - 1 {
                                chunk.set_block(pos, top_id, 0);
                            } else {
                                chunk.set_block(pos, filler_id, 0);
                            }

                        } else if remaining_thickness > 0 {

                            chunk.set_block(pos, filler_id, 0);

                            remaining_thickness -= 1;
                            if remaining_thickness == 0 && filler_id == block::SAND {
                                remaining_thickness = rand.next_int_bounded(4);
                                filler_id = block::SANDSTONE;
                            }

                        }

                    }

                }

            }
        }

    }

    // Generate chunk carving (only caves for beta 1.7.3).
    fn gen_carving(&self, cx: i32, cz: i32, chunk: &mut Chunk) {
        CaveGenerator::new(8).generate(cx, cz, chunk, self.seed);
    }

}

impl ChunkGenerator for OverworldGenerator {

    type State = OverworldState;

    fn gen_biomes(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut Self::State) {
        self.gen_biomes(cx, cz, chunk, state);
    }

    fn gen_terrain(&self, cx: i32, cz: i32, chunk: &mut Chunk, state: &mut Self::State) {

        let chunk_seed = i64::wrapping_add(
            (cx as i64).wrapping_mul(341873128712), 
            (cz as i64).wrapping_mul(132897987541));
        
        let mut rand = JavaRandom::new(chunk_seed);

        self.gen_biomes(cx, cz, chunk, state);
        self.gen_terrain(cx, cz, chunk, state);
        self.gen_surface(cx, cz, chunk, state, &mut rand);
        self.gen_carving(cx, cz, chunk);

        chunk.recompute_all_height();

    }

    fn gen_features(&self, cx: i32, cz: i32, world: &mut World, state: &mut Self::State) {

        let pos = IVec3::new(cx * 16, 0, cz * 16);
        let biome = self.get_biome(pos.x + 16, pos.z + 16);

        // Start by calculating the chunk seed from chunk coordinates and world seed.
        let mut rand = JavaRandom::new(self.seed);

        let x_mul = rand.next_long().wrapping_div(2).wrapping_mul(2).wrapping_add(1);
        let z_mul = rand.next_long().wrapping_div(2).wrapping_mul(2).wrapping_add(1);

        let chunk_seed = i64::wrapping_add(
            (cx as i64).wrapping_mul(x_mul), 
            (cz as i64).wrapping_mul(z_mul)
        ) ^ self.seed;

        rand.set_seed(chunk_seed);

        // if cx == 0 && cz == 2 {
        //     println!("debugging chunk {cx}/{cz} biome: {biome:?}");
        // }

        // Function to pick a uniform random position offset.
        #[inline(always)]
        fn next_offset(rand: &mut JavaRandom, max_y: i32, offset_xz: i32) -> IVec3 {
            IVec3 {
                x: rand.next_int_bounded(16) + offset_xz,
                y: rand.next_int_bounded(max_y),
                z: rand.next_int_bounded(16) + offset_xz,
            }
        }

        // Water lakes...
        if rand.next_int_bounded(4) == 0 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            LakeGenerator::new(block::WATER_STILL).generate(world, pos, &mut rand);
        }

        // Lava lakes...
        if rand.next_int_bounded(8) == 0 {

            let pos = pos + IVec3 {
                x: rand.next_int_bounded(16) + 8,
                y: {
                    let v = rand.next_int_bounded(120);
                    rand.next_int_bounded(v + 8)
                },
                z: rand.next_int_bounded(16) + 8,
            };

            if pos.y < 64 || rand.next_int_bounded(10) == 0 {
                LakeGenerator::new(block::LAVA_STILL).generate(world, pos, &mut rand);
            }

        }

        // Mob dungeons...
        for _ in 0..8 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            DungeonGenerator::new().generate(world, pos, &mut rand);
        }

        // Clay veins (only in water).
        for _ in 0..10 {
            let pos = pos + next_offset(&mut rand, 128, 0);
            if world.get_block_material(pos) == Material::Water {
                VeinGenerator::new_clay(32).generate(world, pos, &mut rand);
            }
        }

        // Dirt veins.
        for _ in 0..20 {
            let pos = pos + next_offset(&mut rand, 128, 0);
            VeinGenerator::new_ore(block::DIRT, 32).generate(world, pos, &mut rand);
        }

        // Gravel veins.
        for _ in 0..10 {
            let pos = pos + next_offset(&mut rand, 128, 0);
            VeinGenerator::new_ore(block::GRAVEL, 32).generate(world, pos, &mut rand);
        }

        // Coal veins.
        for _ in 0..20 {
            let pos = pos + next_offset(&mut rand, 128, 0);
            VeinGenerator::new_ore(block::COAL_ORE, 16).generate(world, pos, &mut rand);
        }

        // Iron veins.
        for _ in 0..20 {
            let pos = pos + next_offset(&mut rand, 64, 0);
            VeinGenerator::new_ore(block::IRON_ORE, 8).generate(world, pos, &mut rand);
        }

        // Gold veins.
        for _ in 0..2 {
            let pos = pos + next_offset(&mut rand, 32, 0);
            VeinGenerator::new_ore(block::GOLD_ORE, 8).generate(world, pos, &mut rand);
        }

        // Redstone veins.
        for _ in 0..8 {
            let pos = pos + next_offset(&mut rand, 16, 0);
            VeinGenerator::new_ore(block::REDSTONE_ORE, 7).generate(world, pos, &mut rand);
        }

        // Diamond veins.
        for _ in 0..1 {
            let pos = pos + next_offset(&mut rand, 16, 0);
            VeinGenerator::new_ore(block::DIAMOND_ORE, 7).generate(world, pos, &mut rand);
        }

        // Lapis veins.
        for _ in 0..1 {
            
            let pos = pos + IVec3 {
                x: rand.next_int_bounded(16),
                y: rand.next_int_bounded(16) + rand.next_int_bounded(16),
                z: rand.next_int_bounded(16),
            };

            VeinGenerator::new_ore(block::LAPIS_ORE, 6).generate(world, pos, &mut rand);

        }

        // Trees, depending on biome and feature noise.
        let feature_noise = self.feature_noise.gen_2d_point(pos.xz().as_dvec2() * 0.5);
        let base_tree_count  = ((feature_noise / 8.0 + rand.next_double() * 4.0 + 4.0) / 3.0) as i32;
        let mut tree_count = 0;

        if rand.next_int_bounded(10) == 0 {
            tree_count += 1;
        }

        match biome {
            Biome::Taiga |
            Biome::RainForest |
            Biome::Forest => tree_count += base_tree_count + 5,
            Biome::SeasonalForest => tree_count += base_tree_count + 2,
            Biome::Desert |
            Biome::Tundra |
            Biome::Plains => tree_count -= 20,
            _ => {}
        }

        // if cx == 0 && cz == 2 {
        //     println!("tree_count: {tree_count}");
        // }

        if tree_count > 0 {
            for _ in 0..tree_count {

                let mut pos = pos + IVec3 {
                    x: rand.next_int_bounded(16) + 8,
                    y: 0,
                    z: rand.next_int_bounded(16) + 8,
                };

                pos.y = world.get_height(pos).unwrap() as i32;

                let mut gen = match biome {
                    Biome::Taiga => {
                        if rand.next_int_bounded(3) == 0 {
                            TreeGenerator::new_spruce1()
                        } else {
                            TreeGenerator::new_spruce2()
                        }
                    }
                    Biome::Forest => {
                        if rand.next_int_bounded(5) == 0 {
                            TreeGenerator::new_birch()
                        } else if rand.next_int_bounded(3) == 0 {
                            TreeGenerator::new_big_natural()
                        } else {
                            TreeGenerator::new_oak()
                        }
                    }
                    Biome::RainForest => {
                        if rand.next_int_bounded(3) == 0 {
                            TreeGenerator::new_big_natural()
                        } else {
                            TreeGenerator::new_oak()
                        }
                    }
                    _ => {
                        if rand.next_int_bounded(10) == 0 {
                            TreeGenerator::new_big_natural()
                        } else {
                            TreeGenerator::new_oak()
                        }
                    }
                };

                gen.generate(world, pos, &mut rand);
                
            }
        }

        // if cx == 0 && cz == 2 {
        //     println!("next float: {}", rand.next_float());
        // }

        // Dandelion patches.
        let dandelion_count = match biome {
            Biome::Forest => 2,
            Biome::Taiga => 2,
            Biome::SeasonalForest => 4,
            Biome::Plains => 3,
            _ => 0,
        };

        for _ in 0..dandelion_count {
            let pos = pos + next_offset(&mut rand, 128, 8);
            PlantGenerator::new_flower(block::DANDELION).generate(world, pos, &mut rand);
        }

        // Tall grass patches.
        let tall_grass_count = match biome {
            Biome::Forest => 2,
            Biome::RainForest => 10,
            Biome::SeasonalForest => 2,
            Biome::Taiga => 1,
            Biome::Plains => 10,
            _ => 0,
        };

        for _ in 0..tall_grass_count {

            let mut metadata = 1;
            if biome == Biome::RainForest && rand.next_int_bounded(3) != 0 {
                metadata = 2;
            }

            let pos = pos + next_offset(&mut rand, 128, 8);
            PlantGenerator::new_tall_grass(metadata).generate(world, pos, &mut rand);

        }

        // Dead bush in deserts.
        if biome == Biome::Desert {
            for _ in 0..2 {
                let pos = pos + next_offset(&mut rand, 128, 8);
                PlantGenerator::new_dead_bush().generate(world, pos, &mut rand);
            }
        }

        // Poppy.
        if rand.next_int_bounded(2) == 0 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            PlantGenerator::new_flower(block::POPPY).generate(world, pos, &mut rand);
        }

        // Brown mushroom.
        if rand.next_int_bounded(4) == 0 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            PlantGenerator::new_flower(block::BROWN_MUSHROOM).generate(world, pos, &mut rand);
        }

        // Red mushroom.
        if rand.next_int_bounded(8) == 0 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            PlantGenerator::new_flower(block::RED_MUSHROOM).generate(world, pos, &mut rand);
        }

        // Sugar canes.
        for _ in 0..10 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            SugarCanesGenerator::new().generate(world, pos, &mut rand);
        }

        // Pumpkin.
        if rand.next_int_bounded(32) == 0 {
            let pos = pos + next_offset(&mut rand, 128, 8);
            PumpkinGenerator::new().generate(world, pos, &mut rand);
        }

        // Cactus.
        if biome == Biome::Desert {
            for _ in 0..10 {
                let pos = pos + next_offset(&mut rand, 128, 8);
                CactusGenerator::new().generate(world, pos, &mut rand);
            }
        }

        // Water sources.
        for _ in 0..50 {

            let pos = pos + IVec3 {
                x: rand.next_int_bounded(16) + 8,
                y: {
                    let v = rand.next_int_bounded(120);
                    rand.next_int_bounded(v + 8)
                },
                z: rand.next_int_bounded(16) + 8,
            };

            LiquidGenerator::new(block::WATER_MOVING).generate(world, pos, &mut rand);

        }

        // Lava sources.
        for _ in 0..20 {

            let pos = pos + IVec3 {
                x: rand.next_int_bounded(16) + 8,
                y: {
                    let v = rand.next_int_bounded(112);
                    let v = rand.next_int_bounded(v + 8);
                    rand.next_int_bounded(v + 8)
                },
                z: rand.next_int_bounded(16) + 8,
            };

            LiquidGenerator::new(block::LAVA_MOVING).generate(world, pos, &mut rand);

        }

        // Finally add snow layer if cold enought.
        let offset = DVec2::new((pos.x + 8) as f64, (pos.y + 8) as f64);
        let temperature = &mut state.temperature;
        let biome = &mut state.biome;
        self.temperature_noise.gen_weird_2d(temperature, offset,TEMPERATURE_SCALE, TEMPERATURE_FREQ_FACTOR);
        self.biome_noise.gen_weird_2d(biome, offset, BIOME_SCALE, BIOME_FREQ_FACTOR);

        for dx in 0usize..16 {
            for dz in 0usize..16 {

                let snow_pos = pos + IVec3 {
                    x: dx as i32,
                    y: 0,
                    z: dz as i32,
                };

                // Find highest block and set pos.y.

                let temp = temperature.get(dx, 0, dz) - (snow_pos.y - 64) as f64 / 64.0 * 0.3;
                if temp < 0.5 && snow_pos.y > 0 && snow_pos.y < 128 &&  world.is_block_air(snow_pos) {
                    let material = world.get_block_material(snow_pos - IVec3::Y);
                    if material.is_solid() && material != Material::Ice {
                        world.set_block(snow_pos, block::SNOW, 0);
                    }
                }

            }
        }

        // TODO: This is temporary code to avoid light bugs at generation, but this
        // considerably slows down the feature generation (that is currently 
        // single-threaded).
        world.tick_light(usize::MAX);

    }

}
