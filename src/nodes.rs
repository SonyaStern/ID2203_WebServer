// use crate::kv::{KeyValue};
// use omnipaxos_core::{
//     omni_paxos::{OmniPaxos, OmniPaxosConfig},
// };
// use omnipaxos_storage::{
//     persistent_storage::{PersistentStorage, PersistentStorageConfig},
// };
// use commitlog::LogOptions;
// use omnipaxos_core::storage::{Entry, Snapshot};
// use sled::{Config};
//
//
// fn OmniPaxos() {
// // configuration with id 1 and the following cluster
//     let configuration_id = 1;
//     let cluster = vec![1, 2, 3];
//
// // create the replica 2 in this cluster (other replica instances are created similarly with pid 1 and 3 on the other nodes)
//     let my_pid = 2;
//     let my_peers = vec![1, 3];
//
//     let omni_paxos_config = OmniPaxosConfig {
//         configuration_id,
//         pid: my_pid,
//         peers: my_peers,
//         ..Default::default()
//     };
//
//     /* Re-creating our node after a crash... */
//
// // Configuration from previous storage
//     let my_path = "/my_path_before_crash/";
//     let my_log_opts = LogOptions::new(my_path);
//     let mut persist_conf = PersistentStorageConfig::default();
//
//     persist_conf.set_path(my_path.parse().unwrap()); // set the path to the persistent storage
//     persist_conf.set_commitlog_options(my_log_opts);
//
// // Re-create storage with previous state, then create `OmniPaxos`
//     let recovered_storage = PersistentStorage::open(persist_conf);
//     let mut recovered_paxos = omni_paxos_config.build(recovered_storage);
//     recovered_paxos.fail_recovery();
// }