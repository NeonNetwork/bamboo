use sc_common::{
  chunk::paletted::Section,
  gnet::cb::Packet,
  math::ChunkPos,
  version::{BlockVersion, ProtocolVersion},
};

mod v1_14;
mod v1_8;
mod v1_9;

mod cb;
mod conv;
mod sb;

pub use cb::{ToTcp, WriteError};
pub use conv::TypeConverter;
pub use sb::{FromTcp, ReadError};

pub fn chunk(
  pos: ChunkPos,
  full: bool,
  bit_map: u16,
  sections: Vec<Section>,
  ver: ProtocolVersion,
  conv: &TypeConverter,
) -> Packet {
  match ver.block() {
    BlockVersion::V1_8 => v1_8::chunk(pos, full, bit_map, &sections, conv),
    BlockVersion::V1_9 | BlockVersion::V1_12 => {
      v1_9::chunk(pos, full, bit_map, &sections, ver, conv)
    }
    // ProtocolVersion::V1_13 => v1_13::serialize_chunk(pos, bit_map, &sections, conv),
    BlockVersion::V1_14 => v1_14::chunk(pos, full, bit_map, &sections, conv),
    // ProtocolVersion::V1_15 => v1_15::serialize_chunk(pos, c),
    // ProtocolVersion::V1_16 => v1_16::serialize_chunk(pos, c),
    _ => todo!("chunk on version {}", ver),
  }
}
