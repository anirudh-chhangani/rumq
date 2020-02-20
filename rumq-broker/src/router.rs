use derive_more::From;
use rumq_core::mqtt4::{has_wildcards, matches, publish, QoS, Packet, Connect, Publish, Subscribe, SubscribeTopic, Unsubscribe};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::mpsc::error::TrySendError;
use tokio::select;
use tokio::time::{self, Duration};
use tokio::stream::StreamExt;

use std::collections::{HashMap, VecDeque};
use std::mem;
use std::fmt;

use crate::state::{self, MqttState};

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
    Publishes(Vec<Publish>),
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
struct ActiveConnection {
    pub state: MqttState,
    pub retained: Vec<Publish>,
    concrete_subscriptions: HashMap<String, Subscription>,
    wild_subscriptions:     HashMap<String, Subscription>,
    tx: Sender<RouterMessage>
}

impl ActiveConnection {
    pub fn new(tx: Sender<RouterMessage>, state: MqttState) -> ActiveConnection {
        ActiveConnection {
            state,
            retained: Vec::new(),
            concrete_subscriptions: HashMap::new(),
            wild_subscriptions: HashMap::new(),
            tx
        }
    }

    pub fn add_to_subscriptions(&mut self, subscribe: Subscribe, retained_publishes: Vec<Publish>) {
        self.retained.extend(retained_publishes);
        // Each subscribe message can send multiple topics to subscribe to. handle dupicates
        for topic in subscribe.topics {
            let mut filter = topic.topic_path.clone();
            let qos = topic.qos;
            let subscriber = Subscription::new(qos);

            let subscriber = if let Some((f, subscriber)) = self.fix_overlapping_subscriptions(&filter, qos) {
                filter = f;
                subscriber
            } else {
                subscriber
            };

            // a publish happens on a/b/c.
            let subscriptions =
                if has_wildcards(&filter) { &mut self.wild_subscriptions } else { &mut self.concrete_subscriptions };

            subscriptions.insert(filter.to_owned(), subscriber);
        }
    }

    /// removes the subscriber from subscription if the current subscription is wider than the
    /// existing subscription and returns it
    ///
    /// if wildcard subscription:
    /// move subscriber from concrete to wild subscription with greater qos
    /// move subscriber from existing wild to current wild subscription if current is wider
    /// move subscriber from current wild to existing wild if the existing wild is wider
    /// returns the subscriber and the wild subscription it is to be added to
    /// none implies that there are no overlapping subscriptions for this subscriber
    /// new subscriber a/+/c (qos 1) matches existing subscription a/b/c
    /// subscriber should be moved from a/b/c to a/+/c
    ///
    /// * if the new subscription is wider than existing subscription, move the subscriber to wider
    /// subscription with highest qos
    ///
    /// * any new wildcard subsciption checks for matching concrete subscriber
    /// * if matches, add the subscriber to `wild_subscriptions` with greatest qos
    ///
    /// * any new concrete subscriber checks for matching wildcard subscriber
    /// * if matches, add the subscriber to `wild_subscriptions` (instead of concrete subscription) with greatest qos
    ///
    /// coming to overlapping wildcard subscriptions
    ///
    /// * new subsciber-a a/+/c/d  mathes subscriber-a in a/#
    /// * add subscriber-a to a/# directly with highest qos
    ///
    /// * new subscriber a/# matches with existing a/+/c/d
    /// * remove subscriber from a/+/c/d and move it to a/# with highest qos
    ///
    /// * finally a subscriber won't be part of multiple subscriptions
    fn fix_overlapping_subscriptions(&mut self, current_filter: &str, qos: QoS) -> Option<(String, Subscription)> {
        let mut filter = current_filter.to_owned();
        let mut qos = qos;
        let mut prune_list = Vec::new();

        // subscriber in concrete_subscriptions a/b/c/d matchs new subscription a/+/c/d on same
        // subscriber. move it from concrete to wild
        if has_wildcards(current_filter) {
            for (existing_filter, subscription) in self.concrete_subscriptions.iter_mut() {
                if matches(existing_filter, current_filter) {
                    prune_list.push(existing_filter.clone());
                    if subscription.qos > qos {
                        qos = subscription.qos
                    }
                }
            }

            for (existing_filter, subscription) in self.wild_subscriptions.iter_mut() {
                // current filter is wider than existing filter. remove subscriber (if it exists)
                // from current filter
                if matches(existing_filter, current_filter) {
                    prune_list.push(existing_filter.clone());
                    if subscription.qos > qos {
                        qos = subscription.qos
                    }
                } else if matches(current_filter, existing_filter) {
                    // existing filter is wider than current filter, return the subscriber with
                    // wider subscription (to be added outside this method)
                    filter = existing_filter.to_owned();
                    prune_list.push(existing_filter.clone());
                    // remove the subscriber and update the global qos (this subscriber will be
                    // added again outside)

                    if subscription.qos > qos {
                        qos = subscription.qos
                    }
                }
            }
        }

        if prune_list.len() > 0 {
            for filter in prune_list.into_iter() {
                let _ = self.concrete_subscriptions.remove(&filter);
                let _ = self.wild_subscriptions.remove(&filter);
            }

            let subscription = Subscription::new(qos);
            Some((filter, subscription))
        } else {
            None
        }
    }

    pub fn remove_from_subscriptions(&mut self, unsubscribe: Unsubscribe) {
        for topic in unsubscribe.topics.iter() {
            if has_wildcards(topic) {
                self.wild_subscriptions.remove(topic);
            } else {
                self.wild_subscriptions.remove(topic);
            };
        }
    }
}

#[derive(Debug)]
struct InactiveConnection {
    pub state: MqttState
}

impl InactiveConnection {
    pub fn new(state: MqttState) -> InactiveConnection {
        InactiveConnection {
            state,
        }
    }
}

#[derive(Debug, Clone)]
struct Subscription {
    qos: QoS,
}

impl Subscription {
    pub fn new(qos: QoS) -> Subscription {
        Subscription {
            qos,
        }
    }
}

#[derive(Debug)]
pub struct Router {
    commitlog: HashMap<String, Vec<Publish>>,
    // handles to all active connections. used to route data
    active_connections:     HashMap<String, ActiveConnection>,
    // inactive persistent connections
    inactive_connections:   HashMap<String, InactiveConnection>,
    // retained publishes
    retained_publishes:     HashMap<String, Publish>,
    // channel receiver to receive data from all the active_connections.
    // each connection will have a tx handle
    data_rx:                Receiver<(String, RouterMessage)>,
}

impl Router {
    pub fn new(data_rx: Receiver<(String, RouterMessage)>) -> Self {
        Router {
            commitlog: HashMap::new(),
            active_connections: HashMap::new(),
            inactive_connections: HashMap::new(),
            retained_publishes: HashMap::new(),
            data_rx,
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let mut interval = time::interval(Duration::from_millis(10));

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
                _ = interval.next() => {
                    self.route()
                }
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
                    Packet::Subscribe(subscribe) => {
                        let retained_publishes = self.match_retainted_publishes(&subscribe.topics);
                        if let Some(connection) = self.active_connections.get_mut(&id) {
                            connection.add_to_subscriptions(subscribe, retained_publishes);
                        }
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

    fn fill_commitlog(&mut self, publish: Publish) {
        if publish.retain {
            if publish.payload.len() == 0 {
                self.retained_publishes.remove(&publish.topic_name);
                return
            } else {
                self.retained_publishes.insert(publish.topic_name.clone(), publish.clone());
            }
        }

        if let Some(publishes) = self.commitlog.get_mut(&publish.topic_name) {
            publishes.push(publish)
        } else {
            // create a new partition with this new topic
            let topic = publish.topic_name.clone();
            let mut publishes = Vec::new();
            publishes.push(publish);
            self.commitlog.insert(topic, publishes);
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
                /*
                   let pending = connection.state.outgoing_publishes.clone();
                   self.active_connections.insert(id.clone(), ActiveConnection::new(connection_handle, connection.state));
                   Some(RouterMessage::Pending(pending))
                   */
                None
            } else {
                let state = MqttState::new(clean_session, will);
                self.active_connections.insert(id.clone(), ActiveConnection::new(connection_handle, state));
                Some(RouterMessage::Pending(VecDeque::new()))
            }
        };


        Ok(reply)
    }

    fn route(&mut self) {
        let active_connections = &mut self.active_connections;
        let mut closed_connections = Vec::new();
        let graveyard = &mut closed_connections;

        for (id, connection) in active_connections.iter_mut() {
            if connection.retained.len() > 0 {
                let mut publishes = connection.retained.split_off(0);
                // TODO: Use correct qos
                connection.state.handle_outgoing_publishes(QoS::AtLeastOnce, &mut publishes);
                let _ = connection.tx.try_send(RouterMessage::Publishes(publishes));
            }

            let concrete_subscriptions = &mut connection.concrete_subscriptions;
            let commitlog = &self.commitlog;
            for (filter, subscription) in concrete_subscriptions.iter_mut() {
                let qos = subscription.qos;
                if let Some(publishes) = commitlog.get(filter) {
                    let mut publishes = publishes.clone();
                    connection.state.handle_outgoing_publishes(qos, &mut publishes);
                    match connection.tx.try_send(RouterMessage::Publishes(publishes)) {
                        Ok(_) => continue,
                        Err(TrySendError::Full(_)) => {
                            error!("Routint to a closed connection. Id = {:?}", id);
                            graveyard.push(id.clone());
                            continue;
                        }
                        Err(TrySendError::Closed(_)) => {
                            error!("Routint to a closed connection. Id = {:?}", id);
                            graveyard.push(id.clone());
                            continue;
                        }
                    }
                }
            }
        }

        mem::replace(&mut self.commitlog, HashMap::new());

        for id in closed_connections.into_iter() {
            if let Some(connection) = active_connections.remove(&id) {
                self.inactive_connections.insert(id.to_owned(), InactiveConnection::new(connection.state));
            }
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

    fn deactivate_and_forward_will(&mut self, id: String) {
        info!("Deactivating client due to connection death. Id = {}", id);

        if let Some(mut connection) = self.active_connections.remove(&id) {
            if let Some(mut will) = connection.state.will.take() {
                let topic = mem::replace(&mut will.topic, "".to_owned());
                let message = mem::replace(&mut will.message, "".to_owned());
                let qos = will.qos;

                let publish = publish(topic, qos, message);
                self.fill_commitlog(publish);
            }

            if !connection.state.clean_session {
                self.inactive_connections.insert(id.clone(), InactiveConnection::new(connection.state));
            }
        }
    }

    fn match_retainted_publishes(&self, subscribe: &Vec<SubscribeTopic>) -> Vec<Publish> {
        let mut publishes = Vec::new();
        for s in subscribe.iter() {
            let filter = &s.topic_path;
            let qos = s.qos;
            let retained_publishes = &self.retained_publishes;
            // Handle retained publishes after subscription duplicates are handled above
            if has_wildcards(filter) {
                for (topic, publish) in retained_publishes.iter() {
                    if matches(&topic, filter) {
                        let mut publish = publish.clone();
                        publish.qos = qos;
                        publishes.push(publish);
                    }
                }
            } else {
                if let Some(publish) = retained_publishes.get(filter) {
                    let mut publish = publish.clone();
                    publish.qos = qos;
                    publishes.push(publish);
                }
            }
        }

        publishes
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
