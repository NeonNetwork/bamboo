use crate::world::WorldManager;
use bb_common::util::Chat;
use bb_ffi::{CChat, CUUID};
use log::Level;
use std::sync::Arc;
use wasmer::{imports, Array, Function, ImportObject, LazyInit, Memory, Store, WasmPtr, WasmerEnv};

#[derive(WasmerEnv, Clone)]
pub struct Env {
  #[wasmer(export)]
  pub memory: LazyInit<Memory>,
  pub wm:     Arc<WorldManager>,
  pub name:   Arc<String>,
}

impl Env {
  pub fn mem(&self) -> &Memory { self.memory.get_ref().expect("Env not initialized") }
}

fn log_from_level(level: u32) -> Option<Level> {
  Some(match level {
    1 => Level::Error,
    2 => Level::Warn,
    3 => Level::Info,
    4 => Level::Debug,
    5 => Level::Trace,
    _ => return None,
  })
}

fn log(env: &Env, level: u32, message: WasmPtr<u8, Array>) {
  let level = match log_from_level(level) {
    Some(l) => l,
    None => return,
  };
  // SAFETY: We aren't using the string outside this function,
  // so this is safe. It also avoids allocating, so that's why
  // we use this instead of `get_utf8_string_with_nul`.
  let s = unsafe { message.get_utf8_str_with_nul(env.mem()).unwrap() };
  log!(level, "`{}`: {}", env.name, s);
}

fn log_len(env: &Env, level: u32, message: WasmPtr<u8, Array>, len: u32) {
  let level = match log_from_level(level) {
    Some(l) => l,
    None => return,
  };
  // SAFETY: We aren't using the string outside this function,
  // so this is safe. It also avoids allocating, so that's why
  // we use this instead of `get_utf8_string_with_nul`.
  let s = unsafe { message.get_utf8_str(env.mem(), len).unwrap() };
  log!(level, "`{}`: {}", env.name, s);
}

fn broadcast(env: &Env, message: WasmPtr<CChat>) {
  let chat = message.deref(env.mem()).unwrap().get();
  let ptr = WasmPtr::<u8, _>::new(chat.message as u32);
  let s = ptr.get_utf8_string_with_nul(env.mem()).unwrap();
  env.wm.broadcast(Chat::new(s));
}

fn player_username(
  env: &Env,
  id: WasmPtr<CUUID>,
  buf: WasmPtr<u8>,
  buf_len: u32,
) -> i32 {
  let mem = env.mem();
  let uuid = match id.deref(mem) {
    Some(id) => id.get(),
    None => return 1,
  };
  let player = match env.wm.get_player(bb_common::util::UUID::from_u128(
    (uuid.bytes[3] as u128) << (3 * 32) | (uuid.bytes[2] as u128) << (2 * 32) | (uuid.bytes[1] as u128) << 32 | uuid.bytes[0] as u128,
  )) {
    Some(p) => p,
    None => return 1,
  };
  let bytes = player.username().as_bytes();
  let end = buf.offset() + bytes.len() as u32;
  if bytes.len() > buf_len as usize {
    return 1;
  }
  if end as usize > mem.size().bytes().0 {
    return 1;
  }
  unsafe {
    let ptr = mem.view::<u8>().as_ptr().add(buf.offset() as usize) as *mut u8;
    let slice: &mut [u8] = std::slice::from_raw_parts_mut(ptr, bytes.len());
    slice.copy_from_slice(bytes);
  }
  0
}

pub fn imports(store: &Store, wm: Arc<WorldManager>, name: String) -> ImportObject {
  let env = Env { memory: LazyInit::new(), wm, name: Arc::new(name) };
  imports! {
    "env" => {
      "bb_log" => Function::new_native_with_env(&store, env.clone(), log),
      "bb_log_len" => Function::new_native_with_env(&store, env.clone(), log_len),
      "bb_broadcast" => Function::new_native_with_env(&store, env.clone(), broadcast),
      "bb_player_username" => Function::new_native_with_env(&store, env.clone(), player_username),
    }
  }
}
