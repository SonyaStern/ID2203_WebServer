use std::{collections::HashMap, sync::{Arc, Mutex}};

use commitlog::LogOptions;
use omnipaxos_core::{messages::Message, util::NodeId};
use omnipaxos_storage::persistent_storage::PersistentStorageConfig;
use sled::Config;
use tokio::{sync::mpsc, time};

use crate::{OmniPaxosKV, recovery, RUNTIME, TO_RECOVER, util::{ELECTION_TIMEOUT, OUTGOING_MESSAGE_PERIOD}, WAIT_LEADER_TIMEOUT};
use crate::kv::{KeyValue, KVSnapshot};

pub struct OmniPaxosServer {
    pub omni_paxos: Arc<Mutex<OmniPaxosKV>>,
    pub incoming: mpsc::Receiver<Message<KeyValue, KVSnapshot>>,
    pub outgoing: HashMap<NodeId, mpsc::Sender<Message<KeyValue, KVSnapshot>>>,
}

impl OmniPaxosServer {
    async fn send_outgoing_msgs(&mut self) {
        let messages = self.omni_paxos.lock().unwrap().outgoing_messages();
        for msg in messages {
            // println!("Outgoing message: {:?}", msg);
            let receiver = msg.get_receiver();
            // send out_msg to receiver on network layer
            let channel = self
                .outgoing
                .get_mut(&receiver)
                .expect("No channel for receiver");
            let response = channel.send(msg).await;
            // println!("Response message: {:?}", response);
            if response.is_err() {
                println!("Here is error: {:?}, pid {}", response, receiver);
                self.omni_paxos.lock().unwrap().reconnected(receiver);
            }
            // else {
            //     let pr: i32 = rand::thread_rng().gen_range(1..100);
            //     println!("Probability: {}", pr);
            //     if pr > 90 {
            //         let (k, v) = recovery(receiver);
            //         OP_SERVER_HANDLERS.lock().unwrap().insert(k, v);
            //     }
            // }
        }
    }

    pub(crate) async fn run(&mut self) {
        let mut outgoing_interval = time::interval(OUTGOING_MESSAGE_PERIOD);
        let mut election_interval = time::interval(ELECTION_TIMEOUT);
        loop {
            tokio::select! {
                biased;
                _ = election_interval.tick() => { self.omni_paxos.lock().unwrap().election_timeout(); },
                _ = outgoing_interval.tick() => { self.send_outgoing_msgs().await; },
                Some(in_msg) = self.incoming.recv() => { self.omni_paxos.lock().unwrap().handle_incoming(in_msg); },
                else => { }
            }
        }
    }

    pub(crate) fn configure_persistent_storage(path: String) -> PersistentStorageConfig {
        let log_opts = LogOptions::new(path.clone());
        let mut sled_opts = Config::new();
        sled_opts = Config::path(sled_opts, path.clone());

        // generate default configuration and set user-defined options
        let persist_config = PersistentStorageConfig::with(
            path.to_string(), log_opts, sled_opts);

        persist_config
    }
}