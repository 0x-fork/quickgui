package ui

import (
	"context"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

// Async runs work in a background goroutine and delivers its result on the UI
// goroutine. Disposing the component or calling the returned cancel function
// cancels the work and suppresses its completion callback.
func Async[T any](work func(context.Context) (T, error), done func(T, error)) context.CancelFunc {
	parent := reactive.GetOwner()
	owner := reactive.NewOwner(parent)
	ctx, cancel := context.WithCancel(context.Background())
	reactive.RunWithOwner(owner, func() struct{} {
		reactive.OnCleanup(cancel)
		return struct{}{}
	})
	go func() {
		value, err := work(ctx)
		native.Dispatch(func() {
			defer reactive.DisposeOwner(owner)
			if owner.Disposed || ctx.Err() != nil {
				return
			}
			reactive.RunWithOwner(parent, func() struct{} {
				done(value, err)
				return struct{}{}
			})
		})
	}()
	return cancel
}
