//! Path finder utility for world.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::{Sub, Add};

use glam::{IVec3, DVec3};

use crate::block::material::Material;
use crate::geom::BoundingBox;
use crate::world::World;
use crate::block;


/// A path finder on a world.
pub struct PathFinder<'a> {
    /// Back-reference to the world.
    world: &'a World,
    /// The size of the entity (or whatever you want) that should go through the path.
    entity_size: IVec3,
    /// All points allocated by the path finder.
    points: Vec<PathPoint>,
    /// Mapping of points from their block position.
    points_map: HashMap<IVec3, usize>,
    /// A sorted array of points by distance to target, decreasing order. We use 
    /// decreasing order because we'll always take the point with less distance to
    /// target, getting it from the end reduces overhead of the operation because no
    /// other element need to be moved.
    pending: Vec<usize>,
}

#[derive(Default)]
struct PathPoint {
    /// The block position of this path point.
    pos: IVec3,
    /// Total distance of the path containing this point.
    total_distance: f32,
    /// Distance to next point.
    distance_to_next: f32,
    /// A sum of known distance and direct distance to destination point.
    distance_to_target: f32,
    /// True if this point has been the nearest one at some point in path finding.
    is_first: bool,
    /// Previous point index, if currently in the path.
    previous_index: Option<usize>,
    /// True if this point is in the pending list, ordered by its distance to target.
    pending: bool,
}

impl<'a> PathFinder<'a> {

    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            entity_size: IVec3::ONE,
            points: Vec::new(),
            points_map: HashMap::new(),
            pending: Vec::new(),
        }
    }

    fn distance(from: IVec3, to: IVec3) -> f32 {
        to.sub(from).as_vec3().length()
    }

    fn ensure_point(&mut self, pos: IVec3) -> (usize, &mut PathPoint) {
        match self.points_map.entry(pos) {
            Entry::Occupied(o) => {
                let index = *o.into_mut();
                (index, &mut self.points[index])
            }
            Entry::Vacant(v) => {
                let index = self.points.len();
                v.insert(index);
                self.points.push(PathPoint { pos, ..Default::default() });
                (index, &mut self.points[index])
            }
        }
    }

    /// Check clearance of the given position, depending on the current entity size.
    fn check_clearance(&self, pos: IVec3) -> PathClearance {
        
        for (_, block, metadata) in self.world.iter_blocks_in(pos, pos + self.entity_size) {
            match block {
                block::AIR => {}
                block::IRON_DOOR | block::WOOD_DOOR => {
                    if !block::door::is_open(metadata) {
                        return PathClearance::Blocked;
                    }
                }
                _ => {
                    match block::material::get_material(block) {
                        Material::Water => return PathClearance::Water,
                        Material::Lava => return PathClearance::Lava,
                        material => {
                            if material.is_solid() {
                                return PathClearance::Blocked;
                            }
                        }
                    }
                }
            }
        }

        PathClearance::Clear

    }

    /// Find a safe point to path find to in above or below the given position.
    fn find_safe_point(&mut self, mut pos: IVec3, clear: bool) -> Option<usize> {

        let mut ret = None;

        if self.check_clearance(pos) == PathClearance::Clear {
            ret = Some(self.ensure_point(pos).0);
        }

        if ret.is_none() && clear && self.check_clearance(pos + IVec3::Y) == PathClearance::Clear {
            pos.y += 1;
            ret = Some(self.ensure_point(pos).0);
        }

        if let Some(point_index) = &mut ret {

            let mut height = 0;
            while pos.y > 0 {
                
                pos.y -= 1;
                height += 1;

                match self.check_clearance(pos) {
                    PathClearance::Clear => {}
                    PathClearance::Lava => return None,
                    _ => break,
                }
                
                // NOTE: Updating height here is important, because if we get block/water
                // for height == 4, then it'll break and still return a valid point.
                if height >= 4 {
                    return None;
                }

                *point_index = self.ensure_point(pos).0;

            }

        }

        ret

    }

    /// Find path options around the given 'from' position, with a maximum distance.
    fn find_path_options(&mut self, from: IVec3, to: IVec3, dist: f32) -> [Option<usize>; 4] {

        let clear = self.check_clearance(from + IVec3::Y) == PathClearance::Clear;

        let mut ret = [
            self.find_safe_point(from + IVec3::Z, clear),
            self.find_safe_point(from - IVec3::X, clear),
            self.find_safe_point(from + IVec3::X, clear),
            self.find_safe_point(from - IVec3::Z, clear),
        ];

        for option_index in &mut ret {
            if let Some(index) = *option_index {
                let point = &self.points[index];
                // If the point was already selected as a first one or is too far away,
                // remove the option.
                if point.is_first || Self::distance(point.pos, to) >= dist {
                    *option_index = None;
                }
            }
        }

        ret

    }

    /// Ensure that a point (given its index) is present in the pending list.
    fn ensure_pending_point(&mut self, point_index: usize) {

        let point = &mut self.points[point_index];
        let point_distance_to_target = point.distance_to_target;

        // If the point was pending, remove it before inserting again.
        if point.pending {
            let index = self.pending.iter().position(|&index| index == point_index)
                .expect("should be in the pending list");
            self.pending.remove(index);
        }

        // Now we are sure that this point will be pending.
        point.pending = true;

        // CRITICAL: We need to keep the pending list ordered, smaller distance to 
        // target at the end of the list.
        let insert_index = self.pending.binary_search_by(|&index| {
            let distance_to_target = self.points[index].distance_to_target;
            point_distance_to_target.total_cmp(&distance_to_target)
        }).unwrap_or_else(|index| index);

        self.pending.insert(insert_index, point_index);

    }

    /// Find a path in the world from on position to another, with a given maximum 
    /// distance, if no path can be found none is returned. The result also depends on
    /// the entity size, which will determine wether or not the entity can go through
    /// a hole or not.
    pub fn find_path(&mut self, from: IVec3, to: IVec3, entity_size: IVec3, dist: f32) -> Option<Vec<IVec3>> {
        
        // println!("== find_path: from {from}, to {to}, entity_size {entity_size}, dist {dist}");

        self.entity_size = entity_size;

        // Initialize the first point.
        let (from_index, from_point) = self.ensure_point(from);
        from_point.total_distance = 0.0;
        from_point.distance_to_next = Self::distance(from, to);
        from_point.distance_to_target = from_point.distance_to_next;

        // The path contains our first point.
        self.pending.push(from_index);

        let mut near_pos = from;
        let mut near_index = 0;

        while let Some(current_index) = self.pending.pop() {

            let current_point = &mut self.points[current_index];

            // println!("pending count: {}, distance to target: {}, total distance: {}", self.pending.len(), current_point.distance_to_target, current_point.total_distance);

            // When we reach target position, create the path.
            if current_point.pos == to {
                near_index = current_index;
                break;
            }

            if Self::distance(current_point.pos, to) < Self::distance(near_pos, to) {
                near_pos = current_point.pos;
                near_index = current_index;
            }

            current_point.is_first = true;

            let current_pos = current_point.pos;
            let current_total_distance = current_point.total_distance;

            // Try each option to check if this is better than the current one.
            for option in self.find_path_options(current_pos, to, dist) {
                if let Some(option_index) = option {
                    let option_point = &mut self.points[option_index];
                    let added_distance = Self::distance(current_pos, option_point.pos);
                    let new_total_distance = current_total_distance + added_distance;
                    // If the point is not in the path or it is shorter than current one.
                    if !option_point.pending || new_total_distance < current_total_distance {
                        // Update our option point to point to the previous point.
                        option_point.previous_index = Some(current_index);
                        option_point.total_distance = new_total_distance;
                        option_point.distance_to_next = Self::distance(option_point.pos, to);
                        option_point.distance_to_target = option_point.total_distance + option_point.distance_to_next;
                        // If the point was already in the path, we need to resort it, if
                        // the point was not in the path, just add it at the right place.
                        self.ensure_pending_point(option_index);
                    }
                }
            }

        }

        // If we did not find any better point that the initial one, return nothing.
        if near_index == 0 {
            None
        } else {

            let mut ret = Vec::new();

            loop {
                let point = &self.points[near_index];
                ret.push(point.pos);
                if let Some(previous_index) = point.previous_index {
                    near_index = previous_index;
                } else {
                    break;
                }
            }

            // Reset all cache.
            self.points.clear();
            self.points_map.clear();
            self.pending.clear();

            ret.reverse();
            Some(ret)

        }

    }

    /// A specialization or [`find_path`](Self::find_path) to find a path of a moving 
    /// bounding box to a given position. The actual position of the bounding is its 
    /// bottom center.
    pub fn find_path_from_bounding_box(&mut self, from: BoundingBox, to: DVec3, dist: f32) -> Option<Vec<IVec3>> {
        
        // println!("== find_path_from_bounding_box: from {from}, to {to}, dist {dist}");

        let size = from.size();
        let from = from.min.floor().as_ivec3();
        let to = to.sub(DVec3 {
            x: size.x / 2.0,
            y: 0.0,
            z: size.z / 2.0,
        }).floor().as_ivec3();

        self.find_path(from, to, size.add(1.0).floor().as_ivec3(), dist)

    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathClearance {
    Clear,
    Blocked,
    Water,
    Lava,
}
