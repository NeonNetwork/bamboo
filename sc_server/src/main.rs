#[macro_use]
extern crate log;

use clap::Parser;
use sc_server::{net::ConnectionManager, plugin::PluginManager, world::WorldManager};
use std::{error::Error, sync::Arc, thread};
use sugarlang::Sugarlang;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
  /// If set, the server will generate sugarlang docs and exit, without doing
  /// anything else.
  #[clap(short, long)]
  only_docs: bool,
}

// #[derive(Clone)]
// pub struct ServerImpl {
//   worlds: Arc<WorldManager>,
// }
//
// #[tonic::async_trait]
// impl Minecraft for ServerImpl {
//   type ConnectionStream = ReceiverStream<Result<Packet, Status>>;
//   async fn connection(
//     &self,
//     req: Request<Streaming<Packet>>,
//   ) -> Result<Response<Self::ConnectionStream>, Status> {
//     let (tx, rx) = mpsc::channel(8);
//
//     // We need to wait for a packet to be recieved from the proxy before we
// can     // create the player (we need a username and uuid). Therefore, we
// need to do     // this on another task.
//     let worlds = self.worlds.clone();
//     tokio::spawn(async move {
//       worlds.new_player(req.into_inner(), tx).await;
//     });
//
//     Ok(Response::new(ReceiverStream::new(rx)))
//   }
//   async fn status(&self, req: Request<StatusRequest>) ->
// Result<Response<StatusResponse>, Status> {     dbg!(req);
//     Ok(Response::new(StatusResponse {
//       id:          vec![],
//       num_players: 5,
//       server_type: "sugarcane-rs".into(),
//     }))
//   }
//   async fn reserve_slots(
//     &self,
//     req: Request<ReserveSlotsRequest>,
//   ) -> Result<Response<ReserveSlotsResponse>, Status> {
//     dbg!(req);
//     Ok(Response::new(ReserveSlotsResponse::default()))
//   }
// }

fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();
  sc_common::init("server");

  if args.only_docs {
    let mut sl = Sugarlang::new();
    PluginManager::add_builtins(&mut sl);
    return Ok(());
  }

  let addr = "0.0.0.0:8483".parse().unwrap();

  let wm = Arc::new(WorldManager::new());
  wm.add_world();

  let w = wm.clone();
  thread::spawn(|| w.run());

  let mut conn = ConnectionManager::new(wm);

  info!("listening on {}", addr);
  conn.run(addr)?;

  Ok(())
}
