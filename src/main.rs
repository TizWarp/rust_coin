use futures::StreamExt;
use libp2p::*;
use libp2p::swarm::*;
use libp2p_gossipsub::{Config, IdentTopic};
use std::{env::args, error::Error, hash::{DefaultHasher, Hash, Hasher}, io::Read, str::FromStr};
// 1. Define custom network behavior
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    kademlia: libp2p::kad::Behaviour<kad::store::MemoryStore>,
    gossipsub : libp2p::gossipsub::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let mut is_bootstrap_node = false;

    let (task_tx, mut task_rx) = tokio::sync::mpsc::channel::<Task>(100);


    tokio::spawn(async move {
        let mut diff : u32 = 1;
        let mut nonce : u64 = 0;
        loop {
            let mut hasher = DefaultHasher::new();
            hasher.write_u64(nonce);
            if hasher.finish().leading_zeros() == diff{
                task_tx.send(Task::MinedBlock).await.unwrap();
                nonce = 0;
                diff += 1;
            } else {
                nonce += 1;
            }

        }
    });

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

    let topic = IdentTopic::new("MinedBlocks");

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    if let Some(addr) = args().nth(1) {
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed to: {addr}");
    } else {
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        is_bootstrap_node = true;
    }

    println!("Node started. Searching for peers via DHT...");

    // 8. Event loop
    loop {

        tokio::select! {

        
        swarm_event = swarm.select_next_some() => match swarm_event {

            SwarmEvent::ConnectionEstablished { peer_id, ..} => {
                println!("Connected to peer {}", peer_id);
            }

            SwarmEvent::ConnectionClosed { peer_id,..} => {
                println!("Connection to peer {} closed", peer_id);
            }

            SwarmEvent::OutgoingConnectionError { peer_id, error , ..} => {
                println!("Failed to connected to peer {}: {}", peer_id.unwrap_or(PeerId::random()), error)
            }

            SwarmEvent::NewListenAddr {address,..} => {
                if is_bootstrap_node{
                    println!("Full connection string for bootstraping {}/p2p/{}",address, swarm.local_peer_id());
                }
            }

            SwarmEvent::Behaviour(MyBehaviourEvent::Kademlia(event)) => match event {
                kad::Event::RoutingUpdated { peer,.. } => {
                    println!("Peer {} added to routing table", peer);
                }

                _ => ()
            }

            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(event)) => match event {
                
                _ => ()
            }


            _ => ()
        },

    Some(task) = task_rx.recv() => {
        match task {
            Task::MinedBlock => {
                match swarm.behaviour_mut().gossipsub.publish(topic.clone(), b"Mined Block".to_vec()) {
                    Ok(_) => (),
                    Err(error) => println!("Failed to send message {}", error),
                }
            }
        }
    }

    }
    }
}



enum Task{
    MinedBlock,
}