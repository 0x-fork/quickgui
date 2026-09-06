import type { NativeNode } from "@quickgui/native";
import { For, Text, createSignal } from "@quickgui/ui";
import {
  Autocomplete,
  Combobox,
  Select,
  useComboboxChips,
  useComboboxState,
  useSelectState,
  type OptionDeclaration,
} from "@quickgui/ui/select";

import { controlStyle, inputStyle, p, pickerAppearance } from "../theme.ts";
import { Note, Panel } from "../ui.tsx";

/* -------------------------------------------------------------------------------------------- *
 * Autocomplete
 * -------------------------------------------------------------------------------------------- */

const componentNames: OptionDeclaration[] = [
  { value: "window", label: "Window" },
  { value: "widget", label: "Widget" },
  { value: "tabs", label: "Tabs" },
  { value: "table", label: "Table" },
  { value: "toolbar", label: "Toolbar" },
  { value: "tooltip", label: "Tooltip" },
];

export function AutocompleteDemo(): NativeNode {
  const [query, setQuery] = createSignal("");
  const [committed, setCommitted] = createSignal("—");
  const [open, setOpen] = createSignal(false);
  return (
    <Panel
      title="Autocomplete"
      hint="Free-form text with suggestions. The core filters, ranks, and paints the rows in its own popover window."
    >
      <Autocomplete.Root
        ariaLabel="Search components"
        placeholder="Type “win”, “tab”, or “tool”…"
        filterMode="fuzzy"
        items={() => componentNames}
        appearance={pickerAppearance()}
        onInputValueChange={(next) => setQuery(next)}
        onOpenChange={(next) => setOpen(next)}
        onCommit={(details) => setCommitted(details.value ?? details.inputValue ?? "—")}
        style={[inputStyle(), { width: 280 }]}
      />
      <Note text={'query "' + query() + '" · popover ' + String(open()) + " · committed " + committed()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Combobox
 * -------------------------------------------------------------------------------------------- */

const fruits: OptionDeclaration[] = [
  { value: "apple", label: "Apple", group: "Common" },
  { value: "banana", label: "Banana", group: "Common" },
  { value: "lychee", label: "Lychee", group: "Tropical" },
  { value: "mango", label: "Mango", group: "Tropical" },
];

const languages: OptionDeclaration[] = [
  { value: "rust", label: "Rust" },
  { value: "zig", label: "Zig" },
  { value: "swift", label: "Swift" },
  { value: "typescript", label: "TypeScript" },
];

function ComboboxChipList(): NativeNode {
  const chips = useComboboxChips();
  return (
    <Combobox.Chips style={{ display: "flex", flexDirection: "row", gap: 6 }}>
      <For each={chips}>
        {(chip, index) => (
          <Combobox.Chip
            index={index()}
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              gap: 4,
              paddingLeft: 8,
              paddingRight: 6,
              height: 22,
              borderRadius: 11,
              backgroundColor: p().selection,
            }}
          >
            <Text style={{ fontSize: 11, color: p().ink }}>{chip.label}</Text>
            <Combobox.ChipRemove index={index()} ariaLabel={"Remove " + chip.label}>
              <Text style={{ fontSize: 11, color: p().muted }}>×</Text>
            </Combobox.ChipRemove>
          </Combobox.Chip>
        )}
      </For>
    </Combobox.Chips>
  );
}

interface ComboboxReadoutProps {
  selected: () => string;
  chips: () => string[];
}

function ComboboxReadout(props: ComboboxReadoutProps): NativeNode {
  const state = useComboboxState();
  return (
    <Note
      text={"value " + props.selected() + " · tags [" + props.chips().join(", ") + "] · open " + String(state().popupOpen) + " · results " + String(state().resultCount)}
    />
  );
}

export function ComboboxDemo(): NativeNode {
  const [fruit, setFruit] = createSignal<string | undefined>("apple");
  const [tags, setTags] = createSignal<string[]>(["rust"]);
  return (
    <Panel title="Combobox" hint="A constrained picker over a declared option source, and a multiple combobox whose chips come back from the core.">
      <Combobox.Root
        ariaLabel="Fruit"
        placeholder="Pick a fruit"
        value={fruit}
        autoHighlight
        filterMode="contains"
        items={() => fruits}
        appearance={pickerAppearance()}
        onValueChange={(next) => setFruit(next)}
        style={[inputStyle(), { width: 240 }]}
      />
      <Combobox.Root
        ariaLabel="Tags"
        multiple
        values={tags}
        onValuesChange={(next) => setTags(next)}
        filterMode="startsWith"
        autoHighlight
        placeholder="Add tags"
        appearance={pickerAppearance()}
        items={() => languages}
        style={[inputStyle(), { width: 300, height: 34, display: "flex", flexDirection: "row", alignItems: "center" }]}
      >
        <ComboboxChipList />
        <ComboboxReadout selected={() => fruit() ?? ""} chips={tags} />
      </Combobox.Root>
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Select
 * -------------------------------------------------------------------------------------------- */

const themes: OptionDeclaration[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark", detail: "⌘D" },
  { value: "system", label: "Match system" },
];

const sizes: OptionDeclaration[] = [
  { value: "s", label: "Small" },
  { value: "m", label: "Medium" },
  { value: "l", label: "Large" },
  { value: "xl", label: "Extra large" },
];

interface SelectValueTextProps {
  text: () => string;
  placeholder: string;
}

function SelectValueText(props: SelectValueTextProps): NativeNode {
  const state = useSelectState();
  return (
    <Text style={{ fontSize: 12, color: state().placeholder ? p().muted : p().ink }}>{state().placeholder ? props.placeholder : props.text()}</Text>
  );
}

function SelectStateLine(): NativeNode {
  const state = useSelectState();
  return (
    <Note
      text={"popup " + String(state().popupOpen) + " · side " + state().popupSide + " · filled " + String(state().filled) + " · touched " + String(state().touched)}
    />
  );
}

export function SelectDemo(): NativeNode {
  const [theme, setTheme] = createSignal<string | undefined>("system");
  const [picked, setPicked] = createSignal<string[]>(["m"]);
  return (
    <Panel
      title="Select"
      hint="A native popover window the core renders itself from the declared appearance. The multiple select keeps the declared order."
    >
      <Select.Root
        ariaLabel="Theme"
        value={theme}
        items={() => themes}
        appearance={pickerAppearance()}
        onValueChange={(next) => setTheme(next)}
        style={[controlStyle(), { width: 210, justifyContent: "space-between" }]}
      >
        <Select.Value>
          <Text style={{ fontSize: 12, color: p().ink }}>{theme() ?? ""}</Text>
        </Select.Value>
        <Select.Icon>
          <Text style={{ fontSize: 11, color: p().muted }}>▾</Text>
        </Select.Icon>
      </Select.Root>

      <Select.Root
        ariaLabel="Sizes"
        multiple
        values={picked}
        onValuesChange={(next) => setPicked(next)}
        alignItemWithTrigger
        appearance={pickerAppearance()}
        items={() => sizes}
        style={[controlStyle(), { width: 210, justifyContent: "space-between" }]}
      >
        <Select.Value>
          <SelectValueText text={() => picked().join(", ")} placeholder="Pick sizes" />
        </Select.Value>
        <Select.Icon>
          <Text style={{ fontSize: 11, color: p().muted }}>▾</Text>
        </Select.Icon>
        <Select.Positioner side="bottom" align="start" sideOffset={6}>
          <Select.ScrollUpArrow />
          <Select.ScrollDownArrow />
        </Select.Positioner>
        <SelectStateLine />
      </Select.Root>
      <Note text={"theme " + (theme() ?? "—") + " · sizes [" + picked().join(", ") + "]"} />
    </Panel>
  );
}
