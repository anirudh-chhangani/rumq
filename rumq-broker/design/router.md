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

Router should scale across threads, processes and machines
---------------------


