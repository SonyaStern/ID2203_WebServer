// #[macro_use]
// extern crate lazy_static;


use std::{collections::HashMap, collections::HashSet, fs, sync::{Arc, Mutex}};

use lazy_static::lazy_static;
use omnipaxos_core::{
    messages::Message,
    omni_paxos::*,
    util::{LogEntry, NodeId},
};
use omnipaxos_storage::persistent_storage::{PersistentStorage, PersistentStorageConfig};
use tokio::{runtime::Builder, runtime::Runtime, sync::mpsc};
use tokio::task::JoinHandle;

use crate::{
    kv::{KeyValue, KVSnapshot},
    server::OmniPaxosServer,
    util::*,
};

mod nodes;
mod kv;
mod server;
mod util;

type OmniPaxosKV = OmniPaxos<KeyValue, KVSnapshot, PersistentStorage<KeyValue, KVSnapshot>>;

const SERVERS: [u64; 3] = [1, 2, 3];
const PERSIST_PATH: &str = "storage";

lazy_static! {
    // static ref OP_SERVER_HANDLES: Mutex<HashMap<u64, (Arc<Mutex<OmniPaxosKV>>, JoinHandle<()>, OmniPaxosConfig)>> = {
    //     let map = HashMap::new();
    //     Mutex::new(map)
    // };
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

fn initialise_channels() -> (
    // TODO: Should replace with tokio-rpc?
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

fn main() {

    // Clean-up
    fs::remove_dir_all("storage1");
    fs::remove_dir_all("storage2");
    fs::remove_dir_all("storage3");

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

    /*
    Demo
    */

    // wait for leader to be elected...
    std::thread::sleep(WAIT_LEADER_TIMEOUT);
    let (first_server, _, _) = handlers.get(&1).unwrap();
    // check which server is the current leader
    let leader = first_server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    println!("Elected leader: {}", leader);

    let follower = SERVERS.iter().find(|&&p| p != leader).unwrap();
    let (follower_server, _, _) = handlers.get(follower).unwrap();
    // append kv1 to the replicated log via follower
    let kv1 = KeyValue {
        key: "a".to_string(),
        value: 1,
    };
    println!("Adding value: {:?} via server {}", kv1, follower);
    follower_server
        .lock()
        .unwrap()
        .append(kv1)
        .expect("append failed");

    // append kv2 to the replicated log via the leader
    let kv2 = KeyValue {
        key: "b".to_string(),
        value: 2,
    };
    println!("Adding value: {:?} via server {}", kv2, leader);
    let (leader_server, leader_join_handle, _) = handlers.get(&leader).unwrap();
    leader_server
        .lock()
        .unwrap()
        .append(kv2)
        .expect("append failed");
    // wait for the entries to be decided...
    std::thread::sleep(WAIT_DECIDED_TIMEOUT);
    let committed_ents = leader_server
        .lock()
        .unwrap()
        .read_decided_suffix(0)
        .expect("Failed to read expected entries");

    let mut simple_kv_store = HashMap::new();
    for ent in committed_ents {
        match ent {
            LogEntry::Decided(kv) => {
                simple_kv_store.insert(kv.key, kv.value);
            }
            _ => {} // ignore not committed entries
        }
    }
    println!("KV store: {:?}", simple_kv_store);
    println!("Killing leader: {}...", leader);
    leader_join_handle.abort();

    // wait for new leader to be elected...
    std::thread::sleep(WAIT_LEADER_TIMEOUT);
    let leader = follower_server
        .lock()
        .unwrap()
        .get_current_leader()
        .expect("Failed to get leader");
    println!("Elected new leader: {}", leader);
    let kv3 = KeyValue {
        key: "b".to_string(),
        value: 3,
    };
    println!("Adding value: {:?} via server {}", kv3, leader);
    let (leader_server, _, _) = handlers.get(&leader).unwrap();
    leader_server
        .lock()
        .unwrap()
        .append(kv3)
        .expect("append failed");
    // wait for the entries to be decided...
    std::thread::sleep(WAIT_DECIDED_TIMEOUT);
    let committed_ents = follower_server
        .lock()
        .unwrap()
        .read_decided_suffix(2)
        .expect("Failed to read expected entries");
    for ent in committed_ents {
        match ent {
            LogEntry::Decided(kv) => {
                simple_kv_store.insert(kv.key, kv.value);
            }
            _ => {} // ignore not committed entries
        }
    }
    println!("KV store: {:?}", simple_kv_store);

    recovery(handlers, sender_storage);
    std::thread::sleep(WAIT_LEADER_TIMEOUT);
    std::thread::sleep(WAIT_LEADER_TIMEOUT * 10);
}


fn recovery(mut handlers: HashMap<u64, (Arc<Mutex<OmniPaxosKV>>, JoinHandle<()>, OmniPaxosConfig)>,
            sender: HashMap<NodeId, mpsc::Sender<Message<KeyValue, KVSnapshot>>>) {
    // Configuration from previous storage
    // let log_opts = LogOptions::new(path);
    let pids = TO_RECOVER.lock().unwrap();

    for pid in pids.iter() {
        println!("---------------- Recovering pid {:?}", pid);

        // Re-create storage with previous state, then create `OmniPaxos`
        // let persist_conf = OmniPaxosServer::configure_persistent_storage(String::from(PERSIST_PATH) + &*pid.to_string());
        // let recovered_storage: PersistentStorage<KeyValue, KVSnapshot> = PersistentStorage::open(persist_conf);
        let (recovered_paxos, old_join, config)
            = handlers.get(pid).unwrap();

        // let mut op_server = OmniPaxosServer {
        //     omni_paxos: Arc::clone(&recovered_paxos),
        //     incoming: receiver_channels.remove(&pid).unwrap(),
        //     outgoing: sender_channels.clone(),
        // };
        // let join_handle = runtime.spawn({
        //     async move {
        //         op_server.run().await;
        //     }
        // });
        let (sender_channels, mut receiver_channels) = initialise_channels();
        recovered_paxos.lock().unwrap().fail_recovery();
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
        handlers.insert(*pid, (recovered_paxos.clone(), join_handle, config.clone()));
        println!("---------------- Recovered pid {:?}", pid);

        // Check leaders
        let follower = SERVERS.iter().find(|&&p| p == *pid).unwrap();
        let (follower_server, _, _) = handlers.get(follower).unwrap();
        let leader = follower_server
            .lock()
            .unwrap()
            .get_current_leader()
            .expect("Failed to get leader");
        println!("Elected new leader: {}, asked this server: {}", leader, follower);

        let follower = SERVERS.iter().find(|&&p| p != *pid).unwrap();
        let (follower_server, _, _) = handlers.get(follower).unwrap();
        let leader = follower_server
            .lock()
            .unwrap()
            .get_current_leader()
            .expect("Failed to get leader");
        println!("Elected new leader: {}, asked this server: {}", leader, follower);
    }
}