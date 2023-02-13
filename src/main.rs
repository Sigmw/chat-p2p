use color_eyre::{eyre::Context, Result};
use futures::StreamExt;
use libp2p::{floodsub::FloodsubEvent, mdns, swarm::SwarmEvent, Multiaddr};
use node::Node;
use tokio::io::{self, AsyncBufReadExt};

use crate::node::BehaviourEvent;
mod node;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = color_eyre::install();
    let mut node = Node::new().context("failed to create node")?;

    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        node.swarm.dial(addr)?;
        println!("Dialed {to_dial:?}");
    }

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    node.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let line = line?.expect("stdin closed");
                node.swarm.behaviour_mut().floodsub.publish(node.subscribed_to.clone(), line.as_bytes());
            }
            event = node.swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {address:?}");
                    }
                    SwarmEvent::Behaviour(BehaviourEvent::Floodsub(FloodsubEvent::Message(message))) => {
                        println!(
                                "{:?}: {:?}",
                                message.source,
                                String::from_utf8_lossy(&message.data),
                            );
                    }
                    SwarmEvent::Behaviour(BehaviourEvent::Mdns(event)) => {
                        match event {
                            mdns::Event::Discovered(list) => {
                                for (peer, _) in list {
                                    node.swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer);
                                }
                            }
                            mdns::Event::Expired(list) => {
                                for (peer, _) in list {
                                    if !node.swarm.behaviour().mdns.has_node(&peer) {
                                        node.swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
