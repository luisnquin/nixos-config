package main

import (
	"os"
	"os/signal"
)

func signalNotifyImpl(channel chan<- os.Signal, signals ...os.Signal) {
	signal.Notify(channel, signals...)
}

func signalStopImpl(channel chan<- os.Signal) {
	signal.Stop(channel)
}
