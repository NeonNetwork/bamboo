pub mod chunk;
pub mod gen;
mod init;
mod players;

use std::{
  collections::HashMap,
  convert::TryInto,
  sync::{
    atomic::{AtomicI32, AtomicU32, Ordering},
    Arc, Mutex as StdMutex, MutexGuard as StdMutexGuard, RwLock,
  },
  thread,
  thread::ThreadId,
  time::{Duration, Instant},
};
use tokio::{
  sync::{mpsc::Sender, Mutex, MutexGuard},
  time,
};
use tonic::{Status, Streaming};

use common::{
  math::{ChunkPos, FPos, Pos, PosError},
  net::cb,
  proto::Packet,
  util::{
    chat::{Chat, Color},
    UUID,
  },
  version::{BlockVersion, ProtocolVersion},
};

use crate::{block, command::CommandTree, entity, item, net::Connection, player::Player, plugin};
use chunk::MultiChunk;
use gen::WorldGen;

pub use players::{PlayersIter, PlayersMap};

// pub struct ChunkRef<'a> {
//   pos:    ChunkPos,
//   // Need to keep this is scope while we mess with the chunk
//   chunks: RwLockReadGuard<'a, HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
// }
//
// impl ChunkRef<'_> {
//   fn lock<'a>(&'a self) -> StdMutexGuard<'a, MultiChunk> {
//     self.chunks.get(&self.pos).unwrap().lock().unwrap()
//   }
// }

pub struct World {
  chunks:           RwLock<HashMap<ChunkPos, Arc<StdMutex<MultiChunk>>>>,
  generators:       RwLock<HashMap<ThreadId, StdMutex<WorldGen>>>,
  players:          Mutex<PlayersMap>,
  eid:              AtomicI32,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
  commands:         CommandTree,
  mspt:             AtomicU32,
  wm:               Arc<WorldManager>,
}

pub struct WorldManager {
  // This will always have at least 1 entry. The world at index 0 is considered the "default"
  // world.
  worlds:           Mutex<Vec<Arc<World>>>,
  block_converter:  Arc<block::TypeConverter>,
  item_converter:   Arc<item::TypeConverter>,
  entity_converter: Arc<entity::TypeConverter>,
  plugins:          Arc<plugin::PluginManager>,
}

impl World {
  pub fn new(
    block_converter: Arc<block::TypeConverter>,
    item_converter: Arc<item::TypeConverter>,
    entity_converter: Arc<entity::TypeConverter>,
    plugins: Arc<plugin::PluginManager>,
    wm: Arc<WorldManager>,
  ) -> Arc<Self> {
    let world = Arc::new(World {
      chunks: RwLock::new(HashMap::new()),
      generators: RwLock::new(HashMap::new()),
      players: Mutex::new(PlayersMap::new()),
      eid: 1.into(),
      block_converter,
      item_converter,
      entity_converter,
      plugins,
      commands: CommandTree::new(),
      mspt: 0.into(),
      wm,
    });
    let w = world.clone();
    tokio::spawn(async move {
      w.init().await;
      w.global_tick_loop().await;
    });
    world
  }
  async fn global_tick_loop(self: Arc<Self>) {
    let mut int = time::interval(Duration::from_millis(50));
    let mut tick = 0;
    loop {
      int.tick().await;
      if tick % 20 == 0 {
        let mut header = Chat::empty();
        let mut footer = Chat::empty();

        header.add("big gaming\n").color(Color::Blue);
        footer.add("\nmspt: ");
        let mspt = self.mspt.swap(0, Ordering::SeqCst) / 20;
        footer.add(format!("{}", mspt)).color(if mspt > 50 {
          Color::Red
        } else if mspt > 20 {
          Color::Gold
        } else if mspt > 10 {
          Color::Yellow
        } else {
          Color::BrightGreen
        });

        let out =
          cb::Packet::PlayerlistHeader { header: header.to_json(), footer: footer.to_json() };
        for p in self.players.lock().await.values() {
          p.conn().send(out.clone()).await;
        }
      }
      tick += 1;
    }
  }
  async fn new_player(self: Arc<Self>, player: Player) {
    let conn = player.clone_conn();
    let player = Arc::new(player);
    {
      let mut p = self.players.lock().await;
      if p.contains_key(&player.id()) {
        player.disconnect("Another player with the same id is already connected!").await;
        return;
      }
      p.insert(player.id(), player.clone());
    }

    // Network recieving task
    let c = conn.clone();
    let p = player.clone();
    let wm = self.wm.clone();
    tokio::spawn(async move {
      c.run(p, wm).await.unwrap();
    });

    // Player tick loop
    tokio::spawn(async move {
      let name = player.username().to_string();
      let id = player.id();
      info!("{} has logged in", name);
      self.player_loop(player, conn).await;
      info!("{} has logged out", name);
      self.players.lock().await.remove(&id);
    });
  }

  async fn player_loop(&self, player: Arc<Player>, conn: Arc<Connection>) {
    let mut int = time::interval(Duration::from_millis(50));
    // Player init
    self.player_init(&player, &conn).await;
    // Player tick loop
    let mut tick = 0;
    loop {
      int.tick().await;
      if conn.closed() {
        // TODO: Close any other tasks for this player
        break;
      }
      let start = Instant::now();
      // Updates the player correctly, and performs collision checks. This also
      // handles new chunks.
      player.tick().await;
      // Do player collision and packets and stuff
      // Once per second, send keep alive packet
      if tick % 20 == 0 {
        conn
          .send(cb::Packet::KeepAlive {
            keep_alive_id_v1_8:    Some(1234556),
            keep_alive_id_v1_12_2: Some(1234556),
          })
          .await;
      }
      tick += 1;
      self.mspt.fetch_add(start.elapsed().as_millis().try_into().unwrap(), Ordering::SeqCst);
    }
  }

  /// Returns a new, unique EID.
  pub fn eid(&self) -> i32 {
    self.eid.fetch_add(1, Ordering::SeqCst)
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn get_block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }
  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn get_item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }
  /// Returns the current entity converter. This can be used to convert old
  /// entity ids to new ones, and vice versa.
  pub fn get_entity_converter(&self) -> &entity::TypeConverter {
    &self.entity_converter
  }
  /// Returns the plugin manager. This is how events can be sent to plugins.
  pub fn get_plugins(&self) -> &plugin::PluginManager {
    &self.plugins
  }
  /// Returns the command tree that the server uses. This can be used to add
  /// custom commands to the server.
  pub fn get_commands(&self) -> &CommandTree {
    &self.commands
  }

  /// Generates a chunk for the given chunk position. This will not store the
  /// chunk, or even look in the chunks table at all. It should be used if you
  /// have a list of chunks to generate, and you would like to generate them in
  /// parallel.
  pub fn pre_generate_chunk(&self, pos: ChunkPos) -> MultiChunk {
    let tid = thread::current().id();
    // We first check (read-only) if we need a world generator for this thread
    if !self.generators.read().unwrap().contains_key(&tid) {
      // If we do, we lock it for writing
      let mut generators = self.generators.write().unwrap();
      // Make sure that the chunk was not written in between locking this chunk
      // Even though we only use this generator on this thread, Rust safety says we
      // need a Mutex here. I could do away with the mutex in unsafe code, but that
      // seems like a pre-mature optimization.
      generators.entry(tid).or_insert_with(|| StdMutex::new(WorldGen::new()));
    }
    let generators = self.generators.read().unwrap();
    let mut lock = generators[&tid].lock().unwrap();
    let mut c = MultiChunk::new(self.block_converter.clone());
    lock.generate(pos, &mut c);
    c
  }

  /// Stores a list of chunks in the internal map. This should be used after
  /// calling [`pre_generate_chunk`](Self::pre_generate_chunk) a number of
  /// times.
  ///
  /// NOTE: This will override pre-existing chunks! This should not be a problem
  /// with multiple threads generating the same chunks, as they have already
  /// done most of the work by the time the override check occurs.
  pub fn store_chunks(&self, chunks: Vec<(ChunkPos, MultiChunk)>) {
    let mut lock = self.chunks.write().unwrap();
    for (pos, c) in chunks {
      lock.insert(pos, Arc::new(StdMutex::new(c)));
    }
  }

  /// This calls f(), and passes it a locked chunk. This will also generate a
  /// new chunk if there is not one stored there.
  ///
  /// I tried to make the chunk a returned value, but that ended up being too
  /// difficult. Since the entire chunks map must be locked for reading, that
  /// read lock must be held while the chunk is in scope. Because of this, you
  /// would have needed to call two functions to get it working. I tried my best
  /// with the [`Deref`](std::ops::Deref) trait, but I couldn't get it to work
  /// the way I liked.
  pub fn chunk<F, R>(&self, pos: ChunkPos, f: F) -> R
  where
    F: FnOnce(StdMutexGuard<MultiChunk>) -> R,
  {
    // We first check (read-only) if we need to generate a new chunk
    if !self.chunks.read().unwrap().contains_key(&pos) {
      // If we do, we lock it for writing
      let mut chunks = self.chunks.write().unwrap();
      // Make sure that the chunk was not written in between locking this chunk
      chunks.entry(pos).or_insert_with(|| Arc::new(StdMutex::new(self.pre_generate_chunk(pos))));
    }
    let chunks = self.chunks.read().unwrap();
    let c = chunks[&pos].lock().unwrap();
    f(c)
  }

  /// This serializes a chunk for the given version. This packet can be sent
  /// directly to a client. Note that on most vanilla versions, sending a chunk
  /// to a client that already has loaded that chunk will cause a memory leak.
  /// Unloading a chunk multiple times will not cause a memory leak. If you are
  /// trying to re-send an entire chunk to a player, make sure to send them an
  /// unload chunk packet first. Use at your own risk!
  pub fn serialize_chunk(&self, pos: ChunkPos, ver: BlockVersion) -> cb::Packet {
    self.chunk(pos, |c| crate::net::serialize::serialize_chunk(pos, &c, ver))
  }

  /// This sets a block within the world. It will return an error if the
  /// position is outside of the world. Unlike
  /// [`MultiChunk::set_type`](chunk::MultiChunk::set_type), this will send
  /// packets to anyone within render distance of the given chunk.
  pub async fn set_block(&self, pos: Pos, ty: &block::Type) -> Result<(), PosError> {
    self.chunk(pos.chunk(), |mut c| c.set_type(pos.chunk_rel(), ty))?;

    for p in self.players().await.iter().in_view(pos.chunk()) {
      p.conn()
        .send(cb::Packet::BlockChange {
          location: pos,
          type_:    self.block_converter.to_old(ty.id(), p.ver().block()) as i32,
        })
        .await;
    }
    Ok(())
  }

  /// This sets a block within the world. This will use the default type of the
  /// given kind. It will return an error if the position is outside of the
  /// world.
  pub async fn set_kind(&self, pos: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.set_block(pos, self.block_converter.get(kind).default_type()).await
  }

  /// Fills the given region with the given block type. Min must be less than or
  /// equal to max. Use [`min_max`](Pos::min_max) to convert two corners of a
  /// cube into a min and max.
  pub async fn fill(&self, min: Pos, max: Pos, ty: &block::Type) -> Result<(), PosError> {
    // Small fills should just send a block update, instead of a multi block change.
    if min == max {
      return self.set_block(min, ty).await;
    }
    let mut blocks_changed = HashMap::new();
    for x in min.chunk_x()..=max.chunk_x() {
      for z in min.chunk_z()..=max.chunk_z() {
        let mut min_x = 0;
        let mut min_z = 0;
        if min.chunk_x() == x {
          min_x = min.chunk_rel_x();
        }
        if min.chunk_z() == z {
          min_z = min.chunk_rel_z();
        }
        let mut max_x = 15;
        let mut max_z = 15;
        if max.chunk_x() == x {
          max_x = max.chunk_rel_x();
        }
        if max.chunk_z() == z {
          max_z = max.chunk_rel_z();
        }

        self.chunk(ChunkPos::new(x, z), |mut c| {
          let mut changes = vec![];
          for x in min_x..=max_x {
            for y in min.y..=max.y {
              for z in min_z..=max_z {
                changes.push(c.get_block(Pos::new(x, y, z)));
              }
            }
          }
          blocks_changed.insert(ChunkPos::new(x, z), changes);
          c.fill(Pos::new(min_x, min.y, min_z), Pos::new(max_x, max.y, max_z), ty)
        })?;
      }
    }

    for x in min.chunk_x()..=max.chunk_x() {
      for z in min.chunk_z()..=max.chunk_z() {
        let pos = ChunkPos::new(x, z);
        let records_v1_8 = vec![];
        let records_v1_16_2 = vec![];
        for p in self.players().await.iter().in_view(pos) {
          if p.ver() >= ProtocolVersion::V1_16_2 {
            p.conn()
              .send(cb::Packet::MultiBlockChange {
                chunk_x_removed_v1_16_2:   None,
                chunk_z_removed_v1_16_2:   None,
                // TODO: Section encoding. Looks like this: ((sectionX & 0x3FFFFF) << 42) |
                // (sectionY & 0xFFFFF) | ((sectionZ & 0x3FFFFF) << 20);
                chunk_coordinates_v1_16_2: Some(vec![]),
                not_trust_edges_v1_16_2:   Some(false),
                records_v1_8:              None,
                // TODO: 1.16 multi block change records
                records_v1_16_2:           Some(records_v1_16_2.clone()),
              })
              .await;
          } else {
            p.conn()
              .send(cb::Packet::MultiBlockChange {
                chunk_x_removed_v1_16_2:   Some(x),
                chunk_z_removed_v1_16_2:   Some(z),
                chunk_coordinates_v1_16_2: None,
                not_trust_edges_v1_16_2:   None,
                records_v1_8:              Some(records_v1_8.clone()),
                records_v1_16_2:           None,
              })
              .await;
          }
        }
      }
    }

    Ok(())
  }

  /// Fills the given region with the default type for the block kind. Min must
  /// be less than or equal to max. Use [`min_max`](Pos::min_max) to convert two
  /// corners of a cube into a min and max.
  pub async fn fill_kind(&self, min: Pos, max: Pos, kind: block::Kind) -> Result<(), PosError> {
    self.fill(min, max, self.block_converter.get(kind).default_type()).await
  }

  /// This broadcasts a chat message to everybody in the world.
  pub async fn broadcast<M: Into<Chat>>(&self, msg: M) {
    let out = cb::Packet::Chat {
      message:      msg.into().to_json(),
      position:     0, // Chat box, not above hotbar
      sender_v1_16: Some(UUID::from_u128(0)),
    };

    for p in self.players.lock().await.values() {
      p.conn().send(out.clone()).await;
    }
  }

  // Runs f for all players within render distance of the chunk.
  pub async fn players(&self) -> MutexGuard<'_, PlayersMap> {
    self.players.lock().await
  }
}

impl Default for WorldManager {
  fn default() -> Self {
    WorldManager::new()
  }
}

impl WorldManager {
  pub fn new() -> Self {
    WorldManager {
      block_converter:  Arc::new(block::TypeConverter::new()),
      item_converter:   Arc::new(item::TypeConverter::new()),
      entity_converter: Arc::new(entity::TypeConverter::new()),
      plugins:          Arc::new(plugin::PluginManager::new()),
      worlds:           Mutex::new(vec![]),
    }
  }

  pub async fn run(self: Arc<Self>) {
    self.plugins.clone().run(self).await;
  }

  /// Adds a new world. Currently, this requires a mutable reference, which
  /// cannot be obtained outside of initialization.
  pub async fn add_world(self: &Arc<Self>) {
    self.worlds.lock().await.push(World::new(
      self.block_converter.clone(),
      self.item_converter.clone(),
      self.entity_converter.clone(),
      self.plugins.clone(),
      self.clone(),
    ));
  }

  /// Returns the current block converter. This can be used to convert old block
  /// ids to new ones, and vice versa. This can also be used to convert block
  /// kinds to types.
  pub fn get_block_converter(&self) -> &block::TypeConverter {
    &self.block_converter
  }

  /// Returns the current item converter. This can be used to convert old item
  /// ids to new ones, and vice versa.
  pub fn get_item_converter(&self) -> &item::TypeConverter {
    &self.item_converter
  }

  /// Broadcasts a message to everyone one the server.
  pub async fn broadcast<M: Into<Chat>>(&self, msg: M) {
    let out = cb::Packet::Chat {
      message:      msg.into().to_json(),
      position:     0, // Chat box, not above hotbar
      sender_v1_16: Some(UUID::from_u128(0)),
    };

    let worlds = self.worlds.lock().await;
    for w in worlds.iter() {
      for p in w.players.lock().await.values() {
        p.conn().send(out.clone()).await;
      }
    }
  }

  /// Returns the default world. This can be used to easily get a world without
  /// any other context.
  pub async fn default_world(&self) -> Arc<World> {
    self.worlds.lock().await[0].clone()
  }

  /// Adds a new player into the game. This should be called when a new grpc
  /// proxy connects.
  pub async fn new_player(&self, req: Streaming<Packet>, tx: Sender<Result<Packet, Status>>) {
    let mut conn = Connection::new(req, tx);
    let (username, uuid, ver) = conn.wait_for_login().await;
    let w = self.worlds.lock().await[0].clone();
    let player = Player::new(
      w.eid(),
      username,
      uuid,
      Arc::new(conn),
      ver,
      w.clone(),
      FPos::new(0.0, 60.0, 0.0),
    );
    w.new_player(player).await;
  }
}
