package main

import (
	"bufio"
	"context"
	"fmt"
	"log"
	"net/http"
	"os/signal"
	"syscall"
)

func main() {
	client := &(*http.DefaultClient)
	ctx := context.Background()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, "http://parrot.live", http.NoBody)
	if err != nil {
		log.Fatal(err)
	}

	req.Header.Add("User-Agent", "curl/8.4.0")
	req.Header.Add("Accept", "*/*")

	resp, err := client.Do(req)
	if err != nil {
		log.Fatal(err)
	}

	ctx, stop := signal.NotifyContext(ctx, syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	go func() {
		<-ctx.Done()
		resp.Body.Close()
	}()

	s := bufio.NewScanner(resp.Body)

	for s.Scan() {
		fmt.Println(s.Text())
	}
}
