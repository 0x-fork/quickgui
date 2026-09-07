package ui

import (
	"fmt"
	"strings"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
)

type groupHoverRule struct {
	name  string
	style Style
}

// GroupHover applies paint styles while the nearest ancestor group is hovered.
// Repeated declarations accumulate; later matching declarations win.
func GroupHover(options ...StyleOption) StyleOption {
	return groupHoverOption("", options)
}

// GroupHoverNamed follows the nearest ancestor group with this name, passing
// over differently named or unnamed groups. No matching ancestor means no style.
func GroupHoverNamed(name string, options ...StyleOption) StyleOption {
	name = strings.TrimSpace(name)
	if name == "" || len(name) > protocol.MaxHoverGroupNameBytes {
		panic(fmt.Sprintf("QuickGUI group names must contain 1 to %d bytes", protocol.MaxHoverGroupNameBytes))
	}
	return groupHoverOption(name, options)
}

func groupHoverOption(name string, options []StyleOption) StyleOption {
	return func(style *Style) {
		rule := groupHoverRule{name: name}
		for _, option := range options {
			option(&rule.style)
		}
		style.groupHoverRules = append(groupHoverRules(*style), rule)
		style.GroupHover = nil
	}
}

// Copy before appending so reusing a Style cannot change another node's rules.
func groupHoverRules(style Style) []groupHoverRule {
	var rules []groupHoverRule
	if style.GroupHover != nil {
		rules = append(rules, groupHoverRule{style: *style.GroupHover})
	}
	return append(rules, style.groupHoverRules...)
}

func setGroupHoverStyles(node *native.Node, rules []groupHoverRule) {
	node.Bind(func() {
		type declaration struct {
			encodedStateStyle
			Group string `json:"group,omitempty"`
		}
		var declarations []declaration
		for _, rule := range rules {
			style, populated := encodeStateStyle("groupHover", &rule.style)
			if populated {
				declarations = append(declarations, declaration{style, rule.name})
			}
		}
		if len(declarations) == 0 {
			native.ClearProperty(node, protocol.GroupHoverStyle)
			return
		}
		setJson(node, protocol.GroupHoverStyle, protocol.MaxStateStyleJSONBytes, declarations)
	})
}
