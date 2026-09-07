package native

import (
	"encoding/json"
	"fmt"
)

func decodeServiceReply[T any](method string, done func(T, error)) func(string, error) {
	return func(raw string, err error) {
		var value T
		if err == nil {
			if decodeErr := json.Unmarshal([]byte(raw), &value); decodeErr != nil {
				err = fmt.Errorf("%s returned an invalid response: %w", method, decodeErr)
			}
		}
		if done != nil {
			done(value, err)
		}
	}
}

func invokeJSON[T any](method string, params any, done func(T, error)) {
	Invoke(method, params, decodeServiceReply(method, done))
}

func invokeVoid(method string, params any, done func(error)) {
	Invoke(method, params, func(_ string, err error) {
		if done != nil {
			done(err)
		}
	})
}

func commandJSON[T any](payload any, done func(T, error)) {
	data, err := json.Marshal(payload)
	if err != nil {
		if done != nil {
			var zero T
			done(zero, err)
		}
		return
	}
	SendCommand(string(data), decodeServiceReply("native command", done))
}

func commandVoid(payload any, done func(error)) {
	data, err := json.Marshal(payload)
	if err != nil {
		if done != nil {
			done(err)
		}
		return
	}
	SendCommand(string(data), func(_ string, err error) {
		if done != nil {
			done(err)
		}
	})
}

// Synchronous calls are limited to core services that do not wait on native
// main-thread execution. Desktop/window requests use the queued command path.
func callJSON[T any](method string, params any) (T, error) {
	var value T
	data, err := json.Marshal(params)
	if err != nil {
		return value, err
	}
	raw, err := CallService(method, string(data))
	if err != nil {
		return value, err
	}
	if err := json.Unmarshal([]byte(raw), &value); err != nil {
		return value, fmt.Errorf("%s returned an invalid response: %w", method, err)
	}
	return value, nil
}
