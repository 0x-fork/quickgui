package ui

// StyleBuilder composes reusable styles with the same fluent modifiers as
// Element. Each modifier returns a new value, leaving shared styles unchanged.
// Accessors remain unevaluated until the style is applied to a node.
type StyleBuilder struct {
	style styleData
}

// Style starts a reusable fluent style. Apply it with
// Element.Style or a compound part's Style field.
func Style() StyleBuilder { return StyleBuilder{} }

// Merge adds styles in order. Later properties override earlier ones;
// interaction styles merge their properties without changing the inputs.
func (style StyleBuilder) Merge(styles ...StyleBuilder) StyleBuilder {
	for _, other := range styles {
		other.applyStyle(&style.style)
	}
	return style
}

func (style StyleBuilder) configure(declarations ...styleOption) StyleBuilder {
	for _, declaration := range declarations {
		declaration(&style.style)
	}
	return style
}

// When applies build when condition is true. The condition is a setup value;
// use Element.When or a compound part's style accessor for reactive conditions.
func (style StyleBuilder) When(condition bool, build func(StyleBuilder) StyleBuilder) StyleBuilder {
	if condition {
		return build(style)
	}
	return style
}

func (style StyleBuilder) apply(props *Props)           { style.applyStyle(&props.Style.style) }
func (style StyleBuilder) applyStyle(target *styleData) { mergeStyle(target, style.style) }

// Flex enables flex layout, or sets grow, shrink 1, and zero basis with a value.
func (style StyleBuilder) Flex(value ...any) StyleBuilder {
	switch len(value) {
	case 0:
		return style.Display("flex")
	case 1:
		return style.configure(styleFlexGrow(value[0]), styleFlexShrink(1), styleFlexBasis(0))
	default:
		panic("QuickGUI Flex accepts zero or one value")
	}
}

// FlexWrap enables wrapping, or sets an explicit wrapping mode.
func (style StyleBuilder) FlexWrap(value ...string) StyleBuilder {
	switch len(value) {
	case 0:
		return style.configure(styleFlexWrap("wrap"))
	case 1:
		return style.configure(styleFlexWrap(value[0]))
	default:
		panic("QuickGUI FlexWrap accepts zero or one value")
	}
}

func (style StyleBuilder) Bg(value any) StyleBuilder { return style.BackgroundColor(value) }

func (style StyleBuilder) GroupHover(build func(StyleBuilder) StyleBuilder) StyleBuilder {
	return style.configure(styleGroupHover(build(Style())))
}

func (style StyleBuilder) GroupHoverNamed(name string, build func(StyleBuilder) StyleBuilder) StyleBuilder {
	return style.configure(styleGroupHoverNamed(name, build(Style())))
}

func (style StyleBuilder) GroupActive(build func(StyleBuilder) StyleBuilder) StyleBuilder {
	return style.configure(styleGroupActive(build(Style())))
}

func (style StyleBuilder) GroupActiveNamed(name string, build func(StyleBuilder) StyleBuilder) StyleBuilder {
	return style.configure(styleGroupActiveNamed(name, build(Style())))
}
