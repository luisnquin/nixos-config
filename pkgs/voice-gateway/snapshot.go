package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strconv"
	"sync"
)

type herdrAgent struct {
	Agent         string `json:"agent"`
	Status        string `json:"agent_status"`
	Cwd           string `json:"cwd"`
	Focused       bool   `json:"focused"`
	PaneID        string `json:"pane_id"`
	TerminalTitle string `json:"terminal_title_stripped"`
	WorkspaceID   string `json:"workspace_id"`
}

type herdrAgentList struct {
	Result struct {
		Agents []herdrAgent `json:"agents"`
	} `json:"result"`
}

type herdrAgentGet struct {
	Result struct {
		Agent herdrAgent `json:"agent"`
	} `json:"result"`
}

type snapshotAgent struct {
	Agent        herdrAgent `json:"agent"`
	ReadSource   string     `json:"read_source"`
	RecentOutput string     `json:"recent_output"`
	Error        string     `json:"error,omitempty"`
}

type herdrSnapshot struct {
	Session string          `json:"session"`
	Agents  []snapshotAgent `json:"agents"`
}

func printSnapshot() error {
	session := envOr("VOICE_HERDR_SESSION", "hub")
	output, err := runHerdr(session, "agent", "list")
	if err != nil {
		return err
	}
	var list herdrAgentList
	if err := json.Unmarshal(output, &list); err != nil {
		return fmt.Errorf("decode Herdr agent list: %w", err)
	}

	items := make([]snapshotAgent, len(list.Result.Agents))
	var workers sync.WaitGroup
	for index := range list.Result.Agents {
		index := index
		workers.Add(1)
		go func() {
			defer workers.Done()
			items[index] = inspectHerdrAgent(session, list.Result.Agents[index])
		}()
	}
	workers.Wait()

	encoder := json.NewEncoder(os.Stdout)
	encoder.SetEscapeHTML(false)
	return encoder.Encode(herdrSnapshot{Session: session, Agents: items})
}

func inspectHerdrAgent(session string, listed herdrAgent) snapshotAgent {
	item := snapshotAgent{Agent: listed}
	output, err := runHerdr(session, "agent", "get", listed.PaneID)
	if err != nil {
		item.Error = err.Error()
		return item
	}
	var exact herdrAgentGet
	if err := json.Unmarshal(output, &exact); err != nil {
		item.Error = fmt.Sprintf("decode Herdr agent state: %v", err)
		return item
	}
	item.Agent = exact.Result.Agent
	item.ReadSource = "recent-unwrapped"
	lines := 120
	if item.Agent.Status == "working" {
		item.ReadSource = "visible"
		lines = 80
	}
	output, err = runHerdr(
		session,
		"agent", "read", item.Agent.PaneID,
		"--source", item.ReadSource,
		"--lines", strconv.Itoa(lines),
	)
	if err != nil {
		item.Error = err.Error()
		return item
	}
	item.RecentOutput = string(output)
	return item
}

func runHerdr(session string, arguments ...string) ([]byte, error) {
	commandArguments := append([]string{"--session", session}, arguments...)
	command := exec.Command(envOr("VOICE_HERDR_BIN", "herdr"), commandArguments...)
	output, err := command.Output()
	if err == nil {
		return output, nil
	}
	if exitErr, ok := err.(*exec.ExitError); ok {
		return nil, fmt.Errorf("herdr %v: %s", arguments, exitErr.Stderr)
	}
	return nil, fmt.Errorf("herdr %v: %w", arguments, err)
}
