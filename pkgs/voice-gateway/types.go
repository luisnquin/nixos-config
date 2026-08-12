package main

import "encoding/json"

const responseSchema = `{
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "spoken": {"type": "string"},
    "display": {"type": "string"},
    "language": {"type": "string", "enum": ["en", "es"]},
    "actions": {"type": "array", "items": {"type": "string"}}
  },
  "required": ["spoken", "display", "language", "actions"]
}`

const developerInstructions = `You are the persistent voice control plane for one user.
Answer in the transcript language, English or Spanish. Use the voice-orchestrator
skill whenever the request concerns agents or Herdr. Agents are independent peers;
never invent parent-child relationships. Keep spoken output brief and conversational.
Put operational detail in display. The response schema is authoritative.`

type rpcMessage struct {
	ID     *int64          `json:"id,omitempty"`
	Method string          `json:"method,omitempty"`
	Params json.RawMessage `json:"params,omitempty"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  json.RawMessage `json:"error,omitempty"`
}

type clientInfo struct {
	Name    string `json:"name"`
	Title   string `json:"title"`
	Version string `json:"version"`
}

type initializeParams struct {
	ClientInfo clientInfo `json:"clientInfo"`
}

type threadParams struct {
	Model                 string `json:"model"`
	Cwd                   string `json:"cwd"`
	ApprovalPolicy        string `json:"approvalPolicy"`
	Sandbox               string `json:"sandbox"`
	Personality           string `json:"personality"`
	DeveloperInstructions string `json:"developerInstructions"`
	ThreadID              string `json:"threadId,omitempty"`
}

type threadResult struct {
	Thread struct {
		ID string `json:"id"`
	} `json:"thread"`
}

type persistedThread struct {
	ID    string `json:"id"`
	Turns int    `json:"turns"`
}

type turnInput struct {
	Type string `json:"type"`
	Name string `json:"name,omitempty"`
	Path string `json:"path,omitempty"`
	Text string `json:"text,omitempty"`
}

type turnStartParams struct {
	ThreadID     string          `json:"threadId"`
	Model        string          `json:"model"`
	Effort       string          `json:"effort"`
	Input        []turnInput     `json:"input"`
	OutputSchema json.RawMessage `json:"outputSchema"`
}

type itemCompletedParams struct {
	Item struct {
		Type string `json:"type"`
		Text string `json:"text"`
	} `json:"item"`
}

type voiceResponse struct {
	Spoken   string   `json:"spoken"`
	Display  string   `json:"display"`
	Language string   `json:"language"`
	Actions  []string `json:"actions"`
}

type gatewayRequest struct {
	Type       string `json:"type"`
	Transcript string `json:"transcript,omitempty"`
}

type gatewayResponse struct {
	Status  string `json:"status"`
	Message string `json:"message,omitempty"`
}

type historyEntry struct {
	Timestamp  int64         `json:"timestamp"`
	Transcript string        `json:"transcript"`
	Response   voiceResponse `json:"response"`
}
