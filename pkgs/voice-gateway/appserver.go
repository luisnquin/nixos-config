package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"sync"
	"sync/atomic"
)

var errTurnCancelled = errors.New("voice turn cancelled")

type appServer struct {
	mu          sync.Mutex
	command     *exec.Cmd
	stdin       io.WriteCloser
	stdout      *bufio.Reader
	requestID   int64
	threadID    string
	threadTurns int
	cancelled   atomic.Bool
	threadFile  string
}

func newAppServer() *appServer {
	return &appServer{threadFile: filepath.Join(stateDir(), "thread.json")}
}

func (a *appServer) start() error {
	a.mu.Lock()
	if a.command != nil && a.command.Process != nil && a.command.ProcessState == nil {
		a.mu.Unlock()
		return nil
	}
	a.mu.Unlock()

	command := exec.Command(envOr("VOICE_CODEX_BIN", "codex"), "app-server", "--listen", "stdio://")
	stdin, err := command.StdinPipe()
	if err != nil {
		return err
	}
	stdout, err := command.StdoutPipe()
	if err != nil {
		return err
	}
	command.Stderr = os.Stderr
	if err := command.Start(); err != nil {
		return err
	}

	a.mu.Lock()
	a.command = command
	a.stdin = stdin
	a.stdout = bufio.NewReader(stdout)
	a.mu.Unlock()
	a.cancelled.Store(false)

	params, err := json.Marshal(initializeParams{ClientInfo: clientInfo{
		Name: "voice_orchestrator", Title: "Voice Orchestrator", Version: "0.1.0",
	}})
	if err != nil {
		return err
	}
	if _, err := a.request("initialize", params); err != nil {
		a.close(false)
		return err
	}
	if err := a.send("initialized", json.RawMessage(`{}`), nil); err != nil {
		a.close(false)
		return err
	}
	if err := a.openThread(); err != nil {
		a.close(false)
		return err
	}
	return nil
}

func (a *appServer) close(cancelled bool) {
	if cancelled {
		a.cancelled.Store(true)
	}

	a.mu.Lock()
	command := a.command
	stdin := a.stdin
	a.command = nil
	a.stdin = nil
	a.stdout = nil
	a.mu.Unlock()

	if stdin != nil {
		_ = stdin.Close()
	}
	if command != nil && command.Process != nil && command.ProcessState == nil {
		_ = command.Process.Kill()
		_ = command.Wait()
	}
}

func (a *appServer) send(method string, params json.RawMessage, requestID *int64) error {
	a.mu.Lock()
	stdin := a.stdin
	a.mu.Unlock()
	if stdin == nil {
		return errors.New("Codex App Server is not running")
	}
	line, err := json.Marshal(rpcMessage{ID: requestID, Method: method, Params: params})
	if err != nil {
		return err
	}
	line = append(line, '\n')
	_, err = stdin.Write(line)
	return err
}

func (a *appServer) read() (rpcMessage, error) {
	a.mu.Lock()
	stdout := a.stdout
	a.mu.Unlock()
	if stdout == nil {
		return rpcMessage{}, errors.New("Codex App Server is not running")
	}
	line, err := stdout.ReadBytes('\n')
	if err != nil {
		if a.cancelled.Load() {
			return rpcMessage{}, errTurnCancelled
		}
		return rpcMessage{}, fmt.Errorf("read Codex App Server output: %w", err)
	}
	var message rpcMessage
	if err := json.Unmarshal(line, &message); err != nil {
		return rpcMessage{}, fmt.Errorf("decode Codex App Server output: %w", err)
	}
	return message, nil
}

func (a *appServer) request(method string, params json.RawMessage) (json.RawMessage, error) {
	requestID := atomic.AddInt64(&a.requestID, 1)
	if err := a.send(method, params, &requestID); err != nil {
		return nil, err
	}
	for {
		message, err := a.read()
		if err != nil {
			return nil, err
		}
		if message.ID == nil || *message.ID != requestID {
			continue
		}
		if hasRPCError(message.Error) {
			return nil, fmt.Errorf("Codex App Server %s: %s", method, message.Error)
		}
		return message.Result, nil
	}
}

func hasRPCError(raw json.RawMessage) bool {
	return len(raw) > 0 && !bytes.Equal(bytes.TrimSpace(raw), []byte("null"))
}

func (a *appServer) threadParams() threadParams {
	home, _ := os.UserHomeDir()
	return threadParams{
		Model:                 envOr("VOICE_CODEX_MODEL", "gpt-5.6-terra"),
		Cwd:                   home,
		ApprovalPolicy:        "never",
		Sandbox:               "danger-full-access",
		Personality:           "pragmatic",
		DeveloperInstructions: developerInstructions,
	}
}

func (a *appServer) openThread() error {
	if err := os.MkdirAll(filepath.Dir(a.threadFile), 0o700); err != nil {
		return err
	}
	if existing, err := os.ReadFile(a.threadFile); err == nil {
		var state persistedThread
		if json.Unmarshal(existing, &state) == nil && state.ID != "" && state.Turns < maxThreadTurns() {
			params := a.threadParams()
			params.ThreadID = state.ID
			encoded, marshalErr := json.Marshal(params)
			if marshalErr != nil {
				return marshalErr
			}
			if result, requestErr := a.request("thread/resume", encoded); requestErr == nil {
				return a.rememberThread(result, state.Turns)
			}
			_ = os.Remove(a.threadFile)
		}
	}

	return a.startThread()
}

func (a *appServer) startThread() error {
	encoded, err := json.Marshal(a.threadParams())
	if err != nil {
		return err
	}
	result, err := a.request("thread/start", encoded)
	if err != nil {
		return err
	}
	return a.rememberThread(result, 0)
}

func (a *appServer) rememberThread(raw json.RawMessage, turns int) error {
	var result threadResult
	if err := json.Unmarshal(raw, &result); err != nil {
		return err
	}
	if result.Thread.ID == "" {
		return errors.New("Codex returned an empty thread ID")
	}
	a.threadID = result.Thread.ID
	a.threadTurns = turns
	return a.writeThreadState()
}

func (a *appServer) writeThreadState() error {
	encoded, err := json.Marshal(persistedThread{ID: a.threadID, Turns: a.threadTurns})
	if err != nil {
		return err
	}
	encoded = append(encoded, '\n')
	if err := os.WriteFile(a.threadFile, encoded, 0o600); err != nil {
		return err
	}
	_ = os.Remove(filepath.Join(stateDir(), "thread-id"))
	return nil
}

func maxThreadTurns() int {
	turns, err := strconv.Atoi(envOr("VOICE_THREAD_MAX_TURNS", "12"))
	if err != nil || turns < 1 {
		return 12
	}
	return turns
}

func (a *appServer) runTurn(transcript string) (voiceResponse, error) {
	if err := a.start(); err != nil {
		return voiceResponse{}, err
	}
	if a.threadTurns >= maxThreadTurns() {
		if err := a.startThread(); err != nil {
			return voiceResponse{}, err
		}
	}
	if a.threadID == "" {
		return voiceResponse{}, errors.New("voice thread was not initialized")
	}

	params := turnStartParams{
		ThreadID: a.threadID,
		Model:    envOr("VOICE_CODEX_MODEL", "gpt-5.6-terra"),
		Effort:   envOr("VOICE_CODEX_EFFORT", "low"),
		Input: []turnInput{
			{Type: "skill", Name: "voice-orchestrator", Path: os.Getenv("VOICE_SKILL_PATH")},
			{Type: "text", Text: transcript},
		},
		OutputSchema: json.RawMessage(responseSchema),
	}
	encoded, err := json.Marshal(params)
	if err != nil {
		return voiceResponse{}, err
	}
	requestID := atomic.AddInt64(&a.requestID, 1)
	if err := a.send("turn/start", encoded, &requestID); err != nil {
		return voiceResponse{}, err
	}

	started := false
	finalMessage := ""
	for {
		message, err := a.read()
		if err != nil {
			return voiceResponse{}, err
		}
		if message.ID != nil && *message.ID == requestID {
			if hasRPCError(message.Error) {
				return voiceResponse{}, fmt.Errorf("start voice turn: %s", message.Error)
			}
			started = true
			continue
		}
		switch message.Method {
		case "item/completed":
			var params itemCompletedParams
			if err := json.Unmarshal(message.Params, &params); err != nil {
				return voiceResponse{}, err
			}
			if params.Item.Type == "agentMessage" && params.Item.Text != "" {
				finalMessage = params.Item.Text
			}
		case "turn/completed":
			if !started {
				return voiceResponse{}, errors.New("turn completed before turn/start response")
			}
			if finalMessage == "" {
				return voiceResponse{}, errors.New("Codex returned no final agent message")
			}
			var response voiceResponse
			if err := json.Unmarshal([]byte(finalMessage), &response); err != nil {
				return voiceResponse{}, fmt.Errorf("decode voice response: %w", err)
			}
			if err := response.validate(); err != nil {
				return voiceResponse{}, err
			}
			a.threadTurns++
			if err := a.writeThreadState(); err != nil {
				fmt.Fprintf(os.Stderr, "persist voice thread: %v\n", err)
			}
			return response, nil
		}
	}
}

func (r voiceResponse) validate() error {
	if r.Spoken == "" || r.Display == "" {
		return errors.New("Codex returned an incomplete voice response")
	}
	if r.Language != "en" && r.Language != "es" {
		return fmt.Errorf("Codex returned unsupported language %q", r.Language)
	}
	return nil
}
