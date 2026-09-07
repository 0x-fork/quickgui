package ui

import (
	"bytes"
	"testing"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

func TestReadOnlySelectionControlsRefuseChanges(t *testing.T) {
	controls := []struct {
		name  string
		mount func(bool, bool, PartProps, func(bool, *native.Event)) *native.Node
	}{
		{"checkbox", func(readOnly, controlled bool, part PartProps, change func(bool, *native.Event)) *native.Node {
			props := CheckboxProps{PartProps: part, ReadOnly: readOnly, DefaultChecked: true, OnCheckedChange: change}
			if controlled {
				props.Checked = func() CheckedState { return true }
			}
			return Checkbox.Root(props)
		}},
		{"switch", func(readOnly, controlled bool, part PartProps, change func(bool, *native.Event)) *native.Node {
			props := SwitchProps{PartProps: part, ReadOnly: readOnly, DefaultChecked: true, OnCheckedChange: change}
			if controlled {
				props.Checked = func() bool { return true }
			}
			return Switch.Root(props)
		}},
		{"radio", func(readOnly, controlled bool, part PartProps, change func(bool, *native.Event)) *native.Node {
			props := RadioProps{PartProps: part, ReadOnly: readOnly, OnCheckedChange: change}
			if controlled {
				props.Checked = func() bool { return false }
			}
			return Radio.Root(props)
		}},
		{"radio-group", func(readOnly, controlled bool, part PartProps, change func(bool, *native.Event)) *native.Node {
			props := RadioGroupProps{
				ReadOnly: readOnly, DefaultValue: "first",
				OnValueChange: func(_ string, event *native.Event) { change(true, event) },
			}
			if controlled {
				props.Value = func() *string { value := "first"; return &value }
			}
			var radio *native.Node
			RadioGroup.Root(props, func() {
				radio = Radio.Root(RadioProps{PartProps: part, Value: "second"})
			})
			return radio
		}},
	}
	for _, control := range controls {
		t.Run(control.name, func(t *testing.T) {
			for _, controlled := range []bool{false, true} {
				for _, readOnly := range []bool{false, true} {
					native.ResetTreeStateForTests()
					reactive.CreateRoot(func(dispose func()) struct{} {
						defer dispose()
						clicks, changes := 0, 0
						node := control.mount(readOnly, controlled,
							PartProps{OnClick: func(*native.Event) { clicks++ }},
							func(bool, *native.Event) { changes++ },
						)
						root := node
						for root.Parent != nil {
							root = root.Parent
						}
						before := append([]byte(nil), root.Pending.Body()...)
						for _, listener := range node.Listeners {
							if listener.Type == protocol.EventClick {
								reactive.Batch(func() {
									listener.Listener(&native.Event{
										Type:   protocol.EventClick,
										Target: node,
									})
								})
							}
						}
						if clicks != 1 {
							t.Fatal("read-only must preserve the click event")
						}
						if readOnly {
							if changes != 0 || !bytes.Equal(before, root.Pending.Body()) {
								t.Errorf("controlled=%v: read-only activation changed state or emitted a change", controlled)
							}
						} else if changes != 1 {
							t.Errorf("controlled=%v: editable activation emitted %d changes", controlled, changes)
						}
						return struct{}{}
					})
				}
			}
		})
	}
}
