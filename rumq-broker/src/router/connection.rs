use super::RouterMessage;
use crate::state::MqttState;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

use rumq_core::mqtt4::{has_wildcards, matches, QoS, Subscribe, Unsubscribe};

#[derive(Debug)]
pub struct ActiveConnection {
    pub state: MqttState,
    pub tx: Sender<RouterMessage>,
    pub concrete_subscriptions: HashMap<String, Subscription>,
    wild_subscriptions: HashMap<String, Subscription>,
}

impl ActiveConnection {
    pub fn new(tx: Sender<RouterMessage>, state: MqttState) -> ActiveConnection {
        ActiveConnection { state, tx, concrete_subscriptions: HashMap::new(), wild_subscriptions: HashMap::new() }
    }

    pub fn add_to_subscriptions(&mut self, subscribe: Subscribe) {
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
                    filter = existing_filter.clone();
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
pub struct InactiveConnection {
    pub state: MqttState,
}

impl InactiveConnection {
    pub fn new(state: MqttState) -> InactiveConnection {
        InactiveConnection { state }
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub qos: QoS,
    pub offset: (u64, usize),
}

impl Subscription {
    pub fn new(qos: QoS) -> Subscription {
        Subscription { qos, offset: (0, 0) }
    }

    pub fn update_offset(&mut self, segment_id: u64, log_offset: usize) {
        self.offset = (segment_id, log_offset)
    }

    pub fn offset(&self) -> (u64, usize) {
        self.offset
    }
}
