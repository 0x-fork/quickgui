package main

import (
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/reactive"
)

func TestEveryDemoMountsWithoutNativeHost(t *testing.T) {
	for _, entry := range demos {
		t.Run(entry.ID, func(t *testing.T) {
			native.ResetTreeStateForTests()
			reactive.CreateRoot(func(dispose func()) struct{} {
				defer dispose()
				roots := native.CollectChildren(entry.Component)
				if len(roots) != 1 {
					t.Fatalf("expected one demo root, got %d", len(roots))
				}
				root := roots[0]
				if root == nil || root.Pending == nil || root.Pending.Empty() {
					t.Fatal("demo did not declare a component tree")
				}
				return struct{}{}
			})
		})
	}
}
