use crate::graphics::Vert3;
use common::{
  chunk::{Chunk, ChunkKind},
  math::{Face, Pos, PosError},
  proto,
};
use std::{ops::Deref, sync::Arc};

use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  device::Device,
};

/// A chunk with a mesh. This acts the same as a normal mesh, but will lazily
/// update a mesh any time it needs to be rendered.
pub struct MeshChunk {
  chunk:    Chunk,
  outdated: bool,
  vbuf:     Arc<CpuAccessibleBuffer<[Vert3]>>,
  device:   Arc<Device>,
}

impl MeshChunk {
  /// Creates a new mesh chunk from the given chunk. This will generate all of
  /// the initial geometry for this chunk. Any time this chunk is rendered, the
  /// geometry will be updated (not when the chunk itself is updated).
  pub fn new(chunk: Chunk, device: Arc<Device>) -> Self {
    let vbuf =
      CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, [].iter().cloned())
        .unwrap();
    let mut c = MeshChunk { chunk, vbuf, outdated: true, device };
    c.update_mesh();
    c
  }

  /// Returns the buffer used to render this chunk. This will also update this
  /// buffer if the geometry is out of date.
  pub fn get_vbuf(&mut self) -> &Arc<CpuAccessibleBuffer<[Vert3]>> {
    if self.outdated {
      self.update_mesh();
    }
    &self.vbuf
  }

  /// Updates the mesh. This should only be called internally, but if called
  /// externally, the new mesh will be correctly used on the next frame.
  pub fn update_mesh(&mut self) {
    let mut buf = vec![];
    for chunk_y in 0..16i32 {
      if self.chunk.has_section(chunk_y as u32) {
        for section_y in 0..16 {
          let y = chunk_y * 16 + section_y;
          for z in 0..16 {
            for x in 0..16 {
              let p = Pos::new(x, y, z);
              // We want p to be solid, and an air block next to it. This makes it easiest to
              // lookup the texture we want.
              if self.chunk.get_block(p) == Ok(0) {
                continue;
              }
              // TODO: Textures
              let u = 0.0;
              let v = 0.0;
              if self.chunk.get_block(Pos::new(x, y + 1, z)) == Ok(0) {
                MeshChunk::add_face(&mut buf, p, Face::Up, u, v);
              }
              if self.chunk.get_block(Pos::new(x, y - 1, z)) == Ok(0) {
                MeshChunk::add_face(&mut buf, p, Face::Down, u, v);
              }
              if self.chunk.get_block(Pos::new(x, y, z + 1)) == Ok(0) {
                MeshChunk::add_face(&mut buf, p, Face::South, u, v);
              }
              if self.chunk.get_block(Pos::new(x, y, z - 1)) == Ok(0) {
                MeshChunk::add_face(&mut buf, p, Face::North, u, v);
              }
              if self.chunk.get_block(Pos::new(x + 1, y, z)) == Ok(0) {
                MeshChunk::add_face(&mut buf, p, Face::East, u, v);
              }
              if self.chunk.get_block(Pos::new(x - 1, y, z)) == Ok(0) {
                MeshChunk::add_face(&mut buf, p, Face::West, u, v);
              }
            }
          }
        }
      }
    }
    info!("len: {}", buf.len());
    self.vbuf = CpuAccessibleBuffer::from_iter(
      self.device.clone(),
      BufferUsage::all(),
      false,
      buf.into_iter(),
    )
    .unwrap();
    self.outdated = false;
  }

  /// Adds a face at the given position. The position is the minimum point on
  /// the block. So if face is up, then all of the points added would be above
  /// the pos passed in.
  fn add_face(buf: &mut Vec<Vert3>, pos: Pos, face: Face, u: f32, v: f32) {
    // TODO: Make faces one-sided, and fix the direction of some of these triangles.
    match face {
      Face::Up => {
        buf.push(Vert3::new(1.0, 1.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 1.0, 1.0, u, v) + pos);
      }
      Face::Down => {
        buf.push(Vert3::new(1.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 1.0, u, v) + pos);
      }
      Face::South => {
        buf.push(Vert3::new(1.0, 1.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 1.0, 1.0, u, v) + pos);
      }
      Face::North => {
        buf.push(Vert3::new(1.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 1.0, 0.0, u, v) + pos);
      }
      Face::East => {
        buf.push(Vert3::new(1.0, 1.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(1.0, 1.0, 1.0, u, v) + pos);
      }
      Face::West => {
        buf.push(Vert3::new(0.0, 1.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 0.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 0.0, 1.0, u, v) + pos);
        buf.push(Vert3::new(0.0, 1.0, 1.0, u, v) + pos);
      }
    }
  }

  /// Creates a new mesh chunk from the given protobuf. This will call
  /// [`new`](Self::new) after parsing the protobuf.
  pub fn from_proto(p: proto::Chunk, device: Arc<Device>) -> Self {
    MeshChunk::new(Chunk::from_latest_proto(p, ChunkKind::Fixed), device)
  }

  // Overrides the [`set_block`](Chunk::set_block) function on [`Chunk`]. This is
  // done so that the mesh will be correctly updated on the next frame.
  pub fn set_block(&mut self, pos: Pos, ty: u32) -> Result<(), PosError> {
    self.outdated = true;
    self.chunk.set_block(pos, ty)
  }

  // Overrides the [`fill`](Chunk::fill) function on [`Chunk`]. This is done so
  // that the mesh will be correctly updated on the next frame.
  pub fn fill(&mut self, min: Pos, max: Pos, ty: u32) -> Result<(), PosError> {
    self.outdated = false;
    self.chunk.fill(min, max, ty)
  }
}

impl Deref for MeshChunk {
  type Target = Chunk;

  fn deref(&self) -> &Self::Target {
    &self.chunk
  }
}
