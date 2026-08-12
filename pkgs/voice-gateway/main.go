package main

import (
	"encoding/json"
	"fmt"
	"os"
)

func main() {
	if err := run(os.Args[1:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run(arguments []string) error {
	if len(arguments) == 0 {
		return usageError()
	}
	switch arguments[0] {
	case "serve":
		if len(arguments) != 1 {
			return usageError()
		}
		return newGateway().serve()
	case "submit":
		if len(arguments) != 2 {
			return usageError()
		}
		response, err := sendRequest(gatewayRequest{Type: "submit", Transcript: arguments[1]})
		return checkGatewayResponse(response, err)
	case "cancel":
		if len(arguments) != 1 {
			return usageError()
		}
		response, err := sendRequest(gatewayRequest{Type: "cancel"})
		return checkGatewayResponse(response, err)
	case "snapshot":
		if len(arguments) != 1 {
			return usageError()
		}
		return printSnapshot()
	default:
		return usageError()
	}
}

func checkGatewayResponse(response gatewayResponse, err error) error {
	if err != nil {
		return err
	}
	if response.Status == "accepted" || response.Status == "cancelled" {
		return nil
	}
	encoded, marshalErr := json.Marshal(response)
	if marshalErr != nil {
		return marshalErr
	}
	return fmt.Errorf("voice gateway rejected request: %s", encoded)
}

func usageError() error {
	return fmt.Errorf("usage: voice-gateway <serve|submit|cancel|snapshot> [transcript]")
}
