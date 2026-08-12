package main

import (
	"path/filepath"
	"testing"
)

func TestGuessLanguage(t *testing.T) {
	tests := []struct {
		transcript string
		language   string
	}{
		{transcript: "What are the agents doing?", language: "en"},
		{transcript: "¿Qué están haciendo los agentes?", language: "es"},
		{transcript: "Consulta Herdr para mí", language: "es"},
	}
	for _, test := range tests {
		if actual := guessLanguage(test.transcript); actual != test.language {
			t.Fatalf("guessLanguage(%q) = %q, want %q", test.transcript, actual, test.language)
		}
	}
}

func TestStateDirUsesXDGStateHome(t *testing.T) {
	root := t.TempDir()
	t.Setenv("XDG_STATE_HOME", root)
	want := filepath.Join(root, "voice-orchestrator")
	if actual := stateDir(); actual != want {
		t.Fatalf("stateDir() = %q, want %q", actual, want)
	}
}

func TestSocketPathUsesXDGRuntimeDir(t *testing.T) {
	root := t.TempDir()
	t.Setenv("XDG_RUNTIME_DIR", root)
	want := filepath.Join(root, "voice-gateway.sock")
	if actual := socketPath(); actual != want {
		t.Fatalf("socketPath() = %q, want %q", actual, want)
	}
}

func TestVoiceResponseValidation(t *testing.T) {
	valid := voiceResponse{Spoken: "Done", Display: "Done", Language: "en", Actions: []string{}}
	if err := valid.validate(); err != nil {
		t.Fatalf("valid response rejected: %v", err)
	}
	invalid := valid
	invalid.Language = "fr"
	if err := invalid.validate(); err == nil {
		t.Fatal("unsupported language accepted")
	}
}
