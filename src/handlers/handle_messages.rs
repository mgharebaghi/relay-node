use libp2p::{
    gossipsub::{IdentTopic, Message},
    PeerId, Swarm,
};

use super::{
    check_trx::handle_transactions,
    nodes_sync_announce::handle_sync_message,
    recieved_block::verifying_block,
    structures::{CustomBehav, FullNodes},
};

//check gossip messages and do its operations.....................................................................
pub async fn msg_check(
    message: Message,
    mut leader: &mut String,
    fullnodes: &mut Vec<FullNodes>,
    swarm: &mut Swarm<CustomBehav>,
    propagation_source: PeerId,
    clients_topic: IdentTopic,
    client_topic_subscriber: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    my_addresses: &mut Vec<String>,
) {
    let str_msg = String::from_utf8(message.data.clone()).unwrap();

    handle_sync_message(fullnodes, &str_msg);

    handle_transactions(String::from_utf8(message.data).unwrap()).await;

    verifying_block(
        &str_msg,
        &mut leader,
        fullnodes,
        swarm,
        propagation_source,
        clients_topic,
        client_topic_subscriber,
        relays,
        clients,
        relay_topic,
        my_addresses,
    )
    .await;
}
