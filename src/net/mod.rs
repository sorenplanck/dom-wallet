// Network module — peer registry, backbone connectivity, peer rotation.
//
// The DOM backbone VPS at 168.100.8.245:33370 is the bootstrap peer.
// Additional peers are discovered through the backbone and persisted to
// <portable>/peers/peers.json so the wallet can reconnect quickly on restart.

pub mod peer;
pub mod p2p;

pub const BACKBONE_HOST: &str = "168.100.8.245";
pub const BACKBONE_PORT: u16 = 33370;
pub const BACKBONE_PEER: &str = "168.100.8.245:33370";
