use libp2p::PeerId;
use mongodb::Database;

use crate::handlers::tools::{
    create_log::write_log,
    syncer::{Sync, Syncer},
};

use super::{
    addresses::Listeners,
    block::Block,
    relay::{DialedRelays, Relay},
};

pub struct StablishedHandler;

impl StablishedHandler {
    pub async fn handle<'a>(
        dialed_relays: &mut DialedRelays,
        peer_id: PeerId,
        db: &'a Database,
        sync_state: &mut Sync,
        recieved_blocks: &mut Vec<Block>,
        multiaddress: &String,
        peerid: &PeerId,
        last_block: &mut Vec<Block>,
    ) -> Result<(), &'a str> {
        write_log(&format!("Connection stablished with: {}", peer_id));
        if let Some(relay) = dialed_relays
            .relays
            .iter()
            .find(|relay| relay.addr.contains(&peer_id.to_string()))
        {
            //updating relay peerid in database
            match Relay::update(&mut relay.clone(), db, Some(peer_id), None).await {
                Ok(_) => {
                    //if sync is NotSynced start syncing with the network
                    match sync_state {
                        Sync::Synced => Ok(()),
                        Sync::NotSynced => {
                            write_log("start syncing...");
                            if let Err(e) = Syncer::syncing(db, recieved_blocks, last_block).await {
                                write_log(e);
                                Err(e)
                            } else {
                                sync_state.synced(); //if syncing doesn't have problem change sync state to Synced

                                match Listeners::new(&multiaddress.parse().unwrap(), peerid, db)
                                    .await
                                {
                                    Ok(listeners) => match listeners.post().await {
                                        Ok(_) => Ok(()),
                                        Err(_) => Err(""),
                                    },
                                    Err(e) => Err(e),
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    write_log(e);
                    Err("Error while updating relay-(practical/connections 60)")
                }
            }
        } else {
            Ok(())
        }
    }
}
