use std::sync::Arc;
use std::{collections::HashMap, error::Error};

use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::{ids, net};

enum ClientMessage {
    Message(Arc<Vec<u8>>),
    Error(Box<dyn std::error::Error + Sync + Send>),
}

struct ClientWrapper {
    id: usize,
    recv_task: tokio::task::JoinHandle<()>,
    send_task: tokio::task::JoinHandle<()>,
    sender: mpsc::Sender<ClientMessage>,
    receiver: mpsc::Receiver<ClientMessage>,
}

impl ClientWrapper {
    fn new(id: usize, client: TcpStream) -> Self {
        let (send_send, mut send_recv) = mpsc::channel(16);
        let (recv_send, recv_recv) = mpsc::channel(16);
        let (mut read, mut write) = client.into_split();
        let recv_task = tokio::spawn(async move {
            while let Some(ClientMessage::Message(msg)) = send_recv.recv().await {
                if let Err(e) = net::send_size_prefixed(&mut write, msg.as_slice()).await {
                    panic!("Error sending message: {}", e);
                }
            }
            panic!("Client recv task dead");
        });
        let send_task = tokio::spawn(async move {
            loop {
                match net::recv_size_prefixed(&mut read).await {
                    Ok(msg) => {
                        let msg = Arc::new(msg);
                        if let Err(e) = recv_send.send(ClientMessage::Message(msg)).await {
                            panic!("Error sending message: {}", e);
                        }
                    }
                    Err(e) => {
                        recv_send.send(ClientMessage::Error(e)).await.unwrap();
                        break;
                    }
                }
            }
            panic!("Client send task dead");
        });
        Self {
            id,
            recv_task,
            send_task,
            sender: send_send,
            receiver: recv_recv,
        }
    }

    async fn send_message(
        &self,
        message: Arc<Vec<u8>>,
    ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        self.sender
            .send(ClientMessage::Message(message))
            .await
            .map_err(|e| e.into())
    }

    async fn receive_message(
        &mut self,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Sync + Send>> {
        match self.receiver.recv().await {
            Some(ClientMessage::Message(msg)) => Ok((*msg).clone()),
            _ => Err("Failed to receive message".into()),
        }
    }
}

/// Manager is a class which keeps track of N clients and aggregates their messages
/// into a single receiver. Similarly, it sends messages to all clients from a single sender.
pub struct Manager {
    client_ids: ids::IdGenerator,
    clients: HashMap<usize, ClientWrapper>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            client_ids: ids::IdGenerator::new(),
            clients: HashMap::new(),
        }
    }

    pub fn add_client(&mut self, client: TcpStream) -> usize {
        // Add client to the list of clients
        let id = self.client_ids.next_id();
        self.clients.insert(id, ClientWrapper::new(id, client));
        id
    }

    pub fn remove_client(&mut self, id: usize) {
        if let Some(_) = self.clients.remove(&id) {
            self.client_ids.release_id(id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    pub async fn recv_message(
        &mut self,
    ) -> Result<(usize, Vec<u8>), (usize, Box<dyn Error + Send + Sync>)> {
        // Create a vector of futures for receiving messages from all clients
        let mut futures = Vec::new();
        for client in self.clients.values_mut() {
            futures.push(Box::pin(client.receive_message()));
        }

        let (result, index, _) = futures::future::select_all(futures).await;
        let client_id = self.clients.keys().nth(index).cloned().unwrap();
        result
            .map(|msg| (client_id, msg))
            .map_err(|e| (client_id, e))
    }

    /// Sends a message to all clients.
    pub async fn send_message(
        &self,
        message: Vec<u8>,
    ) -> Result<(), HashMap<usize, Box<dyn std::error::Error + Sync + Send>>> {
        let mut errors = HashMap::new();
        // wrap in an Arc to avoid cloning the data for EVERY client :^)
        let message = Arc::new(message);
        for client in self.clients.values() {
            if let Err(e) = client.send_message(message.clone()).await {
                errors.insert(client.id, e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
