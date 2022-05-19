pub use bb_ffi as ffi;
pub use log::*;

use bb_ffi::CChat;
use parking_lot::Mutex;
use std::{ffi::CString, marker::PhantomData};

use bb_common::util::Chat;

pub use bb_common::{math, util};

pub mod player;

pub struct Bamboo {
  marker: PhantomData<()>,
}

pub fn instance() -> Bamboo { Bamboo { marker: PhantomData::default() } }

impl Bamboo {
  pub fn broadcast(&self, message: Chat) {
    unsafe {
      let s = CString::new(message.to_codes()).unwrap();
      let c_chat = CChat { message: s.as_ptr() };
      bb_ffi::bb_broadcast(&c_chat);
    }
  }
}

use log::{Level, LevelFilter, Metadata, Record};

struct Logger;
static LOGGER: Logger = Logger;

impl log::Log for Logger {
  fn enabled(&self, metadata: &Metadata) -> bool { metadata.level() <= Level::Info }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {
      unsafe {
        if let Some(s) = record.args().as_str() {
          bb_ffi::bb_log_len(record.level() as u32, s.as_ptr() as *const _, s.len() as u32);
        } else {
          let s = record.args().to_string();
          bb_ffi::bb_log_len(record.level() as u32, s.as_ptr() as *const _, s.len() as u32);
        }
      }
    }
  }
  fn flush(&self) {}
}

pub fn init() {
  log::set_logger(&LOGGER).unwrap();
  log::set_max_level(LevelFilter::Info);
}

use parking_lot::lock_api::RawMutex;

macro_rules! callback {
  ( $setter:ident, $static:ident, $sig:ty ) => {
    static $static: Mutex<Option<Box<dyn ($sig) + Send>>> =
      Mutex::const_new(parking_lot::RawMutex::INIT, None);
    pub fn $setter(callback: impl ($sig) + Send + 'static) {
      *$static.lock() = Some(Box::new(callback));
    }
  };
}

callback!(set_on_block_place, ON_BLOCK_PLACE, Fn(player::Player, ffi::CPos));
#[no_mangle]
extern "C" fn on_block_place(id: ffi::CUUID, x: i32, y: i32, z: i32) {
  if let Some(cb) = ON_BLOCK_PLACE.lock().as_ref() {
    let p = player::Player::new(id);
    let pos = ffi::CPos { x, y, z };
    cb(p, pos);
  }
}
