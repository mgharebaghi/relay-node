use libp2p::gossipsub::Message;

use super::{
    check_trx::handle_transactions, nodes_sync_announce::handle_sync_message,
    recieved_block::verifying_block, structures::FullNodes,
};

//check gossip messages and do its operations.....................................................................
pub async fn msg_check(message: Message, mut leader: &mut String, fullnodes: &mut Vec<FullNodes>) {
    let str_msg = String::from_utf8(message.data.clone()).unwrap();

    handle_sync_message(fullnodes, &str_msg);

    // handle_transactions(message.clone()).await;

    verifying_block(&str_msg, &mut leader, fullnodes).await;
}
