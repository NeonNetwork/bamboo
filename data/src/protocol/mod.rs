mod json;
mod parse;

use convert_case::{Case, Casing};
use itertools::Itertools;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::{
  collections::{HashMap, HashSet},
  error::Error,
  fs,
  fs::File,
  io::Write,
  path::Path,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IntType {
  I8,
  U8,
  U16,
  I16,
  I32,
  I64,
  VarInt,
  OptVarInt, // Acts the same as a varint, but is sometimes not present
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FloatType {
  F32,
  F64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CountType {
  // A typed count
  Typed(IntType),
  // A hardocded count
  Fixed(u32),
  // Another protocol field should be used as the count
  Named(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BitField {
  name:   String,
  size:   u32,
  signed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PacketField {
  // Simple fields
  Native, // Should never exist
  Bool,
  Int(IntType),
  Float(FloatType),
  UUID,
  String,
  Position,

  // Sizable fields
  NBT,
  Slot,
  OptionalNBT,
  RestBuffer, // The rest of the buffer
  EntityMetadata,

  // Complicated fields
  Option(Box<PacketField>),
  Array { count: CountType, value: Box<PacketField> },
  Buffer(CountType),
  BitField(Vec<BitField>),
  Container(HashMap<String, PacketField>),
  Switch { compare_to: String, fields: HashMap<String, PacketField> },
  Mappings(HashMap<String, u32>), // Mapping of packet names to ids

  // Logical fields
  CompareTo(String),
  DefinedType(String), // Another type, defined within either the types map or the packets map
}

impl PacketField {
  pub fn into_container(self) -> Option<HashMap<String, PacketField>> {
    match self {
      Self::Container(v) => Some(v),
      _ => None,
    }
  }
  pub fn into_compare(self) -> Option<String> {
    match self {
      Self::CompareTo(v) => Some(v),
      _ => None,
    }
  }
  pub fn into_defined(self) -> Option<String> {
    match self {
      Self::DefinedType(v) => Some(v),
      _ => None,
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Packet {
  pub name:   String,
  pub fields: HashMap<String, PacketField>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
  // The index is the packet's id. The names should be mapped to the indicies as well.
  pub to_client: Vec<Packet>,
  pub to_server: Vec<Packet>,
}

pub fn store(dir: &Path) -> Result<(), Box<dyn Error>> {
  let dir = Path::new(dir).join("protocol");

  // This is done at runtime of the buildscript, so this path must be relative to
  // where the buildscript is.
  let versions = parse::load_all(Path::new("../data/minecraft-data/data/pc"))?;

  fs::create_dir_all(&dir)?;
  {
    // Generates the version json in a much more easily read format. This is much
    // faster to compile than generating source code.
    let mut f = File::create(&dir.join("versions.json"))?;
    writeln!(f, "{}", serde_json::to_string(&versions)?)?;
  }
  {
    // Generates the packet id enum, for clientbound and serverbound packets
    let mut to_client = HashSet::new();
    let mut to_server = HashSet::new();

    for (_, v) in versions {
      for p in v.to_client {
        to_client.insert(p.name);
      }
      for p in v.to_server {
        to_server.insert(p.name);
      }
    }
    let to_client: Vec<String> = to_client.into_iter().sorted().collect();
    let to_server: Vec<String> = to_server.into_iter().sorted().collect();

    let mut f = File::create(&dir.join("cb.rs"))?;
    writeln!(f, "/// Auto generated packet ids. This is a combination of all packet")?;
    writeln!(f, "/// names for all versions. Some of these packets are never used.")?;
    writeln!(f, "#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq, Hash)]")?;
    writeln!(f, "pub enum ID {{")?;
    // We always want a None type, to signify an invalid packet
    writeln!(f, "  None,")?;
    for n in &to_client {
      let name = n.to_case(Case::Pascal);
      writeln!(f, "  {},", name)?;
    }
    writeln!(f, "}}")?;

    let mut f = File::create(&dir.join("sb.rs"))?;
    writeln!(f, "/// Auto generated packet ids. This is a combination of all packet")?;
    writeln!(f, "/// names for all versions. Some of these packets are never used.")?;
    writeln!(f, "#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive, PartialEq, Eq, Hash)]")?;
    writeln!(f, "pub enum ID {{")?;
    // We always want a None type, to signify an invalid packet
    writeln!(f, "  None,")?;
    for n in &to_server {
      let name = n.to_case(Case::Pascal);
      writeln!(f, "  {},", name)?;
    }
    writeln!(f, "}}")?;
    writeln!(f, "impl ID {{")?;
    writeln!(f, "  pub fn from_str(s: &str) -> Self {{")?;
    writeln!(f, "    match s {{")?;
    for n in &to_server {
      let name = n.to_case(Case::Pascal);
      writeln!(f, "      \"{}\" => ID::{},", n, name)?;
    }
    writeln!(f, "      _ => ID::None,")?;
    writeln!(f, "    }}")?;
    writeln!(f, "  }}")?;
    writeln!(f, "}}")?;
  }
  Ok(())
}
