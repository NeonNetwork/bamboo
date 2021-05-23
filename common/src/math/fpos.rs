use super::{ChunkPos, Pos};
use std::{
  error::Error,
  fmt,
  ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Debug)]
pub struct FPosError {
  pos: FPos,
  msg: String,
}

impl fmt::Display for FPosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "invalid position: {} {}", self.pos, self.msg)
  }
}

impl Error for FPosError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FPos {
  x: f64,
  y: f64,
  z: f64,
}

impl fmt::Display for FPos {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "FPos({} {} {})", self.x, self.y, self.z)
  }
}

impl Default for FPos {
  fn default() -> FPos {
    FPos::new(0.0, 0.0, 0.0)
  }
}

impl FPos {
  /// Creates a new block position. This can be used to find chunk coordinates,
  /// place blocks, or send a position in a packet.
  pub fn new(x: f64, y: f64, z: f64) -> Self {
    FPos { x, y, z }
  }
  /// Returns the X value of the position.
  #[inline(always)]
  pub fn x(&self) -> f64 {
    self.x
  }
  /// Returns the Y value of the position.
  #[inline(always)]
  pub fn y(&self) -> f64 {
    self.y
  }
  /// Returns the Z value of the position.
  #[inline(always)]
  pub fn z(&self) -> f64 {
    self.z
  }
  /// Returns the block that this position is in.
  #[inline(always)]
  pub fn block(&self) -> Pos {
    Pos::new(self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32)
  }
  /// Returns the chunk that this position is in. This is the same as
  /// `self.block().chunk()`.
  #[inline(always)]
  pub fn chunk(&self) -> ChunkPos {
    self.block().chunk()
  }
  /// Creates a new error from this position. This should be used to signify
  /// that an invalid position was passed somewhere.
  pub fn err(&self, msg: String) -> FPosError {
    FPosError { pos: *self, msg }
  }
}

impl Add for FPos {
  type Output = Self;
  fn add(self, other: Self) -> Self {
    Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
  }
}

impl AddAssign for FPos {
  fn add_assign(&mut self, other: Self) {
    self.x += other.x;
    self.y += other.y;
    self.z += other.z;
  }
}

impl Sub for FPos {
  type Output = Self;
  fn sub(self, other: Self) -> Self {
    Self { x: self.x - other.x, y: self.y - other.y, z: self.z - other.z }
  }
}

impl SubAssign for FPos {
  fn sub_assign(&mut self, other: Self) {
    self.x -= other.x;
    self.y -= other.y;
    self.z -= other.z;
  }
}
