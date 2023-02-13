use color_eyre::Result;
use libp2p::{
    core::upgrade,
    floodsub::{self, Floodsub, FloodsubEvent, Topic},
    identity, mdns, mplex, noise,
    swarm::NetworkBehaviour,
    tcp, PeerId, Swarm, Transport,
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
pub struct Behaviour {
    pub floodsub: Floodsub,
    pub mdns: mdns::tokio::Behaviour,
}

#[allow(clippy::large_enum_variant)]
pub enum BehaviourEvent {
    Floodsub(FloodsubEvent),
    Mdns(mdns::Event),
}

impl From<FloodsubEvent> for BehaviourEvent {
    fn from(event: FloodsubEvent) -> Self {
        BehaviourEvent::Floodsub(event)
    }
}

impl From<mdns::Event> for BehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        BehaviourEvent::Mdns(event)
    }
}

pub struct Node {
    pub subscribed_to: Topic,
    pub swarm: Swarm<Behaviour>,
}

impl Node {
    pub fn new() -> Result<Self> {
        let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());

        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(
                noise::NoiseAuthenticated::xx(&id_keys)
                    .expect("Signing libp2p-noise static DH keypair failed."),
            )
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        let floodsub_topic = floodsub::Topic::new("chat");

        let mdns_behaviour = mdns::Behaviour::new(Default::default())?;
        let mut behaviour = Behaviour {
            floodsub: Floodsub::new(peer_id),
            mdns: mdns_behaviour,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());
        let swarm = Swarm::with_tokio_executor(transport, behaviour, peer_id);

        Ok(Self {
            swarm,
            subscribed_to: floodsub_topic,
        })
    }
}
