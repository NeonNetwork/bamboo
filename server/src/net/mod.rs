use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc::Sender, Mutex};
use tonic::{Status, Streaming};

use common::{
  net::{cb, sb},
  proto,
};

use crate::{block, player::Player};

pub struct Connection {
  rx:     Mutex<Streaming<proto::Packet>>,
  tx:     Mutex<Sender<Result<proto::Packet, Status>>>,
  closed: AtomicBool,
}

impl Connection {
  pub(crate) fn new(
    rx: Streaming<proto::Packet>,
    tx: Sender<Result<proto::Packet, Status>>,
  ) -> Self {
    Connection { rx: Mutex::new(rx), tx: Mutex::new(tx), closed: false.into() }
  }

  /// This starts up the recieving loop for this connection. Do not call this
  /// more than once.
  pub(crate) async fn run(&self, player: &Mutex<Player>) -> Result<(), Status> {
    'running: loop {
      let p = match self.rx.lock().await.message().await? {
        Some(p) => sb::Packet::from_proto(p),
        None => break 'running,
      };
      match p.id() {
        sb::ID::BlockDig => {
          let pos = p.get_pos("location");
          info!("digging at {}", &pos);
          player.lock().await.world().set_kind(pos, block::Kind::Air);
        }
        _ => warn!("got unknown packet from client: {:?}", p),
      }
      // info!("got packet from client {:?}", p);
    }
    self.closed.store(true, Ordering::SeqCst);
    Ok(())
  }

  /// Sends a packet to the proxy, which will then get sent to the client.
  pub async fn send(&self, p: cb::Packet) {
    info!("sending packet");
    self.tx.lock().await.send(Ok(p.into_proto())).await.unwrap();
  }

  // Returns true if the connection has been closed.
  pub fn closed(&self) -> bool {
    self.closed.load(Ordering::SeqCst)
  }
}
