use std::{collections::HashMap, collections::HashSet, fs, sync::{Arc, Mutex}};

use actix_web::{App, HttpServer};
use lazy_static::lazy_static;
use omnipaxos_core::{
    messages::Message,
    omni_paxos::*,
    util::NodeId,
};
use omnipaxos_storage::persistent_storage::PersistentStorage;
use tokio::{runtime::Builder, runtime::Runtime, sync::mpsc, time};
use tokio::task::JoinHandle;

use crate::{
    kv::{KeyValue, KVSnapshot},
    server::OmniPaxosServer,
    util::*,
};
use crate::kv_controller::{cas, create, get};

mod kv;
mod server;
mod util;
mod kv_controller;
mod storage;
mod nodes;

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
async fn main() -> std::io::Result<()> {

    // Clean-up storage
    cleanup();

    OP_SERVER_HANDLERS.lock().unwrap().extend(initialise_handlers());

    HttpServer::new(move || {
        App::new()
            .service(create)
            .service(get)
            .service(cas)
    })
        .bind(("127.0.0.1", 8000))?
        .run()
        .await
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

fn initialise_handlers() -> HashMap<u64, (Arc<Mutex<OmniPaxosKV>>, JoinHandle<()>, OmniPaxosConfig)> {
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
            String::from(PERSIST_PATH) + &*pid.to_string());
        let omni_paxos: Arc<Mutex<OmniPaxosKV>> =
            Arc::new(Mutex::new(op_config.clone().build(PersistentStorage::new(persist_config))));

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


fn recovery(pid: u64) -> (u64, (Arc<Mutex<OmniPaxosKV>>, JoinHandle<()>, OmniPaxosConfig)) {
    // Configuration from previous storage
    let handlers = OP_SERVER_HANDLERS.lock().unwrap();

    std::thread::sleep(WAIT_LEADER_TIMEOUT * 5);

    let follower = SERVERS.iter().find(|&&p| p != pid).unwrap();
    let (follower_server, _, _) = handlers.get(follower).unwrap();
    println!("---------------- Searching for a leader pid {:?}", follower);
    let leader = follower_server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    println!("Old leader: {}, asked this server: {}", leader, follower);

    // Re-create storage with previous state, then create `OmniPaxos`
    let (recovered_paxos, old_join, config)
        = handlers.get(&pid).unwrap();

    old_join.abort();

    println!("---------------- Recovering pid {:?}", pid);

    let (sender_channels, mut receiver_channels) = initialise_channels();
    let peers: Vec<u64> = SERVERS.iter().filter(|&&p| p != pid).copied().collect();
    recovered_paxos.lock().unwrap().fail_recovery();
    let mut op_server = OmniPaxosServer {
        omni_paxos: Arc::clone(&recovered_paxos),
        incoming: receiver_channels.remove(&pid).unwrap(),
        outgoing: sender_channels.clone(),
    };
    for peer in peers {
        recovered_paxos.lock().unwrap().reconnected(peer);
    }
    let join_handle = RUNTIME.spawn({
        async move {
            op_server.run().await;
        }
    });
    std::thread::sleep(WAIT_LEADER_TIMEOUT * 2);
    println!("---------------- Recovered pid {:?}", pid);

    // Check leaders
    let follower = SERVERS.iter().find(|&&p| p != pid).unwrap();
    let (follower_server, _, _) = handlers.get(follower).unwrap();
    println!("---------------- Searching for a leader pid {:?}", follower);
    let leader = follower_server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    println!("Elected new leader: {}, asked this server: {}", leader, follower);

    (pid, (recovered_paxos.clone(), join_handle, config.clone()))
}