package main

import (
	"fmt"
	"sync/atomic"
	"time"
	"math/rand"
	arg "github.com/alexflint/go-arg" 
	progressbar "github.com/schollz/progressbar/v2"
	mqtt "github.com/eclipse/paho.mqtt.golang"
)


var opts struct {
	Connections int `arg:"-c" help:"Number of connections"`
	Messages int `arg:"-m" help:"Number of messages per connection"`
	PayloadSize int `arg:"-s" help:"Size of each message"`
}


func init() {
	opts.Connections = 1
	opts.Messages = 10000
	opts.PayloadSize = 1024

	arg.MustParse(&opts)
}

func data(n int) string {
	const letterBytes = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"

	b := make([]byte, n)
	for i := range b {
		b[i] = letterBytes[rand.Intn(len(letterBytes))]
	}

	return string(b)
}

type Statistics struct {
	id string
	timeTaken time.Duration
	totalMessageCount uint64
	totalSize float64
}

func NewStatiscs(id string, timeTaken time.Duration, count uint64, totalSize float64) Statistics {
	return Statistics {
		id: id,
		timeTaken: timeTaken,
		totalMessageCount: count,
		totalSize: totalSize,
	}
}


type Connection struct {
	id string
	total int
	client mqtt.Client
	stats chan Statistics
	progress chan uint64
}

func NewConnection(id string, total int, stats chan Statistics, progress chan uint64) *Connection {
	connLostHandler := func(c mqtt.Client, err error) {
		fmt.Printf("Connection lost, reason: %v\n", err)
	}

	opts := mqtt.NewClientOptions().AddBroker("tcp://localhost:1883")
	opts.SetClientID(id)
	opts.SetProtocolVersion(4)
	opts.SetCleanSession(true)
	opts.SetKeepAlive(30 * time.Second)
	opts.SetMaxReconnectInterval(10 * time.Second)
	opts.SetConnectionLostHandler(connLostHandler)

	c := mqtt.NewClient(opts)
	if token := c.Connect(); token.Wait() && token.Error() != nil {
		panic(token.Error())
	}

	return &Connection {
		id: id,
		client: c,
		total: total,
		stats: stats,
		progress: progress,
	}
}


var totalCount uint64 = 0

func (c *Connection) Start() {
	var counter uint64
	var start = time.Now()
	exit := make(chan bool, 10)

	msgHandler := func(client mqtt.Client, msg mqtt.Message) {
		count := atomic.AddUint64(&counter, 1)
		atomic.AddUint64(&totalCount, 1)
		if count == uint64(c.total) {
			exit <- true
		}
	}


	if token := c.client.Subscribe("hello/mqtt/rumqtt", 1, msgHandler); token.Wait() && token.Error() != nil {
		panic(token.Error())
	}

	go func() {
		text := data(opts.PayloadSize)
		for i := 0; i < c.total ; i++ {
			token := c.client.Publish("hello/mqtt/rumqtt", 1, false, text)
			token.Wait()
		}
	}()


	<-exit
	totalSize := float64(c.total * opts.PayloadSize)
	statistics := NewStatiscs(c.id, time.Since(start), counter, totalSize)
	c.stats <- statistics 
	return	
}

func main() {
	exit := make(chan Statistics, 10)
	progress := make(chan uint64, 1000)
	totalMessages :=  opts.Connections * opts.Messages
	totalConnectionsDone := 0
	progressbar := progressbar.NewOptions(totalMessages, progressbar.OptionSetTheme(progressbar.Theme{Saucer: "|", SaucerPadding: "."}))
	results := make([]Statistics, 0)
	var start = time.Now()

	for i := 0; i < opts.Connections; i++ {
		id := fmt.Sprintf("bench-%v", i)
		connection := NewConnection(id, opts.Messages, exit, progress)
		go connection.Start()
	}

	L:
	for {
		select {
		case  statistics := <-exit:
			results = append(results, statistics)
			totalConnectionsDone += 1
			if totalConnectionsDone >= opts.Connections {
				fmt.Println("\n")
				break L
			}
		case <-time.After(10 * time.Millisecond):
			c := atomic.LoadUint64(&totalCount);
			progressbar.Set(int(c))
		}
	}

	c := int(atomic.LoadUint64(&totalCount))
	progressbar.Set(c)

	// size in MB
	totalSize := float64(c * opts.PayloadSize ) / 1024.0 / 1024.0
	// time in seconds
	timeTaken := time.Since(start).Seconds()

	time.Sleep(1 * time.Second)
	for _, statistics := range results {
		fmt.Println("Id =", statistics.id, "Total Messages =", statistics.totalMessageCount, "Average throughput =", statistics.totalSize/1024.0/1024.0/statistics.timeTaken.Seconds(), "MB/s")
	}

	fmt.Println("\n\n Total Messages = ", c, "Average throughput = ", totalSize/timeTaken, "MB/s")
}
