//! Various geometry utilities that completes the [`glam`] math crate.

use std::ops::{Add, AddAssign, Sub, SubAssign, BitOr, BitOrAssign};
use std::fmt;

use glam::{DVec3, IVec3};


/// An axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoundingBox {
    pub min: DVec3,
    pub max: DVec3,
}

impl BoundingBox {

    pub const CUBE: Self = Self { min: DVec3::ZERO, max: DVec3::ONE };

    /// Construct a new bounding box from the minimum and maximum points.
    pub const fn new(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min: DVec3::new(min_x, min_y, min_z),
            max: DVec3::new(max_x, max_y, max_z),
        }
    }

    /// Calculate the size of this bounding box.
    pub fn size(self) -> DVec3 {
        self.max - self.min
    }

    pub fn size_x(self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn size_y(self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn size_z(self) -> f64 {
        self.max.z - self.min.z
    }

    /// Calculate the center of the bounding box.
    pub fn center(self) -> DVec3 {
        (self.min + self.max) / 2.0
    }

    /// Calculate the X center of the bounding box.
    pub fn center_x(self) -> f64 {
        (self.min.x + self.max.x) / 2.0
    }

    /// Calculate the Y center of the bounding box.
    pub fn center_y(self) -> f64 {
        (self.min.y + self.max.y) / 2.0
    }

    /// Calculate the Z center of the bounding box.
    pub fn center_z(self) -> f64 {
        (self.min.z + self.max.z) / 2.0
    }

    /// Expand this bounding box in all direction by the given delta.
    pub fn inflate(self, delta: DVec3) -> Self {
        Self {
            min: self.min - delta,
            max: self.max + delta,
        }
    }

    /// Offset this bounding box' coordinates by the given delta.
    pub fn offset(self, delta: DVec3) -> Self {
        Self {
            min: self.min + delta,
            max: self.max + delta,
        }
    }

    /// Expand this bounding box by the given delta, only in the delta's direction.
    pub fn expand(mut self, delta: DVec3) -> Self {

        if delta.x < 0.0 {
            self.min.x += delta.x;
        } else if delta.x > 0.0 {
            self.max.x += delta.x;
        }

        if delta.y < 0.0 {
            self.min.y += delta.y;
        } else if delta.y > 0.0 {
            self.max.y += delta.y;
        }

        if delta.z < 0.0 {
            self.min.z += delta.z;
        } else if delta.z > 0.0 {
            self.max.z += delta.z;
        }

        self

    }

    /// Return true if this bounding box intersects with the given one.
    pub fn intersects(self, other: Self) -> bool {
        other.max.x > self.min.x && other.min.x < self.max.x &&
        other.max.y > self.min.y && other.min.y < self.max.y &&
        other.max.z > self.min.z && other.min.z < self.max.z
    }

    /// Return true if this bounding box intersects with the given one on the X axis.
    pub fn intersects_x(self, other: Self) -> bool {
        other.max.x > self.min.x && other.min.x < self.max.x
    }

    /// Return true if this bounding box intersects with the given one on the Y axis.
    pub fn intersects_y(self, other: Self) -> bool {
        other.max.y > self.min.y && other.min.y < self.max.y
    }

    /// Return true if this bounding box intersects with the given one on the Z axis.
    pub fn intersects_z(self, other: Self) -> bool {
        other.max.z > self.min.z && other.min.z < self.max.z
    }

    /// Return true if this bounding box contains the given point.
    pub fn contains(self, point: DVec3) -> bool {
        point.x > self.min.x && point.x < self.max.x &&
        point.y > self.min.y && point.y < self.max.y &&
        point.z > self.min.z && point.z < self.max.z
    }

    /// Return true if the point is contained in this bounding box on Y/Z axis only.
    pub fn contains_yz(self, point: DVec3) -> bool {
        point.y >= self.min.y && point.y <= self.max.y && 
        point.z >= self.min.z && point.z <= self.max.z
    }

    /// Return true if the point is contained in this bounding box on X/Z axis only.
    pub fn contains_xz(self, point: DVec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x && 
        point.z >= self.min.z && point.z <= self.max.z
    }

    /// Return true if the point is contained in this bounding box on X/Y axis only.
    pub fn contains_xy(self, point: DVec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x && 
        point.y >= self.min.y && point.y <= self.max.y
    }

    /// Simulate an offset of the given bounding box by the given delta, but with this 
    /// bounding box potentially colliding with it in the way, this function will return 
    /// the new delta that avoid this collision.
    pub fn calc_x_delta(self, other: Self, mut dx: f64) -> f64 {
        if other.max.y > self.min.y && other.min.y < self.max.y {
            if other.max.z > self.min.z && other.min.z < self.max.z {
                if dx > 0.0 && other.max.x <= self.min.x {
                    dx = dx.min(self.min.x - other.max.x);
                } else if dx < 0.0 && other.min.x >= self.max.x {
                    dx = dx.max(self.max.x - other.min.x);
                }
            }
        }
        dx
    }

    /// Simulate an offset of the given bounding box by the given delta, but with this 
    /// bounding box potentially colliding with it in the way, this function will return 
    /// the new delta that avoid this collision.
    pub fn calc_y_delta(self, other: Self, mut dy: f64) -> f64 {
        if other.max.x > self.min.x && other.min.x < self.max.x {
            if other.max.z > self.min.z && other.min.z < self.max.z {
                if dy > 0.0 && other.max.y <= self.min.y {
                    dy = dy.min(self.min.y - other.max.y);
                } else if dy < 0.0 && other.min.y >= self.max.y {
                    dy = dy.max(self.max.y - other.min.y);
                }
            }
        }
        dy
    }

    /// Simulate an offset of the given bounding box by the given delta, but with this 
    /// bounding box potentially colliding with it in the way, this function will return 
    /// the new delta that avoid this collision.
    pub fn calc_z_delta(self, other: Self, mut dz: f64) -> f64 {
        if other.max.x > self.min.x && other.min.x < self.max.x {
            if other.max.y > self.min.y && other.min.y < self.max.y {
                if dz > 0.0 && other.max.z <= self.min.z {
                    dz = dz.min(self.min.z - other.max.z);
                } else if dz < 0.0 && other.min.z >= self.max.z {
                    dz = dz.max(self.max.z - other.min.z);
                }
            }
        }
        dz
    }

    /// Compute an intersection of a ray into this bounding box. If this ray intersects
    /// this box, the new vector that hit the first face is returned.
    pub fn calc_ray_trace(self, origin: DVec3, ray: DVec3) -> Option<(DVec3, Face)> {

        if ray.x * ray.x >= 1e-7 {

            let (factor, face) =
            if ray.x > 0.0 { // We can collide only with NegX face.
                ((self.min.x - origin.x) / ray.x, Face::NegX)
            } else { // We can collide only with PosX face.
                ((self.max.x - origin.x) / ray.x, Face::PosX)
            };

            let point = origin + ray * factor;
            if self.contains_yz(point) {
                return Some((point - origin, face))
            }

        }

        if ray.y * ray.y >= 1e-7 {

            let (factor, face) =
            if ray.y > 0.0 { // We can collide only with NegY face.
                ((self.min.y - origin.y) / ray.y, Face::NegY)
            } else { // We can collide only with PosY face.
                ((self.max.y - origin.y) / ray.y, Face::PosY)
            };

            let point = origin + ray * factor;
            if self.contains_xz(point) {
                return Some((point - origin, face))
            }

        }

        if ray.z * ray.z >= 1e-7 {

            let (factor, face) =
            if ray.z > 0.0 { // We can collide only with NegZ face.
                ((self.min.z - origin.z) / ray.z, Face::NegZ)
            } else { // We can collide only with PosZ face.
                ((self.max.z - origin.z) / ray.z, Face::PosZ)
            };

            let point = origin + ray * factor;
            if self.contains_xy(point) {
                return Some((point - origin, face))
            }

        }

        None

    }

}

impl Add<DVec3> for BoundingBox {
    type Output = BoundingBox;
    #[inline]
    fn add(self, rhs: DVec3) -> Self::Output {
        self.offset(rhs)
    }
}

impl AddAssign<DVec3> for BoundingBox {
    #[inline]
    fn add_assign(&mut self, rhs: DVec3) {
        *self = self.offset(rhs);
    }
}

impl Sub<DVec3> for BoundingBox {
    type Output = BoundingBox;
    #[inline]
    fn sub(self, rhs: DVec3) -> Self::Output {
        self.offset(-rhs)
    }
}

impl SubAssign<DVec3> for BoundingBox {
    #[inline]
    fn sub_assign(&mut self, rhs: DVec3) {
        *self = self.offset(-rhs);
    }
}

// The bit or operator can be used to make a union of two bounding boxes.
impl BitOr<BoundingBox> for BoundingBox {
    type Output = BoundingBox;
    #[inline]
    fn bitor(self, rhs: BoundingBox) -> Self::Output {
        BoundingBox {
            min: self.min.min(rhs.min),
            max: self.max.max(rhs.max),
        }
    }
}

impl BitOrAssign for BoundingBox {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl fmt::Display for BoundingBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.min, self.max)
    }
}


/// Represent a facing in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Face {
    NegY = 0,
    PosY = 1,
    NegZ = 2,
    PosZ = 3,
    NegX = 4,
    PosX = 5,
}

impl Face {

    /// Array containing all 6 faces.
    pub const ALL: [Self; 6] = [Self::NegY, Self::PosY, Self::NegZ, Self::PosZ, Self::NegX, Self::PosX];
    /// Array containing all 4 horizontal faces.
    pub const HORIZONTAL: [Self; 4] = [Self::NegZ, Self::PosZ, Self::NegX, Self::PosX];

    /// Get a face from the yaw component of a look only.
    #[inline]
    pub fn from_yaw(yaw: f32) -> Self {
        match ((yaw * 4.0 / std::f32::consts::TAU + 0.5).floor() as i32) & 3 {
            0 => Self::PosZ,
            1 => Self::NegX,
            2 => Self::NegZ,
            3 => Self::PosX,
            _ => unreachable!()
        }
    }

    /// Get a face from the yaw and pitch components of a look.
    #[inline]
    pub fn from_look(yaw: f32, pitch: f32) -> Self {
        if pitch < -std::f32::consts::FRAC_PI_4 {
            Self::PosY
        } else if pitch > std::f32::consts::FRAC_PI_4 {
            Self::NegY
        } else {
            Self::from_yaw(yaw)
        }
    }

    #[inline]
    pub fn is_pos(self) -> bool {
        matches!(self, Face::PosX | Face::PosY | Face::PosZ)
    }

    #[inline]
    pub fn is_neg(self) -> bool {
        matches!(self, Face::NegX | Face::NegY | Face::NegZ)
    }

    #[inline]
    pub fn is_y(self) -> bool {
        matches!(self, Face::NegY | Face::PosY)
    }

    #[inline]
    pub fn is_x(self) -> bool {
        matches!(self, Face::NegX | Face::PosX)
    }

    #[inline]
    pub fn is_z(self) -> bool {
        matches!(self, Face::NegZ | Face::PosZ)
    }

    /// Get the axis (X, Y, Z) index for that face when using `glam` vectors.
    #[inline]
    pub fn axis_index(self) -> usize {
        match self {
            Face::NegY |
            Face::PosY => 1,
            Face::NegZ |
            Face::PosZ => 2,
            Face::NegX |
            Face::PosX => 0,
        }
    }

    /// Get the opposite face.
    #[inline]
    pub fn opposite(self) -> Self {
        match self {
            Face::NegY => Face::PosY,
            Face::PosY => Face::NegY,
            Face::NegZ => Face::PosZ,
            Face::PosZ => Face::NegZ,
            Face::NegX => Face::PosX,
            Face::PosX => Face::NegX,
        }
    }

    /// Rotate this face horizontally to right, Y faces don't change.
    #[inline]
    pub fn rotate_right(self) -> Self {
        match self {
            Face::NegZ => Face::PosX,
            Face::PosX => Face::PosZ,
            Face::PosZ => Face::NegX,
            Face::NegX => Face::NegZ,
            _ => self
        }
    }

    /// Rotate this face horizontally to left, Y faces don't change.
    #[inline]
    pub fn rotate_left(self) -> Self {
        match self {
            Face::NegZ => Face::NegX,
            Face::NegX => Face::PosZ,
            Face::PosZ => Face::PosX,
            Face::PosX => Face::NegZ,
            _ => self
        }
    }

    /// Get the delta vector for this face.
    #[inline]
    pub fn delta(self) -> IVec3 {
        match self {
            Face::NegY => IVec3::NEG_Y,
            Face::PosY => IVec3::Y,
            Face::NegZ => IVec3::NEG_Z,
            Face::PosZ => IVec3::Z,
            Face::NegX => IVec3::NEG_X,
            Face::PosX => IVec3::X,
        }
    }

    /// Extrude a face and form a bounding box. The face is extruded toward the opposite
    /// face. The given inset allows shrinking the face toward the center axis.
    #[inline]
    pub fn extrude(self, inset: f64, depth: f64) -> BoundingBox {
        let pos = inset;
        let neg = 1.0 - inset;
        match self {
            Face::NegY => BoundingBox::new(pos, 0.0, pos, neg, depth, neg),
            Face::PosY => BoundingBox::new(pos, 1.0 - depth, pos, neg, 1.0, neg),
            Face::NegZ => BoundingBox::new(pos, pos, 0.0, neg, neg, depth),
            Face::PosZ => BoundingBox::new(pos, pos, 1.0 - depth, neg, neg, 1.0),
            Face::NegX => BoundingBox::new(0.0, pos, pos, depth, neg, neg),
            Face::PosX => BoundingBox::new(1.0 - depth, pos, pos, 1.0, neg, neg),
        }
    }

}


/// A set of unique faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FaceSet {
    /// Presence of face are encoded bit by bit, the index of each face is the value of 
    /// their enumeration discriminant.
    inner: u8,
}

impl FaceSet {

    /// Create a new empty set.
    #[inline]
    pub const fn new() -> Self {
        Self { inner: 0 }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner == 0
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner = 0;
    }

    #[inline]
    pub fn insert(&mut self, face: Face) -> bool {
        let prev = self.inner;
        self.inner |= 1 << face as u8;
        self.inner != prev
    }

    #[inline]
    pub fn remove(&mut self, face: Face) -> bool {
        let prev = self.inner;
        self.inner &= !(1 << face as u8);
        self.inner != prev
    }

    #[inline]
    pub fn contains(&self, face: Face) -> bool {
        self.inner & (1 << face as u8) != 0
    }

    #[inline]
    pub fn contains_x(&self) -> bool {
        const MASK: u8 = (1 << Face::NegX as u8) | (1 << Face::PosX as u8);
        self.inner & MASK != 0
    }

    #[inline]
    pub fn contains_y(&self) -> bool {
        const MASK: u8 = (1 << Face::NegY as u8) | (1 << Face::PosY as u8);
        self.inner & MASK != 0
    }

    #[inline]
    pub fn contains_z(&self) -> bool {
        const MASK: u8 = (1 << Face::NegZ as u8) | (1 << Face::PosZ as u8);
        self.inner & MASK != 0
    }

}

impl FromIterator<Face> for FaceSet {

    #[inline]
    fn from_iter<T: IntoIterator<Item = Face>>(iter: T) -> Self {
        let mut set = FaceSet::new();
        for face in iter {
            set.insert(face);
        }
        set
    }

}
