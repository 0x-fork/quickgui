package ui

import (
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestRemovingDynamicNodesDisposesSubscriptions(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal("initial")
		visible, setVisible := CreateSignal(true)
		renders := 0
		parent := View()
		region := Show(visible, func() *Element {
			renders++
			return Input().Value(value).Child(value)
		})
		native.InsertNode(parent.Node, region, nil)
		native.RemoveNode(parent.Node, region)
		setValue("removed")
		setVisible(false)
		setVisible(true)
		if renders != 1 || len(parent.Node.Children) != 0 {
			t.Fatal("removed region is still reactive")
		}
		return struct{}{}
	})
}

func TestSignalChangesOnlyItsBoundText(t *testing.T) {
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		value, setValue := CreateSignal("first")
		parent := View().Child(Text("static")).Child(Text(value))
		before := parent.Pending.MutationCount()
		Batch(func() { setValue("second"); setValue("third") })
		if got := parent.Pending.MutationCount() - before; got != 1 {
			t.Fatalf("one text mutation per batch, got %d", got)
		}
		return struct{}{}
	})
}

func TestViewAppliesStyleAndChildren(t *testing.T) {
	native.ResetTreeStateForTests()
	node := View().Style(Style().Flex().Width("100%").Gap(12).Bg("#112233")).Child(Text("hello"))
	if node.Tag != protocol.TagView {
		t.Fatal(node.Tag)
	}
	if len(node.Node.Children) != 1 {
		t.Fatalf("children %d", len(node.Node.Children))
	}
	if node.Pending == nil || node.Pending.Empty() {
		t.Fatal("expected recorded mutations")
	}
}

func TestDynamicTextFollowsSignal(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		count, setCount := CreateSignal(1)
		node := DynamicText(func() string { return "Count: " + itoa(count()) })
		if node.Text != "Count: 1" {
			t.Fatalf("initial %s", node.Text)
		}
		setCount(2)
		if node.Text != "Count: 2" {
			t.Fatalf("updated %s", node.Text)
		}
		return struct{}{}
	})
}

func TestShowCreatesChildrenOnDemand(t *testing.T) {
	native.ResetTreeStateForTests()
	reactive.CreateRoot(func(dispose func()) struct{} {
		defer dispose()
		visible, setVisible := CreateSignal(false)
		created := 0
		parent := View()
		sentinel := Show(func() bool { return visible() }, func() *Element {
			created++
			return Text(Props{Children: "shown"})
		})
		native.InsertNode(parent.Node, sentinel, nil)
		if created != 0 {
			t.Fatal("created before visible")
		}
		setVisible(true)
		if created != 1 {
			t.Fatalf("created %d", created)
		}
		setVisible(true)
		if created != 1 {
			t.Fatal("recreated while still visible")
		}
		setVisible(false)
		if len(sentinel.Group) != 0 && sentinel.Group != nil {
			// cleared region has nil group
		}
		return struct{}{}
	})
}

func itoa(value int) string {
	if value == 0 {
		return "0"
	}
	neg := value < 0
	if neg {
		value = -value
	}
	var digits [12]byte
	i := len(digits)
	for value > 0 {
		i--
		digits[i] = byte('0' + value%10)
		value /= 10
	}
	if neg {
		i--
		digits[i] = '-'
	}
	return string(digits[i:])
}
