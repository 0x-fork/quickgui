package main

import (
	"bytes"
	"context"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

func launcherDefinitions() []launcher {
	return []launcher{
		{ID: "codex", Label: "Codex", Mark: "C", Description: "OpenAI Codex CLI"},
		{ID: "claude", Label: "Claude", Mark: "A", Description: "Anthropic Claude Code"},
		{ID: "opencode", Label: "OpenCode", Mark: "O", Description: "OpenCode terminal agent"},
	}
}
func loginShell() string {
	if shell := os.Getenv("SHELL"); shell != "" {
		return shell
	}
	return "/bin/zsh"
}
func environmentMap(entries []string) map[string]string {
	env := map[string]string{}
	for _, entry := range entries {
		if name, value, ok := strings.Cut(entry, "="); ok && name != "" && !strings.ContainsAny(name, "\n\r") {
			env[name] = value
		}
	}
	return env
}
func environmentList(env map[string]string) []string {
	out := make([]string, 0, len(env))
	for name, value := range env {
		out = append(out, name+"="+value)
	}
	return out
}

// Shell startup output is bounded and never printed: it can contain credentials.
type boundedOutput struct{ bytes.Buffer }

func (b *boundedOutput) Write(p []byte) (int, error) {
	if b.Len()+len(p) > 1024*1024 {
		return 0, io.ErrShortBuffer
	}
	return b.Buffer.Write(p)
}
func captureShellEnvironment(ctx context.Context) map[string]string {
	inherited := environmentMap(os.Environ())
	ctx, cancel := context.WithTimeout(ctx, 4*time.Second)
	defer cancel()
	command := exec.CommandContext(ctx, loginShell(), "-i", "-l", "-c", "/usr/bin/env -0")
	env := environmentMap(os.Environ())
	env["DISABLE_AUTO_UPDATE"] = "true"
	env["DISABLE_UPDATE_PROMPT"] = "true"
	env["ZSH_DISABLE_COMPFIX"] = "true"
	command.Env = environmentList(env)
	command.WaitDelay = 250 * time.Millisecond
	var output boundedOutput
	command.Stdout = &output
	if command.Run() != nil {
		return inherited
	}
	for name, value := range environmentMap(strings.Split(output.String(), "\x00")) {
		inherited[name] = value
	}
	return inherited
}
func resolveLaunchers(env map[string]string, home string) []launcher {
	directories := append(filepath.SplitList(env["PATH"]), filepath.SplitList(os.Getenv("PATH"))...)
	for _, suffix := range []string{".local/bin", ".bun/bin", ".cargo/bin", ".local/share/mise/shims", ".volta/bin"} {
		directories = append(directories, filepath.Join(home, suffix))
	}
	directories = append(directories, "/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin")
	result := launcherDefinitions()
	for i := range result {
		for _, directory := range directories {
			// Do not resolve an implicit current-directory executable from an empty PATH entry.
			if directory == "" {
				continue
			}
			candidate := filepath.Join(directory, result[i].ID)
			if info, err := os.Stat(candidate); err == nil && info.Mode().IsRegular() && info.Mode().Perm()&0111 != 0 {
				result[i].Executable = candidate
				break
			}
		}
	}
	return result
}
