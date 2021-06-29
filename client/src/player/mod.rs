use crate::{graphics::WindowData, net::Connection, Settings};
use cgmath::Vector3;
use common::util::UUID;
use std::{ops::Deref, sync::Arc};

/// The player that the client is using. This include information about
/// rendering, the camera position, and anything else that is client specific.
pub struct MainPlayer {
  player: OtherPlayer,
  conn:   Arc<Connection>,
}

/// This is a struct used for any player. This includes logic for parsing
/// packets coming from the server, and how to render a player model.
pub struct OtherPlayer {
  name: String,
  uuid: UUID,

  pos:   Vector3<f32>,
  pitch: f32,
  yaw:   f32,
}

impl MainPlayer {
  pub fn new(settings: &Settings, conn: Arc<Connection>) -> Self {
    let info = settings.get_info();
    MainPlayer {
      player: OtherPlayer::new(info.username(), info.uuid(), Vector3::new(0.0, 0.0, 0.0)),
      conn,
    }
  }

  /// Called every frame. This updates the player's view direction and position.
  pub fn render(&self, win: &mut WindowData) {
    let (dx, dy) = win.mouse_delta();
    info!("mouse delta: {} {}", dx, dy);
  }
}

impl Deref for MainPlayer {
  type Target = OtherPlayer;

  fn deref(&self) -> &Self::Target {
    &self.player
  }
}

impl OtherPlayer {
  pub fn new(name: &str, uuid: UUID, pos: Vector3<f32>) -> Self {
    OtherPlayer { name: name.into(), uuid, pos, pitch: 0.0, yaw: 0.0 }
  }

  #[inline(always)]
  pub fn pos(&self) -> Vector3<f32> {
    self.pos
  }
  #[inline(always)]
  pub fn pitch(&self) -> f32 {
    self.pitch
  }
  #[inline(always)]
  pub fn yaw(&self) -> f32 {
    self.yaw
  }
}
