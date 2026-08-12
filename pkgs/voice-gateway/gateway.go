package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"time"
)

type gateway struct {
	app      *appServer
	requests chan string
	stopping atomic.Bool
	workers  sync.WaitGroup
}

func newGateway() *gateway {
	return &gateway{app: newAppServer(), requests: make(chan string, 16)}
}

func (g *gateway) serve() error {
	path := socketPath()
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return err
	}
	_ = os.Remove(path)
	listener, err := net.ListenUnix("unix", &net.UnixAddr{Name: path, Net: "unix"})
	if err != nil {
		return err
	}
	defer func() {
		_ = listener.Close()
		_ = os.Remove(path)
	}()
	if err := os.Chmod(path, 0o600); err != nil {
		return err
	}

	g.workers.Add(1)
	go g.work()

	signals := make(chan os.Signal, 1)
	signalNotify(signals)
	defer signalStop(signals)

	for !g.stopping.Load() {
		if err := listener.SetDeadline(time.Now().Add(500 * time.Millisecond)); err != nil {
			return err
		}
		connection, acceptErr := listener.AcceptUnix()
		if acceptErr != nil {
			var netErr net.Error
			if errors.As(acceptErr, &netErr) && netErr.Timeout() {
				select {
				case <-signals:
					g.stopping.Store(true)
				default:
				}
				continue
			}
			return acceptErr
		}
		g.accept(connection)
	}

	close(g.requests)
	g.app.close(true)
	g.workers.Wait()
	return nil
}

func (g *gateway) accept(connection *net.UnixConn) {
	defer connection.Close()
	_ = connection.SetDeadline(time.Now().Add(5 * time.Second))
	payload, err := io.ReadAll(io.LimitReader(connection, 1<<20))
	if err != nil {
		g.writeResponse(connection, gatewayResponse{Status: "error", Message: err.Error()})
		return
	}
	var request gatewayRequest
	if err := json.Unmarshal(payload, &request); err != nil {
		g.writeResponse(connection, gatewayResponse{Status: "error", Message: err.Error()})
		return
	}

	switch request.Type {
	case "submit":
		if strings.TrimSpace(request.Transcript) == "" {
			g.writeResponse(connection, gatewayResponse{Status: "error", Message: "empty transcript"})
			return
		}
		select {
		case g.requests <- request.Transcript:
			g.writeResponse(connection, gatewayResponse{Status: "accepted"})
		default:
			g.writeResponse(connection, gatewayResponse{Status: "busy", Message: "voice queue is full"})
		}
	case "cancel":
		g.app.close(true)
		for {
			select {
			case <-g.requests:
			default:
				g.writeResponse(connection, gatewayResponse{Status: "cancelled"})
				return
			}
		}
	default:
		g.writeResponse(connection, gatewayResponse{Status: "error", Message: "unknown request type"})
	}
}

func (g *gateway) writeResponse(connection *net.UnixConn, response gatewayResponse) {
	encoded, err := json.Marshal(response)
	if err == nil {
		_, _ = connection.Write(encoded)
	}
}

func (g *gateway) work() {
	defer g.workers.Done()
	for transcript := range g.requests {
		response, err := g.app.runTurn(transcript)
		if err != nil {
			if errors.Is(err, errTurnCancelled) || g.stopping.Load() {
				continue
			}
			g.app.close(false)
			deliverError(transcript, err)
			continue
		}
		deliver(transcript, response)
	}
}

func deliver(transcript string, response voiceResponse) {
	historyPath := filepath.Join(stateDir(), "history.jsonl")
	if err := os.MkdirAll(filepath.Dir(historyPath), 0o700); err == nil {
		if stream, openErr := os.OpenFile(historyPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0o600); openErr == nil {
			_ = json.NewEncoder(stream).Encode(historyEntry{
				Timestamp: time.Now().Unix(), Transcript: transcript, Response: response,
			})
			_ = stream.Close()
		}
	}
	fmt.Println(response.Display)
	_ = exec.Command(envOr("VOICE_REPLY_BIN", "voice-reply"), "--lang", response.Language, response.Spoken).Run()
}

func deliverError(transcript string, detail error) {
	language := guessLanguage(transcript)
	spoken := "I could not complete that request. Check the orchestrator log."
	if language == "es" {
		spoken = "No pude completar esa solicitud. Revisa el registro del orquestador."
	}
	fmt.Fprintf(os.Stderr, "voice-orchestrator error: %v\n", detail)
	_ = exec.Command(envOr("VOICE_REPLY_BIN", "voice-reply"), "--lang", language, spoken).Run()
}

func sendRequest(request gatewayRequest) (gatewayResponse, error) {
	payload, err := json.Marshal(request)
	if err != nil {
		return gatewayResponse{}, err
	}
	var lastErr error
	for attempt := 0; attempt < 20; attempt++ {
		connection, dialErr := net.DialTimeout("unix", socketPath(), 250*time.Millisecond)
		if dialErr != nil {
			lastErr = dialErr
			time.Sleep(100 * time.Millisecond)
			continue
		}
		_ = connection.SetDeadline(time.Now().Add(5 * time.Second))
		var response gatewayResponse
		if _, err = connection.Write(payload); err == nil {
			if unixConnection, ok := connection.(*net.UnixConn); ok {
				_ = unixConnection.CloseWrite()
			}
			err = json.NewDecoder(connection).Decode(&response)
		}
		_ = connection.Close()
		if err == nil {
			return response, nil
		}
		lastErr = err
		time.Sleep(100 * time.Millisecond)
	}
	return gatewayResponse{}, fmt.Errorf("voice gateway unavailable: %w", lastErr)
}

func stateDir() string {
	if root := os.Getenv("XDG_STATE_HOME"); root != "" {
		return filepath.Join(root, "voice-orchestrator")
	}
	home, _ := os.UserHomeDir()
	return filepath.Join(home, ".local", "state", "voice-orchestrator")
}

func socketPath() string {
	runtimeDir := os.Getenv("XDG_RUNTIME_DIR")
	if runtimeDir == "" {
		runtimeDir = fmt.Sprintf("/run/user/%d", os.Getuid())
	}
	return filepath.Join(runtimeDir, "voice-gateway.sock")
}

func guessLanguage(transcript string) string {
	lowered := " " + strings.ToLower(transcript) + " "
	for _, marker := range []string{"¿", "¡", " que ", " los ", " las ", " una ", " por ", " para ", " está "} {
		if strings.Contains(lowered, marker) {
			return "es"
		}
	}
	return "en"
}

func envOr(name string, fallback string) string {
	if value := os.Getenv(name); value != "" {
		return value
	}
	return fallback
}

var signalNotify = func(channel chan<- os.Signal) {
	signalNotifyImpl(channel, syscall.SIGINT, syscall.SIGTERM)
}

var signalStop = func(channel chan<- os.Signal) {
	signalStopImpl(channel)
}
