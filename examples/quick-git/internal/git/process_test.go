package git

import (
	"bytes"
	"context"
	"io"
	"os"
	"runtime"
	"testing"
	"time"
)

func TestProcessChild(t *testing.T) {
	if os.Getenv("QUICKGUI_PROCESS_TEST") != "1" {
		return
	}
	args := os.Args
	for len(args) > 0 && args[0] != "--" {
		args = args[1:]
	}
	switch args[1] {
	case "echo":
		io.Copy(os.Stdout, os.Stdin)
	case "literal":
		os.Stdout.WriteString(args[2])
	case "streams":
		os.Stdout.WriteString("out")
		os.Stderr.WriteString("err")
		os.Exit(7)
	case "flood":
		block := bytes.Repeat([]byte("x"), 4096)
		for {
			if _, err := os.Stdout.Write(block); err != nil {
				os.Exit(1)
			}
		}
	case "sleep":
		time.Sleep(10 * time.Second)
	}
	os.Exit(0)
}

func testProcess(ctx context.Context, mode string, input []byte, timeout time.Duration, maxOutput int, args ...string) Result {
	command := append([]string{os.Args[0], "-test.run=^TestProcessChild$", "--", mode}, args...)
	return runProcess(ctx, command, "", input, append(os.Environ(), "QUICKGUI_PROCESS_TEST=1"), timeout, maxOutput)
}

func TestProcessPreservesBinaryArgumentsStreamsAndExit(t *testing.T) {
	input := []byte{0, 10, 127, 255, 65}
	result := testProcess(context.Background(), "echo", input, time.Second, len(input))
	if result.ExitCode != 0 || !bytes.Equal(result.Stdout, input) || result.Truncated {
		t.Fatalf("binary roundtrip: %+v", result)
	}
	literal := "one \"quote\"; $(exit 9) `exit 8`"
	result = testProcess(context.Background(), "literal", nil, time.Second, 0, literal)
	if string(result.Stdout) != literal || result.ExitCode != 0 {
		t.Fatalf("literal args: %+v", result)
	}
	result = testProcess(context.Background(), "streams", nil, time.Second, 0)
	if string(result.Stdout) != "out" || result.Stderr != "err" || result.ExitCode != 7 {
		t.Fatalf("streams: %+v", result)
	}
}

func TestProcessBoundsTimeoutAndCancellation(t *testing.T) {
	result := testProcess(context.Background(), "flood", nil, 300*time.Millisecond, 17)
	if !result.Truncated || len(result.Stdout) != 17 || result.TimedOut || result.Aborted {
		t.Fatalf("output cap must stop process: %+v", result)
	}
	result = testProcess(context.Background(), "sleep", nil, 30*time.Millisecond, 0)
	if !result.TimedOut || result.Aborted {
		t.Fatalf("deadline: %+v", result)
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	result = testProcess(ctx, "sleep", nil, time.Second, 0)
	if !result.Aborted || result.TimedOut {
		t.Fatalf("pre-cancel: %+v", result)
	}
	ctx, cancel = context.WithCancel(context.Background())
	defer cancel()
	timer := time.AfterFunc(30*time.Millisecond, cancel)
	defer timer.Stop()
	result = testProcess(ctx, "sleep", nil, time.Second, 0)
	if !result.Aborted || result.TimedOut {
		t.Fatalf("active cancel: %+v", result)
	}
}

func TestRunnerCancellationDoesNotLoseQueueCapacity(t *testing.T) {
	runner := NewRunner(1)
	for i := 0; i < 256; i++ {
		requireOK(t, runner.acquire(context.Background(), true))
		ctx, cancel := context.WithCancel(context.Background())
		done := make(chan error, 1)
		go func() {
			err := runner.acquire(ctx, true)
			if err == nil {
				runner.release()
			}
			done <- err
		}()
		for {
			runner.mu.Lock()
			queued := len(runner.waiters) == 1
			runner.mu.Unlock()
			if queued {
				break
			}
			runtime.Gosched()
		}
		if i%2 == 0 {
			cancel()
			runner.release()
		} else {
			runner.release()
			cancel()
		}
		<-done
		runner.mu.Lock()
		inFlight, waiting := runner.inFlight, len(runner.waiters)
		runner.mu.Unlock()
		if inFlight != 0 || waiting != 0 {
			t.Fatalf("cancelled request leaked capacity: active=%d queued=%d", inFlight, waiting)
		}
	}
}
