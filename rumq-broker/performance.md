
Measuring throughput with benchmarking tool
---------

* start the broker
```
RUST_LOG=rumq_broker=info cargo run --release
```

* run the tool with the configuration you like
```
./benchmark -c 10 -m 10000 (open 10 connection with 10000 publishes each)
```

* results 
```
  99% |||||||||||||||||||||||||||||||||||||||.  [1s:0s]

Id = bench-0 Total Messages = 10000 Average throughput = 8.521107612817508 MB/s
Id = bench-5 Total Messages = 10009 Average throughput = 8.542670064545169 MB/s
Id = bench-2 Total Messages = 10001 Average throughput = 8.395775853675655 MB/s
Id = bench-8 Total Messages = 10001 Average throughput = 8.751427391693111 MB/s
Id = bench-1 Total Messages = 10000 Average throughput = 8.34767405859393 MB/s
Id = bench-9 Total Messages = 10000 Average throughput = 8.644405152661644 MB/s
Id = bench-4 Total Messages = 10001 Average throughput = 8.324409708196072 MB/s
Id = bench-6 Total Messages = 10004 Average throughput = 8.321790134298995 MB/s
Id = bench-3 Total Messages = 10005 Average throughput = 8.243539513023283 MB/s
Id = bench-7 Total Messages = 10003 Average throughput = 8.45395720459848 MB/s


 Total Messages =  100024 Average throughput =  80.49581882917525 MB/s
```

Journal
---------

* There doesn't seem to be any improvement wrt cpu and throughput using `BufStream`
* Using codec on top of `Read` has a huge perf benefit over async MqttRead/Write. There is 8X improvement in throughput


Moooree tools
----------

#####Flamegraph

```
RUSTFLAGS="-g -C force-frame-pointers=y" cargo flamegraph rumq-cli 
```

#####Perf

Build in release mode with debug symbols (-g) and frame pointers enabled

```
RUSTFLAGS="-g -C force-frame-pointers=y" cargo build --release
perf record -g <binary>
perf report -g graph,0.5,caller
```

References
-----------

* https://github.com/flamegraph-rs/flamegraph
* https://gendignoux.com/blog/2019/11/09/profiling-rust-docker-perf.html

