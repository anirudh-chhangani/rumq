use derive_more::From;
use rumq_core::mqtt4::{Packet, Connect, Publish};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::mpsc::error::TrySendError;
use tokio::select;
use tokio::time::{self, Duration};
use tokio::stream::StreamExt;

use std::collections::{HashMap, VecDeque};
use std::fmt;

use crate::state::{self, MqttState};

mod commitlog;
mod connection;

use connection::{ActiveConnection, InactiveConnection};

#[derive(Debug, From)]
pub enum Error {
    State(state::Error),
    AllSendersDown,
    Mpsc(TrySendError<RouterMessage>),
}

/// Router message to orchestrate data between connections. We can also
/// use this to send control signals to connections to modify their behavior
/// dynamically from the console
#[derive(Debug)]
pub enum RouterMessage {
    /// Client id and connection handle
    Connect(Connection),
    /// Packet
    Packet(Packet),
    /// Packets
    Packets(Vec<Packet>),
    /// Disconnects a client from active connections list. Will handling
    Death(String),
    /// Pending messages of the previous connection
    Pending(VecDeque<Publish>)
}

pub struct Connection {
    pub connect: Connect,
    pub handle: Option<Sender<RouterMessage>>
}

impl Connection {
    pub fn new(connect: Connect, handle: Sender<RouterMessage>) -> Connection {
        Connection {
            connect,
            handle: Some(handle)
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.connect)
    }
}

#[derive(Debug)]
pub struct Router {
    commitlog: commitlog::CommitLog,
    // handles to all active connections. used to route data
    active_connections:     HashMap<String, ActiveConnection>,
    // inactive persistent connections
    inactive_connections:   HashMap<String, InactiveConnection>,
    // channel receiver to receive data from all the active_connections.
    // each connection will have a tx handle
    data_rx:                Receiver<(String, RouterMessage)>,
}

impl Router {
    pub fn new(data_rx: Receiver<(String, RouterMessage)>) -> Self {
        Router {
            commitlog: commitlog::CommitLog::new(10 * 1024 * 1024, 100).unwrap(),
            active_connections: HashMap::new(),
            inactive_connections: HashMap::new(),
            data_rx,
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let mut interval = time::interval(Duration::from_millis(100));

        loop {
            select! {
                o = self.data_rx.recv() => {
                    let (id, mut message) = o.unwrap();                   
                    debug!("In router message. Id = {}, {:?}", id, message);
                    match self.reply(id.clone(), &mut message) {
                        Ok(Some(message)) => self.forward(&id, message),
                        Ok(None) => (),
                        Err(e) => {
                            error!("Incoming handle error = {:?}", e);
                            continue;
                        }
                    }

                    // adds routes, routes the message to other connections etc etc
                    self.fill_and_track(id, message);
                }
                _ = interval.next() => self.route(),
            }
        }
    }

    /// replys back to the connection sending the message
    /// doesn't modify any routing information of the router
    /// all the routing and route modifications due to subscriptions
    /// are part of `route` method
    /// generates reply to send backto the connection. Shouldn't touch anything except active and 
    /// inactive connections
    /// No routing modifications here
    fn reply(&mut self, id: String, message: &mut RouterMessage) -> Result<Option<RouterMessage>, Error> {
        match message {
            RouterMessage::Connect(connection) => {
                let handle = connection.handle.take().unwrap();
                let message = self.handle_connect(connection.connect.clone(), handle)?;
                Ok(message)
            }
            RouterMessage::Packet(packet) => {
                let message = self.handle_incoming_packet(&id, packet.clone())?;
                Ok(message)
            }
            _ => Ok(None)
        }
    }

    /// fills publishes to the commit log and tracks new routes
    fn fill_and_track(&mut self, id: String, message: RouterMessage) {
        match message {
            RouterMessage::Packet(packet) => {
                match packet {
                    Packet::Publish(publish) => {
                        self.fill_commitlog(publish.clone());
                    }
                    Packet::Subscribe(subscribe) => if let Some(connection) = self.active_connections.get_mut(&id) {
                        connection.add_to_subscriptions(subscribe);
                    }
                    Packet::Unsubscribe(unsubscribe) => if let Some(connection) = self.active_connections.get_mut(&id) {
                        connection.remove_from_subscriptions(unsubscribe);
                    }
                    Packet::Disconnect => self.deactivate(id),
                    _ => return
                }
            }
            RouterMessage::Death(_) => {
                self.deactivate_and_forward_will(id.to_owned());
            }
            _ => () 
        }
    }

    fn deactivate_and_forward_will(&mut self, id: String) {
        info!("Deactivating client due to connection death. Id = {}", id);

        if let Some(_connection) = self.active_connections.remove(&id) {}
    }

    fn route(&mut self) {
        let active_connections = &mut self.active_connections;
        for (_id, connection) in active_connections.iter_mut() {
            let concrete_subscriptions = &mut connection.concrete_subscriptions;
            let commitlog = &self.commitlog;
            for (filter, subscription) in concrete_subscriptions.iter_mut() {
                let (segment_id, log_offset) = subscription.offset();
                let qos = subscription.qos;
                if let Some(logs) = commitlog.get(&filter, segment_id, log_offset, 100) {
                    let packets = logs.packets.clone();
                    connection.state.handle_outgoing_publishes(qos, packets.len());
                    match connection.tx.try_send(RouterMessage::Packets(packets)) {
                        Ok(_) => {
                            subscription.update_offset(logs.segment_id, logs.log_offset + 1);
                            continue
                        }
                        Err(e) => {
                            error!("Failed to route. Error = {:?}", e);
                            continue;
                        }
                    }
                }
            }
        }
    }

    fn handle_connect(&mut self, connect: Connect, connection_handle: Sender<RouterMessage>) -> Result<Option<RouterMessage>, Error> {
        let id = connect.client_id;
        let clean_session = connect.clean_session;
        let will = connect.last_will;

        info!("Connect. Id = {:?}", id);
        let reply = if clean_session {
            self.inactive_connections.remove(&id);

            let state = MqttState::new(clean_session, will);
            self.active_connections.insert(id.clone(), ActiveConnection::new(connection_handle, state));
            Some(RouterMessage::Pending(VecDeque::new()))
        } else {
            if let Some(connection) = self.inactive_connections.remove(&id) {
                let pending = connection.state.outgoing_publishes.clone();
                self.active_connections.insert(id.clone(), ActiveConnection::new(connection_handle, connection.state));
                // Some(RouterMessage::Pending(pending))
                None
            } else {
                let state = MqttState::new(clean_session, will);
                self.active_connections.insert(id.clone(), ActiveConnection::new(connection_handle, state));
                Some(RouterMessage::Pending(VecDeque::new()))
            }
        };

        // TODO: clean_session specifics
        Ok(reply)
    }

    fn fill_commitlog(&mut self, publish: Publish) {
        if let Err(e) = self.commitlog.fill(publish) {
            error!("Failed to fill the commitlog. Error = {:?}", e);
        }
    }


    fn deactivate(&mut self, id: String) {
        info!("Deactivating client due to disconnect packet. Id = {}", id);

        if let Some(connection) = self.active_connections.remove(&id) {
            if !connection.state.clean_session {
                self.inactive_connections.insert(id, InactiveConnection::new(connection.state));
            }
        }
    }

    /// Saves state and sends network reply back to the connection
    fn handle_incoming_packet(&mut self, id: &str, packet: Packet) -> Result<Option<RouterMessage>, Error> {
        if let Some(connection) = self.active_connections.get_mut(id) {
            let reply = match connection.state.handle_incoming_mqtt_packet(packet) {
                Ok(Some(reply)) => reply,
                Ok(None) => return Ok(None),
                Err(state::Error::Unsolicited(packet)) => {
                    // NOTE: Some clients seems to be sending pending acks after reconnection
                    // even during a clean session. Let's be little lineant for now
                    error!("Unsolicited ack = {:?}. Id = {}", packet, id);
                    return Ok(None)
                }
                Err(e) => {
                    error!("State error = {:?}. Id = {}", e, id);
                    self.active_connections.remove(id);
                    return Err::<_, Error>(e.into())
                }
            };

            return Ok(Some(reply))
        }

        Ok(None)
    }

    fn forward(&mut self, id: &str, message: RouterMessage) {
        if let Some(connection) = self.active_connections.get_mut(id) {
            // slow connections should be moved to inactive connections. This drops tx handle of the
            // connection leading to connection disconnection
            if let Err(e) = connection.tx.try_send(message) {
                match e {
                    TrySendError::Full(_m) => {
                        error!("Slow connection during forward. Dropping handle and moving id to inactive list. Id = {}", id);
                        if let Some(connection) = self.active_connections.remove(id) {
                            self.inactive_connections.insert(id.to_owned(), InactiveConnection::new(connection.state));
                        }
                    }
                    TrySendError::Closed(_m) => {
                        error!("Closed connection. Forward failed");
                        self.active_connections.remove(id);
                    }
                }
            }
        }
    }
}




#[cfg(test)]
mod test {
    #[test]
    fn persistent_disconnected_and_dead_connections_are_moved_to_inactive_state() {}

    #[test]
    fn persistend_reconnections_are_move_from_inactive_to_active_state() {}

    #[test]
    fn offline_messages_are_given_back_to_reconnected_persistent_connection() {}

    #[test]
    fn remove_client_from_concrete_subsctiptions_if_new_wildcard_subscription_matches_existing_concrecte_subscription() {
        // client subscibing to a/b/c and a/+/c should receive message only once when
        // a publish happens on a/b/c
    }

    #[test]
    fn ingnore_new_concrete_subscription_if_a_matching_wildcard_subscription_exists_for_the_client() {}

    #[test]
    fn router_should_remove_the_connection_during_disconnect() {}

    #[test]
    fn router_should_not_add_same_client_to_subscription_list() {}

    #[test]
    fn router_saves_offline_messages_of_a_persistent_dead_connection() {}
} 
