// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of substrate-archive.

// substrate-archive is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// substrate-archive is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with substrate-archive.  If not, see <http://www.gnu.org/licenses/>.

use log::*;
use futures::{Future, Stream, sync::mpsc, future, lazy};
use tokio::runtime::{self, Runtime};
use tokio::util::StreamExt;
use failure::{Error as FailError};
use substrate_subxt::{Client, ClientBuilder, srml::system::System};
use substrate_rpc_primitives::number::NumberOrHex;
use runtime_primitives::traits::Header;

use crate::types::{Data, Payload, BlockNumber, Block};
use crate::error::{Error as ArchiveError};

// temporary util function to get a Substrate Client and Runtime
fn client<T: System + 'static>() -> Result<(Runtime, Client<T>), ArchiveError> {
    let mut rt = Runtime::new()?;
    let client_future = ClientBuilder::<T>::new().build();
    let client = rt.block_on(client_future)?;
    Ok((rt, client))
}

fn handle_data<T>(receiver: mpsc::UnboundedReceiver<Data<T>>,
                  rpc: Rpc<T>,
                  sender: mpsc::UnboundedSender<Data<T>>
) -> impl Future<Item = (), Error = ()> + 'static
where T: System + std::fmt::Debug + 'static
{
    println!("Spawning Receiver");
    receiver.enumerate().for_each(move |(i, data)| {
        match &data.payload {
            Payload::FinalizedHead(header) => {
                println!("Finalized Header");
                println!("item: {}, {:?}", i, data);
            },
            Payload::Header(header) => {
                tokio::spawn(
                    rpc.block(header.hash(), sender.clone())
                        .map_err(|e| println!("{:?}", e))
                );
                println!("item: {}, {:?}", i, data);
                println!("Header");
            }
            Payload::BlockNumber(number) => {
                println!("Block Number");

                println!("item: {}, {:?}", i, data);
            },
            Payload::Block(block) => {
                println!("GOT A Block");
                println!("item: {}, {:?}", i, data);
            },
            Payload::Event(event) => {
                println!("Event");

                println!("item: {}, {:?}", i, data);
            },
            Payload::Hash(hash) => {
                println!("HASH: {:?}", hash);
            }
            _ => {
                println!("not handled");
            }
        };
        future::ok(())
    })
}

pub fn run<T: System + std::fmt::Debug + 'static>() -> Result<(), ArchiveError>{
    let  (mut rt, client) = client::<T>()?;
    let (sender, receiver) = mpsc::unbounded();
    let rpc = Rpc::new(client);
    rt.spawn(rpc.subscribe_new_heads(sender.clone()).map_err(|e| println!("{:?}", e)));
    rt.spawn(rpc.subscribe_finalized_blocks(sender.clone()).map_err(|e| println!("{:?}", e)));
    rt.spawn(rpc.subscribe_events(sender.clone()).map_err(|e| println!("{:?}", e)));
    tokio::run(handle_data(receiver, rpc, sender));
    Ok(())
}

/// Communicate with Substrate node via RPC
pub struct Rpc<T: System> {
    client: Client<T>
}

impl<T> Rpc<T> where T: System + 'static {

    /// instantiate new client
    pub fn new(client: Client<T>) -> Self {
        Self { client }
    }

    /// send all new headers back to main thread
    pub fn subscribe_new_heads(&self, sender: mpsc::UnboundedSender<Data<T>>) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client.subscribe_blocks()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |head| {
                    sender.unbounded_send(Data {
                        payload: Payload::Header(head)
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }

    /// send all finalized headers back to main thread
    pub fn subscribe_finalized_blocks(&self, sender: mpsc::UnboundedSender<Data<T>>) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client.subscribe_finalized_blocks()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |head| {
                    sender.unbounded_send(Data {
                        payload: Payload::FinalizedHead(head)
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }

    /// send all substrate events back to main thread
    pub fn subscribe_events(&self, sender: mpsc::UnboundedSender<Data<T>>) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client.subscribe_events()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |storage_change| {
                    sender.unbounded_send(Data {
                        payload: Payload::Event(storage_change),
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }

    fn block_hash(&self, block_number: Option<BlockNumber<T>>, sender: mpsc::UnboundedSender<Data<T>>)
             -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client
            .block_hash(block_number)
            .map_err(Into::into)
            .and_then(move |hash| {
                if let Some(h) = hash {
                    sender.unbounded_send(Data {
                        payload: Payload::Hash(h)
                    }).map_err(|e| ArchiveError::from(e))
                } else {
                    info!("No Hash Exists!");
                    Ok(()) // TODO Error out
                }
            })
    }

    fn block(&self, hash: T::Hash, sender: mpsc::UnboundedSender<Data<T>>)
             -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client
            .block(Some(hash))
            .map_err(Into::into)
            .and_then(move |block| {
                if let Some(b) = block {
                    sender.unbounded_send(Data {
                        payload: Payload::Block(b)
                    }).map_err(|e| ArchiveError::from(e))
                } else {
                    info!("No block exists! (somehow)");
                    Ok(()) // TODO: error Out
                }
            })
    }

    fn unsubscribe_finalized_heads() {
        unimplemented!();
    }

    fn unsubscribe_new_heads() {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn can_query_blocks() {

    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
