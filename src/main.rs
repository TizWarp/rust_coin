use futures::StreamExt;
use libp2p::*;
use libp2p::swarm::*;
use libp2p_gossipsub::{Config, IdentTopic};
use std::error::Error;
// 1. Define custom network behavior
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    kademlia: libp2p::kad::Behaviour<kad::store::MemoryStore>,
    gossipsub : libp2p::gossipsub::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let mut swarm = SwarmBuilder::with_new_identity()
    .with_tokio()
    .with_tcp(
        tcp::Config::default(), noise::Config::new, yamux::Config::default)?
    .with_behaviour(|key| {

        let kad_id = PeerId::from(key.public());

        let kademlia = kad::Behaviour::new(kad_id, kad::store::MemoryStore::new(kad_id));
        
        let gossipsub = gossipsub::Behaviour::new(
            libp2p_gossipsub::MessageAuthenticity::Signed(key.clone()), Config::default())?;

        Ok(MyBehaviour{kademlia, gossipsub})
    })?
    .build();

    println!("{:?}", swarm.local_peer_id());

    let topic = IdentTopic::new("TEST");

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // 6. Listen on all interfaces
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;



    println!("Node started. Searching for peers via DHT...");

    // 8. Event loop
    loop {
        match swarm.select_next_some().await {


            SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(kad::Event::RoutingUpdated { peer, is_new_peer, ..})) => {
                if is_new_peer{
                    println!("added peer: {:?}", peer);
                    swarm.behaviour_mut().gossipsub.publish(topic.clone(), b"There is a new peer".to_vec())?;
                }
            }


            _ => ()
        }
    }
}