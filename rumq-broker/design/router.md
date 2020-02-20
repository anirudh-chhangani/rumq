Router is a central stateful component in the design. It holds handles to all the active connections

Publishes shouldn't be held per connection
-------------------

While routing data from one connection to multiple connections, each
connection's mqtt state holds outgoing publishes and removes them when
ack is received. The current design looks like this

	- Router receives a publish from a connection
	- Router replys that connection after some state handling
	- Matches subscribers
	- adds the publish to connection state with correct qos and pkid
	- fills active & inactive connections publish queue with publish
	- Sends the batch to the connection when the interval is triggered
	- When a reconnection happens, all the messages filled during inactive state are forwarded to the connection

* This flow works but doesn't scale well wrt to memory and cpu
	- 1 copy per publish per connection with matching subscription while batching before forwarding to connection task
	- 1 copy per publish per connection state
	- move batched publishes to a channel

10 publish which matches 10 subscribers will cost 200 copies


Commit log
---------------------

Alternate to the above strategy is implementing an in memory commitlog inspired by kafka

	- Router receives a publish from a connection
	- Router fills the commitlog with the publish
	- When the timer interval is triggered, match subscribers
	- get n publish copies from the commitlog based on subscriber's current offset
	- add correct qos and pkid while fetching the copies from the commitlog
	- send the batch to the connection task over a channel 
	- update subscriber's offset if sending is successful
	- during a reconnection, resume from offset in the state

10 publish which matches 10 subscribers will cost 100 copies

The keeps memory usage constant limited by
	- commitlog's retention policy
	- router to connection's channel size

We don't have to copy data to inactive persistent connections and
bloating the memory per inactive connection with the hope that the
reconnection will happen soon

Subscriptions can go back in time like kafka


Routing data to clients can be a little tricky

The current architecture maps subsciptions to subscribers

```rust
#[derive(Debug)]
pub struct Router {
    commitlog: commitlog::CommitLog,
    active_connections:     HashMap<String, ActiveConnection>,
    concrete_subscriptions: HashMap<String, Vec<Subscriber>>,
    wild_subscriptions:     HashMap<String, Vec<Subscriber>>,
    data_rx:                Receiver<(String, RouterMessage)>,
}
```

Any new publish fills each active and inactive connection mapped by the
subscriber. This way, a connection is indirectly part of multiple topics via
subscribers. And the subscriber always will be part of subscriptions
regardless of the connection status of a connection

Now that sending data to an active connection is lazy, And offset is
part of the subscriber. All the inactive subscribers can be removed the
subscriptions

Maybe a better option would be to make each active connection hold the
subscriptions it is interested in. Since we anyway have to sweep all the
active connection


For every timer interval

* go through a subscription and all the subscribers in it
* get data from the commitlog based on subscriber offset
* get active connection from subscriber id
* send the data to the connection task and update commitlog offset

or

For every timer interval

* go through active connections and all the subscriptions in it
* get data from the commitlog for a given subscription
* send the data to the connection task and update commitlog offset


E.g

Total 100 connections with 1000 subscriptions and they are unevenly
distributed across connections. say connections 1..=10 hold all the
subscriptions. 100 each.

100 iterations. 1 per connections + 1000 iterations (1..=10 100
iterations each)

vs

1000 iterations. 1 per subscription. + each subscription holds 100
devices


