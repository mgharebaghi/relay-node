use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use libp2p::{PeerId, Swarm};
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};

use crate::relay::{events::connections::ConnectionsHandler, tools::create_log::write_log};

use super::swarm::CentichainBehaviour;

pub struct Leader {
    pub peerid: Option<PeerId>,
    pub timer: LeaderTime,
    pub time: Option<DateTime<Utc>>,
    pub in_check: bool,
    pub votes: Vec<PeerId>,
}

#[derive(Debug, PartialEq)]
pub enum LeaderTime {
    On,
    Off,
}

impl LeaderTime {
    pub fn start(&mut self) {
        *self = Self::On
    }

    pub fn off(&mut self) {
        *self = Self::Off
    }
}

impl Leader {
    //define new leader
    pub fn new(peerid: Option<PeerId>) -> Self {
        Self {
            peerid,
            timer: LeaderTime::Off,
            time: None,
            in_check: false,
            votes: Vec::new(),
        }
    }

    //set in ckeck true
    fn check_start(&mut self) {
        self.in_check = true
    }

    // start leader time for check its block
    pub fn timer_start(&mut self) {
        self.timer.start();
        self.time.get_or_insert(Utc::now() + Duration::seconds(59));
    }

    //update to new leader
    pub fn update(&mut self, peerid: Option<PeerId>) {
        self.timer.off();
        self.time = None;
        self.peerid = peerid;
        self.in_check = false;
    }

    //propagate validator's vote about new leader to the network
    pub async fn start_voting<'a>(
        &mut self,
        db: &'a Database,
        conection_handler: &mut ConnectionsHandler,
        swarm: &mut Swarm<CentichainBehaviour>,
    ) -> Result<(), &'a str> {
        // set in_check of leader true
        self.check_start();

        //first, delete left leader from validators as a wrongdoer
        match conection_handler
            .remove(db, self.peerid.unwrap(), swarm)
            .await
        {
            Ok(_) => Ok(write_log(&format!(
                "Left leader remove as a wrongdoer: {}",
                self.peerid.unwrap()
            ))),
            Err(e) => Err(e),
        }
    }

    //check votes and if it was quorum set it as leader
    pub async fn check_votes<'a>(&mut self, db: &Database, vote: PeerId) -> Result<(), &'a str> {
        let collection: Collection<Document> = db.collection("validators");
        //get count documents for knowing votes are upper than 50% of documents number or not
        match collection.count_documents(doc! {}).await {
            Ok(count) => {
                self.votes.push(vote);

                //if votes are upper than 50% of validators find most vote to set as leader
                if self.votes.len() >= ((count / 2) + 1) as usize {
                    let mut hashmap_of_votes = HashMap::new(); //make hashmap to group by vote per peerid
                    for v in self.votes.clone() {
                        *hashmap_of_votes.entry(v).or_insert(0) += 1; //plus 1 if key is repetitive
                    }
                    //get the most vote and change leader
                    let result = hashmap_of_votes
                        .iter()
                        .max_by_key(|v| *v)
                        .unwrap()
                        .0
                        .clone();
                    self.update(Some(result));
                    self.timer_start();
                }
                Ok(())
            }
            Err(_) => Err("Error while get count of validators' doc-(generator/leader 169)"),
        }
    }
}
