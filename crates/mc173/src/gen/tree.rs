//! Tree generation functions.

use glam::IVec3;

use crate::rand::JavaRandom;
use crate::world::World;
use crate::block;

use super::FeatureGenerator;


/// A feature generator for simple trees of varying blocks and height.
pub struct SimpleTreeGenerator {
    /// Minimum height for the simple tree.
    min_height: u8,
    /// Metadata to apply to wood and leaves.
    metadata: u8,
}

impl SimpleTreeGenerator {

    #[inline]
    pub fn new(min_height: u8, metadata: u8) -> Self {
        Self {
            min_height,
            metadata,
        }
    }
    
    #[inline]
    pub fn new_oak() -> Self {
        Self::new(4, 0)
    }

    #[inline]
    pub fn new_birch() -> Self {
        Self::new(5, 2)
    }

}

impl FeatureGenerator for SimpleTreeGenerator {
    
    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        let height = rand.next_int_bounded(3) + self.min_height as i32;

        let check_radius = |y| {
            if y == pos.y {
                0
            } else if y >= pos.y + height - 1 {
                2
            } else {
                1
            }
        };

        if !check_tree(world, pos, height, check_radius) {
            return false;
        }

        world.set_block(pos - IVec3::Y, block::DIRT, 0);

        for y in (pos.y + height - 3)..=(pos.y + height) {

            let dy = y - (pos.y + height);  // Delta from top of the tree.
            let radius = 1 - dy / 2;

            for x in pos.x - radius..=pos.x + radius {
                for z in pos.z - radius..=pos.z + radius {
                    let dx = (x - pos.x).abs();
                    let dz = (z - pos.z).abs();
                    if dx != radius || dz != radius || (rand.next_int_bounded(2) != 0 && dy != 0) {
                        let replace_pos = IVec3::new(x, y, z);
                        if !world.is_block_opaque_cube(replace_pos) {
                            world.set_block(replace_pos, block::LEAVES, self.metadata);
                        }
                    }
                }
            }

        }

        for y in pos.y..(pos.y + height) {
            let replace_pos = IVec3::new(pos.x, y, pos.z);
            if let Some((block::AIR | block::LEAVES, _)) = world.get_block(replace_pos) {
                world.set_block(replace_pos, block::LOG, self.metadata);
            }
        }

        true
        
    }

}


/// Generator for big oak trees.
pub struct BigTreeGenerator {
    height_range: i32,
    height_attenuation: f32,
    leaf_density: f32,
    branch_delta_height: i32,
    branch_scale: f32,
    branch_slope: f32,
}

impl FeatureGenerator for BigTreeGenerator {
    
    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        let mut rand = JavaRandom::new(rand.next_long());
        let mut height = rand.next_int_bounded(self.height_range) + 5;

        if !matches!(world.get_block(pos - IVec3::Y), Some((block::GRASS | block::DIRT, _))) {
            return false;
        }

        // Check that we can grow the main branch.
        let main_branch_from = pos;
        let main_branch_to = pos + IVec3::new(0, height, 0);
        match self.check_big_tree_branch(world, main_branch_from, main_branch_to) {
            Some(new_to) => {
                // If the new length is too short, abort.
                if new_to.y - pos.y < 6 {
                    return false;
                } else {
                    height = new_to.y - pos.y;
                }
            }
            None => {}
        }

        // Now we grow the main branch and generate all branches.
        let mut height_attenuated = (height as f32 * self.height_attenuation) as i32;
        if height_attenuated >= height {
            height_attenuated = height - 1;
        }

        // Calculate the maximum nodes count.
        let nodes_per_height = ((1.382 + (self.leaf_density * height as f32 / 13.0).powi(2)) as i32).max(1) as usize;

        // IDEA: Use a thread local vector.
        let mut nodes = Vec::with_capacity(nodes_per_height * height as usize);

        // Current leaf offset, this will be decreased.
        let mut leaf_offset = height - self.branch_delta_height;
        let mut leaf_y = pos.y + leaf_offset;
        let start_y = pos.y + height_attenuated;

        // First node is mandatory and is the center one.
        nodes.push(BigTreeNode {
            pos: IVec3::new(pos.x, leaf_y, pos.z),
            start_y,
        });

        leaf_y -= 1;

        while leaf_offset >= 0 {

            let size = self.calc_big_tree_layer_size(leaf_offset, height);
            if size >= 0.0 {

                for _ in 0..nodes_per_height {

                    let length = self.branch_scale * size as f32 * (rand.next_float() + 0.328);
                    let angle = rand.next_float() * 2.0 * 3.14159;

                    let leaf_x = (length * angle.sin() + pos.x as f32 + 0.5).floor() as i32;
                    let leaf_z = (length * angle.cos() + pos.z as f32 + 0.5).floor() as i32;

                    let leaf_pos = IVec3::new(leaf_x, leaf_y, leaf_z);
                    let leaf_check_pos = leaf_pos + IVec3::new(0, self.branch_delta_height, 0);
                    if self.check_big_tree_branch(world, leaf_pos, leaf_check_pos).is_none() {

                        // We compute the horizontal distance to the leaf from the main 
                        // branch, the branch will start this distance subtracted to leaf Y.
                        // This cause the branch slope to be 45 degrees, we then applies a
                        // slop factor from the config that will eventually reduce or increase
                        // the slope angle.
                        let horiz_dist = (((pos.x as f32 - leaf_x as f32).powi(2) + (pos.z as f32 - leaf_z as f32).powi(2))).sqrt();
                        let leaf_start_delta = horiz_dist * self.branch_slope;

                        // Do not go below global start Y.
                        let leaf_start_y = ((leaf_y as f32 - leaf_start_delta) as i32).min(start_y);
                        
                        let leaf_start_pos = IVec3::new(pos.x, leaf_start_y, pos.z);
                        if self.check_big_tree_branch(world, leaf_start_pos, leaf_pos).is_none() {
                            nodes.push(BigTreeNode {
                                pos: leaf_pos,
                                start_y: leaf_start_y,
                            });
                        }
                        
                    }

                }

            }

            leaf_y -= 1;
            leaf_offset -= 1;

        }

        // Place all the leaves blocks.
        for node in &nodes {
            self.place_big_tree_leaf(world, node.pos);
        }

        // Place the main branch.
        self.place_big_tree_branch(world, pos, pos + IVec3::new(0, height_attenuated, 0));
        
        // Place all branches.
        let min_height = height as f32 * 0.2;
        for node in &nodes {
            if (node.start_y - pos.y) as f32 >= min_height {
                self.place_big_tree_branch(world, IVec3::new(pos.x, node.start_y, pos.z), node.pos);
            }
        }

        true
        
    }

}

impl BigTreeGenerator {

    #[inline]
    pub fn new() -> Self {
        Self { 
            height_range: 12, 
            height_attenuation: 0.618, 
            leaf_density: 1.0,
            branch_delta_height: 4,
            branch_scale: 1.0,
            branch_slope: 0.381,
        }
    }

    /// Create a new big tree generator for natural generation, it has some slight 
    /// modification from the default one, such as the branch delta height.
    #[inline]
    pub fn new_natural() -> Self {
        let mut ret = Self::new();
        ret.branch_delta_height = 5;
        ret
    }

    /// Grow a big tree leaf ball of leaves.
    fn place_big_tree_leaf(&self, world: &mut World, pos: IVec3) {
        for dy in 0..self.branch_delta_height {
            let radius = if dy != 0 && dy != self.branch_delta_height - 1 { 3.0 } else { 2.0 };
            self.place_big_tree_leaf_layer(world, pos + IVec3::new(0, dy, 0), radius);
        }
    }

    /// Grow a single horizontal layer of leaves of given radius.
    fn place_big_tree_leaf_layer(&self, world: &mut World, pos: IVec3, radius: f32) {

        let block_radius = (radius + 0.618) as i32;

        for dx in -block_radius..=block_radius {
            for dz in -block_radius..=block_radius {
                let dist = ((dx.abs() as f32 + 0.5).powi(2) + (dz.abs() as f32 + 0.5).powi(2)).sqrt();
                if dist <= radius {
                    let replace_pos = pos + IVec3::new(dx, 0, dz);
                    if let Some((block::AIR | block::LEAVES, _)) = world.get_block(replace_pos) {
                        world.set_block(replace_pos, block::LEAVES, 0);
                    }
                }
            }
        }

    }

    /// Place a branch from a position to another one.
    fn place_big_tree_branch(&self, world: &mut World, from: IVec3, to: IVec3) {
        for pos in BlockLineIter::new(from, to) {
            world.set_block(pos, block::LOG, 0);
        }
    }

    /// Check a big tree branch, this function returns the first position on the line 
    /// that is not valid for growing a branch.
    /// If none is returned then the branch is fully valid.
    fn check_big_tree_branch(&self, world: &mut World, from: IVec3, to: IVec3) -> Option<IVec3> {

        for pos in BlockLineIter::new(from, to) {
            if !matches!(world.get_block(pos), Some((block::AIR | block::LEAVES, _))) {
                return Some(pos);
            }
        }

        None

    }

    fn calc_big_tree_layer_size(&self, leaf_offset: i32, height: i32) -> f32 {
    
        if (leaf_offset as f64) < (height as f64 * 0.3) {
            return -1.618;  // Seems to be a joke value because it's never used.
        }
    
        let a = height as f32 / 2.0;
        let b = a - leaf_offset as f32;
    
        (if b == 0.0 {
            a
        } else if b.abs() >= a {
            0.0
        } else {
            (a.abs().powi(2) - b.abs().powi(2)).sqrt()
        }) * 0.5
    
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BigTreeNode {
    /// Center of the leaves node.
    pos: IVec3,
    /// Start Y position on the main branch.
    start_y: i32,
}


pub struct Spruce1TreeGenerator(());

impl Spruce1TreeGenerator {
    pub fn new() -> Self {
        Self(())
    }
}

impl FeatureGenerator for Spruce1TreeGenerator {
    
    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        let height = rand.next_int_bounded(5) + 7;
        let leaves_offset = height - rand.next_int_bounded(2) - 3;
        let leaves_height = height - leaves_offset;
        let max_radius = rand.next_int_bounded(leaves_height + 1);

        let leaves_y = pos.y + leaves_offset;
        let check_radius = |y| {
            if y < leaves_y {
                0
            } else {
                max_radius
            }
        };

        if !check_tree(world, pos, height, check_radius) {
            return false;
        }

        world.set_block(pos - IVec3::Y, block::DIRT, 0);

        let mut current_radius = 0;

        for y in leaves_y..=(pos.y + height) {
            
            for x in pos.x - current_radius..=pos.x + current_radius {
                for z in pos.z - current_radius..=pos.z + current_radius {

                    let dx = (x - pos.x).abs();
                    let dz = (z - pos.z).abs();
                    if dx != current_radius || dz != current_radius || current_radius <= 0 {
                        let replace_pos = IVec3::new(x, y, z);
                        if !world.is_block_opaque_cube(replace_pos) {
                            world.set_block(replace_pos, block::LEAVES, 1);
                        }
                    }

                }
            }

            if current_radius >= 1 && y == leaves_y + 1 {
                current_radius -= 1;
            } else if current_radius < max_radius {
                current_radius += 1;
            }

        }

        for y in pos.y..(pos.y + height - 1) {
            let replace_pos = IVec3::new(pos.x, y, pos.z);
            if let Some((block::AIR | block::LEAVES, _)) = world.get_block(replace_pos) {
                world.set_block(replace_pos, block::LOG, 1);
            }
        }

        true

    }

}


/// A generator for a spruce (variation 2) tree.
pub struct Spruce2TreeGenerator(());

impl Spruce2TreeGenerator {
    pub fn new() -> Self {
        Self(())
    }
}

impl FeatureGenerator for Spruce2TreeGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        
        let height = rand.next_int_bounded(4) + 6;
        let leaves_offset = rand.next_int_bounded(2) + 1;
        let leaves_height = height - leaves_offset;
        let max_radius = rand.next_int_bounded(2) + 2;

        let leaves_y = pos.y + leaves_offset;
        let check_radius = |y| {
            if y < leaves_y {
                0
            } else {
                max_radius
            }
        };

        if !check_tree(world, pos, height, check_radius) {
            return false;
        }

        world.set_block(pos - IVec3::Y, block::DIRT, 0);

        let mut current_radius = rand.next_int_bounded(2);
        let mut start_radius = 0;
        let mut global_radius = 1;

        for dy in 0..=leaves_height {

            let y = pos.y + height - dy;

            for x in pos.x - current_radius..=pos.x + current_radius {
                for z in pos.z - current_radius..=pos.z + current_radius {
                    let dx = (x - pos.x).abs();
                    let dz = (z - pos.z).abs();
                    if dx != current_radius || dz != current_radius || current_radius <= 0 {
                        let replace_pos = IVec3::new(x, y, z);
                        if !world.is_block_opaque_cube(replace_pos) {
                            world.set_block(replace_pos, block::LEAVES, 1);
                        }
                    }
                }
            }

            if current_radius >= global_radius {
                current_radius = start_radius;
                start_radius = 1;
                global_radius = max_radius.min(global_radius + 1);
            } else {
                current_radius += 1;
            }

        }

        let log_offset = rand.next_int_bounded(3);
        for y in pos.y..(pos.y + height - log_offset) {
            let replace_pos = IVec3::new(pos.x, y, pos.z);
            if let Some((block::AIR | block::LEAVES, _)) = world.get_block(replace_pos) {
                world.set_block(replace_pos, block::LOG, 1);
            }
        }

        true
        
    }

}


/// A generic tree generator of any type.
pub enum TreeGenerator {
    Simple(SimpleTreeGenerator),
    Big(BigTreeGenerator),
    Spruce1(Spruce1TreeGenerator),
    Spruce2(Spruce2TreeGenerator),
}

impl TreeGenerator {

    #[inline]
    pub fn new_oak() -> Self {
        Self::Simple(SimpleTreeGenerator::new_oak())
    }

    #[inline]
    pub fn new_birch() -> Self {
        Self::Simple(SimpleTreeGenerator::new_birch())
    }

    #[inline]
    pub fn new_big() -> Self {
        Self::Big(BigTreeGenerator::new())
    }

    #[inline]
    pub fn new_big_natural() -> Self {
        Self::Big(BigTreeGenerator::new_natural())
    }

    #[inline]
    pub fn new_spruce1() -> Self {
        Self::Spruce1(Spruce1TreeGenerator::new())
    }

    #[inline]
    pub fn new_spruce2() -> Self {
        Self::Spruce2(Spruce2TreeGenerator::new())
    }

}

impl FeatureGenerator for TreeGenerator {

    fn generate(&mut self, world: &mut World, pos: IVec3, rand: &mut JavaRandom) -> bool {
        match self {
            TreeGenerator::Simple(gen) => gen.generate(world, pos, rand),
            TreeGenerator::Big(gen) => gen.generate(world, pos, rand),
            TreeGenerator::Spruce1(gen) => gen.generate(world, pos, rand),
            TreeGenerator::Spruce2(gen) => gen.generate(world, pos, rand),
        }
    }

}

impl TreeGenerator {

    // Special function for generating a tree from its sapling, this ensure that the
    // sapling remains is the generation fails. This implementation also pass world
    // random for the the randomization of tree.
    pub fn generate_from_sapling(&mut self, world: &mut World, pos: IVec3) -> bool {
        
        let Some((prev_id, prev_metadata)) = world.set_block(pos, block::AIR, 0) else { 
            return false
        };

        let mut rand = world.get_rand_mut().clone();
        let success = if !self.generate(world, pos, &mut rand) {
            world.set_block(pos, prev_id, prev_metadata);
            false
        } else {
            true
        };

        *world.get_rand_mut() = rand;
        success

    }

}


/// Check if a tree can grow based on some common properties.
fn check_tree(
    world: &mut World, 
    pos: IVec3, 
    height: i32,
    check_radius: impl Fn(i32) -> i32,
) -> bool {

    let max_y = pos.y + height + 1;
    if pos.y < 1 || max_y >= 128 {
        return false;
    }

    // NOTE: This also ensure that our chunk is loaded.
    if !matches!(world.get_block(pos - IVec3::Y), Some((block::GRASS | block::DIRT, _))) {
        return false;
    }

    // Just check if there is enough space for the tree to grow.
    // NOTE: Skip the dy == 0 block because its a sapling.
    for y in pos.y..=max_y {
        let check_radius = check_radius(y);
        for x in pos.x - check_radius..=pos.x + check_radius {
            for z in pos.z - check_radius..=pos.z + check_radius {
                if let Some((block::AIR | block::LEAVES, _)) = world.get_block(IVec3::new(x, y, z)) {
                    continue;
                }
                return false;
            }
        }
    }

    true

}


/// Internal iterator for iterating all blocks of a straight line between two points.
#[derive(Default)]
struct BlockLineIter {
    from: IVec3,
    major_axis: usize,
    second_axis: usize,
    third_axis: usize,
    second_ratio: f32,
    third_ratio: f32,
    major_inc: i32,
    major_max: i32,
    major: i32,
}

impl BlockLineIter {

    fn new(from: IVec3, to: IVec3) -> Self {

        let delta = to - from;
        if delta == IVec3::ZERO {
            return Self::default();
        }

        // Find the axis with the maximum delta, our operations will be based on it.
        let major_axis = (0..3).map(|i: usize| (i, delta[i].abs())).max_by_key(|&(_, delta)| delta).unwrap().0;
        let second_axis = (major_axis + 1) % 3;
        let third_axis = (major_axis + 2) % 3;

        let major_delta = delta[major_axis];
        let second_ratio = delta[second_axis] as f32 / major_delta as f32;
        let third_ratio = delta[third_axis] as f32 / major_delta as f32;
        
        let major_inc = major_delta.signum();
        let major_max = major_delta + major_inc;

        Self {
            from,
            major_axis,
            second_axis,
            third_axis,
            second_ratio,
            third_ratio,
            major_inc,
            major_max,
            major: 0,
        }
        
    }

}

impl Iterator for BlockLineIter {

    type Item = IVec3;

    fn next(&mut self) -> Option<Self::Item> {
        if self.major == self.major_max {
            None
        } else {
            let mut pos = IVec3::ZERO;
            // PARITY: The Notchian client adds an offset of 0.5 before floor only when
            // placing logs, but it's incoherent with its checking, we fix this 
            // incoherency here.
            pos[self.major_axis] = self.from[self.major_axis] + self.major;
            pos[self.second_axis] = (self.from[self.second_axis] as f32 + self.major as f32 * self.second_ratio + 0.5).floor() as i32;
            pos[self.third_axis] = (self.from[self.third_axis] as f32 + self.major as f32 * self.third_ratio + 0.5).floor() as i32;
            self.major += self.major_inc;
            Some(pos)
        }
    }

}
