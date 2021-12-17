use crate::world::chunk::MultiChunk;
use sc_common::{
  math::{ChunkPos, Pos},
  net::cb,
  version::BlockVersion,
};

// mod v1_13;
// mod v1_14;
// mod v1_15;
// mod v1_16;
// mod v1_8;
// mod v1_9;

pub fn serialize_multi_block_change<I>(_pos: ChunkPos, ver: BlockVersion, _changes: I) -> cb::Packet
where
  I: ExactSizeIterator<Item = (Pos, i32)>,
{
  match ver {
    // BlockVersion::V1_8 | BlockVersion::V1_12 => v1_8::serialize_multi_block_change(pos, changes),
    // BlockVersion::V1_13 => v1_13::serialize_multi_block_change(pos, c),
    // BlockVersion::V1_14 => v1_14::serialize_multi_block_change(pos, c),
    ver => unimplemented!("cannot serialize multi block change for version {:?}", ver),
  }
}
