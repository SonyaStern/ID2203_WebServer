use std::{
    collections::HashMap,
    collections::HashSet,
    fs,
    sync::{Arc, Mutex},
};

use actix_web::{App, HttpServer};
use futures::io::BufWriter;
use lazy_static::lazy_static;
use omnipaxos_core::{messages::Message, omni_paxos::*, util::NodeId};
use omnipaxos_storage::persistent_storage::PersistentStorage;
use tokio::task::JoinHandle;
use tokio::{runtime::Builder, runtime::Runtime, sync::mpsc};

use crate::kv_controller::{create, get};
use crate::{
    kv::{KVSnapshot, KeyValue},
    server::OmniPaxosServer,
    util::*,
};

// Libraries for statics
use omnipaxos::{Acceptor, Ballot, Paxos, Proposer};
use std::collections::HashMap;
// These libraries are for the output to be on a file txt for the statistics
use std::fs::File;
use std::oi::{BufWriter, Write};
use std::path::Path;
// #########################################################################

mod kv;
mod kv_controller;
mod nodes;
mod server;
mod storage;
mod util;

type OmniPaxosKV = OmniPaxos<KeyValue, KVSnapshot, PersistentStorage<KeyValue, KVSnapshot>>;

const SERVERS: [u64; 3] = [1, 2, 3];
const PERSIST_PATH: &str = "storage";

lazy_static! {
    static ref OP_SERVER_HANDLERS: Mutex<HashMap<u64, (Arc<Mutex<OmniPaxosKV>>, JoinHandle<()>, OmniPaxosConfig)>> = {
        let map = HashMap::new();
        Mutex::new(map)
    };
    static ref TO_RECOVER: Mutex<HashSet<NodeId>> = {
        let list = HashSet::new();
        Mutex::new(list)
    };
    static ref RUNTIME: Runtime = {
        let runtime = Builder::new_multi_thread()
            .worker_threads(8)
            .enable_all()
            .build()
            .unwrap();
        runtime
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<(), Box<dyn std::error::Error>> {
    // Clean-up storage
    cleanup();

    let acceptors = vec![Acceptor::new(1), Acceptor::new(2), Acceptor::new(3)];

    let mut paxos_acceptors = Paxos::new(acceptors);

    let output_file_path = "output_statistics.txt";
    get_statistics(&paxos, output_file_path);

    println!("Statistics written to file: {}", output_file_path);

    OP_SERVER_HANDLERS
        .lock()
        .unwrap()
        .extend(initialise_handlers());

    HttpServer::new(move || App::new().service(create).service(get))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await;

    Ok(())
}

fn get_statics(paxos: &Paxos, output_file_path: &src) -> Result<(), Box<dyn std::error::Error>> {
    let mut statistics = HashMap::new();

    for acceptor in &paxos.acceptors {
        let accepted_ballot = acceptor.get_accepted_ballot();
        if let Some(ballot) = accepted_ballot {
            *statistics.entry(ballot).or_insert(0) += 1;
        }
    }

    let path = Path::new(output_file_path);
    let file = File::create(&path);
    let mut writer = BufWriter::new(file);

    writeln!(&mut writer, "{:20} {}", "Ballot", "Count");
    for (ballot, count) in statistics {
        writeln!(&mut writer, "{:20} {}", format!("{:?}", ballot), count);
    }

    Ok(())
}

fn cleanup() -> () {
    fs::remove_dir_all("storage1");
    fs::remove_dir_all("storage2");
    fs::remove_dir_all("storage3");
}

fn initialise_channels() -> (
    HashMap<NodeId, mpsc::Sender<Message<KeyValue, KVSnapshot>>>,
    HashMap<NodeId, mpsc::Receiver<Message<KeyValue, KVSnapshot>>>,
) {
    let mut sender_channels = HashMap::new();
    let mut receiver_channels = HashMap::new();

    for pid in SERVERS {
        let (sender, receiver) = mpsc::channel(BUFFER_SIZE);
        sender_channels.insert(pid, sender);
        receiver_channels.insert(pid, receiver);
    }
    (sender_channels, receiver_channels)
}

fn initialise_handlers() -> HashMap<u64, (Arc<Mutex<OmniPaxosKV>>, JoinHandle<()>, OmniPaxosConfig)>
{
    // configuration with id 1 and the following cluster
    let configuration_id = 1;

    let (sender_storage, mut receiver_storage) = initialise_channels();

    let mut handlers = HashMap::new();
    // create the replicas in this cluster
    for pid in SERVERS {
        let peers = SERVERS.iter().filter(|&&p| p != pid).copied().collect();
        let op_config = OmniPaxosConfig {
            pid,
            configuration_id,
            peers,
            ..Default::default()
        };

        // user-defined configuration for the persistent storage for each node
        let persist_config = OmniPaxosServer::configure_persistent_storage(
            String::from(PERSIST_PATH) + &*pid.to_string(),
        );
        let omni_paxos: Arc<Mutex<OmniPaxosKV>> = Arc::new(Mutex::new(
            op_config
                .clone()
                .build(PersistentStorage::new(persist_config)),
        ));

        let mut op_server = OmniPaxosServer {
            omni_paxos: Arc::clone(&omni_paxos),
            incoming: receiver_storage.remove(&pid).unwrap(),
            outgoing: sender_storage.clone(),
        };
        let join_handle = RUNTIME.spawn({
            async move {
                op_server.run().await;
            }
        });
        handlers.insert(pid, (omni_paxos, join_handle, op_config.clone()));
    }
    (handlers)
}

fn recovery(pid: u64) {
    // Configuration from previous storage
    // let pids = TO_RECOVER.lock().unwrap();

    let mut handlers = OP_SERVER_HANDLERS.lock().unwrap();

    // for pid in pids.iter() {
    println!("---------------- Recovering pid {:?}", pid);

    // Re-create storage with previous state, then create `OmniPaxos`
    let (recovered_paxos, old_join, config) = handlers.get(&pid).unwrap();

    let (sender_channels, mut receiver_channels) = initialise_channels();
    recovered_paxos.lock().unwrap().fail_recovery();
    let peers: Vec<u64> = SERVERS.iter().filter(|&&p| p != pid).copied().collect();
    for peer in peers {
        recovered_paxos.lock().unwrap().reconnected(peer);
    }
    let mut op_server = OmniPaxosServer {
        omni_paxos: Arc::clone(&recovered_paxos),
        incoming: receiver_channels.remove(&pid).unwrap(),
        outgoing: sender_channels.clone(),
    };
    let join_handle = RUNTIME.spawn({
        async move {
            op_server.run().await;
        }
    });
    std::thread::sleep(WAIT_LEADER_TIMEOUT);
    // handlers.insert(pid, (recovered_paxos.clone(), join_handle, config.clone()));
    println!("---------------- Recovered pid {:?}", pid);

    // Check leaders
    let follower = SERVERS.iter().find(|&&p| p == pid).unwrap();
    let (follower_server, _, _) = handlers.get(follower).unwrap();
    let leader = follower_server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    println!(
        "Elected new leader: {}, asked this server: {}",
        leader, follower
    );

    let follower = SERVERS.iter().find(|&&p| p != pid).unwrap();
    let (follower_server, _, _) = handlers.get(follower).unwrap();
    let leader = follower_server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    println!(
        "Elected new leader: {}, asked this server: {}",
        leader, follower
    );
    // }
}
