use sc_common::version::BlockVersion;

pub struct TypeConverter {
  blocks:   &'static [block::Version],
  items:    &'static [item::Version],
  entities: &'static [entity::Version],
}

mod block {
  use sc_common::version::BlockVersion;

  #[derive(Debug)]
  pub struct Version {
    pub to_old: &'static [u32],
    pub to_new: &'static [u32],
    pub ver:    BlockVersion,
  }

  include!(concat!(env!("OUT_DIR"), "/block/version.rs"));
}

mod item {
  use sc_common::version::BlockVersion;

  #[derive(Debug)]
  pub struct Version {
    pub to_old: &'static [(u32, u32)],
    pub to_new: &'static [&'static [u32]],
    pub ver:    BlockVersion,
  }

  include!(concat!(env!("OUT_DIR"), "/item/version.rs"));
}

mod entity {
  use sc_common::version::BlockVersion;

  #[derive(Debug)]
  pub struct Version {
    pub to_old: &'static [u32],
    pub to_new: &'static [u32],
    pub ver:    BlockVersion,
  }

  include!(concat!(env!("OUT_DIR"), "/entity/version.rs"));
}

impl TypeConverter {
  pub fn new() -> Self {
    TypeConverter {
      blocks:   block::generate_versions(),
      items:    item::generate_versions(),
      entities: entity::generate_versions(),
    }
  }
}

impl TypeConverter {
  /// The `id` argument is a block id in the given version. The returned block
  /// id should be the equivalent id in the latest version this server supports.
  /// This should also support passing in the latest version (it should return
  /// the same id).
  pub fn block_to_new(&self, id: u32, ver: BlockVersion) -> u32 {
    // Air always maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.blocks[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
  /// The `id` argument is a block id in the latest version. This function
  /// should return the equivalent block id for the given version. It should
  /// also work when passed the latest version (it should return the same id).
  pub fn block_to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.blocks[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }

  /// Converts an item id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  pub fn item_to_new(&self, id: u32, damage: u32, ver: BlockVersion) -> u32 {
    // Air always maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.items[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => v.get(damage as usize).copied().unwrap_or(0),
      None => 0,
    }
  }
  /// Converts an item id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  pub fn item_to_old(&self, id: u32, ver: BlockVersion) -> (u32, u32) {
    if ver == BlockVersion::latest() {
      return (id, 0);
    }
    match self.items[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => (0, 0),
    }
  }

  /// Converts an entity id into the latest version. It should work the same as
  /// [`block_to_new`](Self::block_to_new).
  pub fn entity_to_new(&self, id: u32, ver: BlockVersion) -> u32 {
    // Air alwas maps to air. Since multiple latest blocks convert to air, we need
    // this check
    if id == 0 {
      return 0;
    }
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.entities[ver.to_index() as usize].to_new.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
  /// Converts an entity id into an id for the given version. It should work the
  /// same as [`block_to_old`](Self::block_to_old).
  pub fn entity_to_old(&self, id: u32, ver: BlockVersion) -> u32 {
    if ver == BlockVersion::latest() {
      return id;
    }
    match self.entities[ver.to_index() as usize].to_old.get(id as usize) {
      Some(v) => *v,
      None => 0,
    }
  }
}
