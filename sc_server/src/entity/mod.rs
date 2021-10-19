mod metadata;
mod ty;
mod version;

pub use metadata::Metadata;
pub use ty::{Data, Type};
pub use version::TypeConverter;

use crate::world::World;
use parking_lot::{Mutex, RwLock};
use sc_common::math::{FPos, Vec3};
use std::sync::Arc;

pub mod behavior;

use behavior::Behavior;

#[derive(Debug, Clone)]
struct EntityPos {
  pos: FPos,
  vel: Vec3,

  yaw:   f32,
  pitch: f32,
}

impl EntityPos {
  pub fn new(pos: FPos) -> Self {
    EntityPos { pos, vel: Vec3::new(0.0, 0.0, 0.0), yaw: 0.0, pitch: 0.0 }
  }
}

pub struct Entity {
  /// The unique id for this entity. This is the key used to store entities in
  /// the World.
  eid:    i32,
  /// The position of this entity. Must be valid for all entities.
  pos:    Mutex<EntityPos>,
  /// The type of this entity.
  ty:     Type,
  /// For some entities, such as projectiles, this field is ignored. To make the
  /// entity not disappear when it hits 0 health, overwrite the
  /// `should_despawn` function in `EntityData`.
  health: Mutex<f32>,
  /// The world this entity is in. Used whenever something changes, and nearby
  /// players need to be notified. This can change if the entity is teleported.
  world:  RwLock<Arc<World>>,
  data:   Mutex<Box<dyn Behavior + Send>>,
}

impl Entity {
  /// Creates a new entity, with default functionality. They will take normal
  /// damage, and despawn if their health hits 0. If you want custom
  /// functionality of any kind, call [`new_custom`].
  pub fn new(eid: i32, ty: Type, world: Arc<World>, pos: FPos) -> Self {
    let behavior = behavior::for_entity(ty);
    Entity {
      eid,
      pos: Mutex::new(EntityPos::new(pos)),
      ty,
      health: Mutex::new(behavior.max_health()),
      world: RwLock::new(world),
      data: Mutex::new(behavior),
    }
  }

  /// Creates a new entity, with the given functionality. This value will be
  /// store within the entity until it despawns.
  pub fn new_custom<B: Behavior + Send + 'static>(
    eid: i32,
    ty: Type,
    pos: FPos,
    world: Arc<World>,
    behavior: B,
  ) -> Self {
    Entity {
      eid,
      pos: Mutex::new(EntityPos::new(pos)),
      ty,
      health: Mutex::new(behavior.max_health()),
      world: RwLock::new(world),
      data: Mutex::new(Box::new(behavior)),
    }
  }

  /// Reads this entity's position. This will always be up to date with the
  /// server's known position of this entity. Some clients may be behind this
  /// position (by up to 1/20 of a second).
  pub fn pos(&self) -> FPos {
    self.pos.lock().pos
  }

  /// Returns the unique id for this entity.
  pub fn eid(&self) -> i32 {
    self.eid
  }

  /// Returns this entity's type. This can be used to send spawn packets to
  /// clients.
  pub fn ty(&self) -> Type {
    self.ty
  }

  /// Returns this entity's health.
  pub fn health(&self) -> f32 {
    *self.health.lock()
  }

  /// Returns true if this entity should despawn.
  pub fn should_despawn(&self) -> bool {
    self.data.lock().should_despawn(self.health())
  }

  /// Returns the amount of exp stored in this entity. This is just the amount
  /// for an exp orb, but it is also used to find out how much exp an entity
  /// will drop when killed.
  pub fn exp_count(&self) -> i32 {
    self.data.lock().exp_count()
  }

  /// Sets this entity's velocity. This will send velocity updates to nearby
  /// players, and will affect how the entity moves on the next tick.
  pub fn set_vel(&self, vel: Vec3) {
    self.pos.lock().vel = vel;
    self.world.read().send_entity_vel(self.pos().chunk(), self.eid, vel);
  }

  /// Called 20 times a second. Calling this more/less frequently will break
  /// things.
  pub(crate) fn tick(&self) {
    let mut p = self.pos.lock();
    let old = p.pos;
    let vel = p.vel;
    p.pos += vel;
    // 9.8 m/s ~= 0.5 m/tick. However, minecraft go brrr, and gravity is actually
    // 0.03 b/tick for projectiles, and 0.08 b/tick for living entities.
    p.vel.y -= 0.08;
    p.vel.y *= 0.98;
    self.world.read().send_entity_vel(old.chunk(), self.eid, p.vel);
    self.world.read().send_entity_pos(self.eid, old, p.pos, false);
  }
}
