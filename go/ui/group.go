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
func GroupHover(options ...StyleDeclaration) StyleOption {
	return groupHoverOption("", options)
}

// GroupHoverNamed follows the nearest ancestor group with this name, passing
// over differently named or unnamed groups. No matching ancestor means no style.
func GroupHoverNamed(name string, options ...StyleDeclaration) StyleOption {
	name = strings.TrimSpace(name)
	if name == "" || len(name) > protocol.MaxHoverGroupNameBytes {
		panic(fmt.Sprintf("QuickGUI group names must contain 1 to %d bytes", protocol.MaxHoverGroupNameBytes))
	}
	return groupHoverOption(name, options)
}

func groupHoverOption(name string, options []StyleDeclaration) StyleOption {
	return func(style *Style) {
		rule := groupHoverRule{name: name}
		for _, option := range options {
			option.applyStyle(&rule.style)
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
	setGroupStyles(node, protocol.GroupHoverStyle, "groupHover", rules)
}

func setGroupStyles(node *native.Node, code uint16, state string, rules []groupHoverRule) {
	if len(rules) > 8 {
		panic("QuickGUI supports at most eight group style rules per state")
	}
	node.Bind(func() {
		type declaration struct {
			encodedStateStyle
			Group string `json:"group,omitempty"`
		}
		var declarations []declaration
		for _, rule := range rules {
			style, populated := encodeStateStyle(state, &rule.style)
			if populated {
				declarations = append(declarations, declaration{style, rule.name})
			}
		}
		if len(declarations) == 0 {
			native.ClearProperty(node, code)
			return
		}
		setJson(node, code, protocol.MaxStateStyleJSONBytes, declarations)
	})
}

// GroupActive applies paint styles while the nearest ancestor group is pressed.
func GroupActive(options ...StyleDeclaration) StyleOption {
	return groupActiveOption("", options)
}

// GroupActiveNamed follows the nearest ancestor group with this name while pressed.
func GroupActiveNamed(name string, options ...StyleDeclaration) StyleOption {
	name = strings.TrimSpace(name)
	if name == "" || len(name) > protocol.MaxHoverGroupNameBytes {
		panic(fmt.Sprintf("QuickGUI group names must contain 1 to %d bytes", protocol.MaxHoverGroupNameBytes))
	}
	return groupActiveOption(name, options)
}

func groupActiveOption(name string, options []StyleDeclaration) StyleOption {
	return func(style *Style) {
		rule := groupHoverRule{name: name}
		for _, option := range options {
			option.applyStyle(&rule.style)
		}
		style.groupActiveRules = append(groupActiveRules(*style), rule)
		style.GroupActive = nil
	}
}

func groupActiveRules(style Style) []groupHoverRule {
	var rules []groupHoverRule
	if style.GroupActive != nil {
		rules = append(rules, groupHoverRule{style: *style.GroupActive})
	}
	return append(rules, style.groupActiveRules...)
}
