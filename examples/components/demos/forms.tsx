import type { NativeNode } from "@quickgui/native";
import { For, Input, Show, Text, View, createSignal, type Style } from "@quickgui/ui";
import { OtpField } from "@quickgui/ui/base-ui";
import { Checkbox, Radio, RadioGroup, Switch, type CheckedState } from "@quickgui/ui/controls";
import { Calendar, DateField, TimeField } from "@quickgui/ui/date-time";
import { Field, Fieldset } from "@quickgui/ui/field";
import { NumberField, useNumberFieldState } from "@quickgui/ui/number-field";

import { controlStyle, inputStyle, p } from "../theme.ts";
import { Btn, Col, Label, Muted, Note, Panel, Row } from "../ui.tsx";

/* -------------------------------------------------------------------------------------------- *
 * Field
 * -------------------------------------------------------------------------------------------- */

export function FieldDemo(): NativeNode {
  const [email, setEmail] = createSignal("");
  const [triggers, setTriggers] = createSignal("—");
  const valid = (): boolean => email().includes("@");
  return (
    <Panel
      title="Field"
      hint="One control plus its label, description, error, and live validity. validationMode and validationDebounceTime are answered by the core through onValidationChange."
    >
      <Field.Root
        invalid={() => !valid()}
        required
        filled={() => email().length > 0}
        validationMessage={() => "Enter an address containing @"}
        validationMode="onChange"
        validationDebounceTime={200}
        onValidationChange={(validation) =>
          setTriggers(
            "change " + String(validation.triggers.change) + " · blur " + String(validation.triggers.blur) + " · submit " + String(validation.triggers.submit) + " · debounce " + String(validation.delay.change) + "ms",
          )
        }
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <Field.Label>
          <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>Email</Text>
        </Field.Label>
        <Field.Control
          value={email}
          placeholder="you@example.com"
          onInput={(event) => setEmail(event.value ?? "")}
          style={[inputStyle(), { width: 280 }]}
        />
        <Field.Description>
          <Muted text="We never share it." />
        </Field.Description>
        <Field.Error>
          <Text style={{ fontSize: 12, color: p().danger }}>Enter an address containing @</Text>
        </Field.Error>
        <Field.Validity>
          <Muted text={valid() ? "valid" : "invalid"} />
        </Field.Validity>
      </Field.Root>
      <Note text={'value "' + email() + '" · ' + triggers()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Fieldset
 * -------------------------------------------------------------------------------------------- */

function switchShell(on: boolean): Style {
  return {
    width: 40,
    height: 22,
    borderRadius: 11,
    padding: 2,
    backgroundColor: on ? p().accent : p().track,
    focus: { outline: "2px solid " + p().accent },
  };
}

export function FieldsetDemo(): NativeNode {
  const [saving, setSaving] = createSignal(false);
  const [name, setName] = createSignal("Ada");
  const [org, setOrg] = createSignal("Analytical Engines");
  return (
    <Panel
      title="Fieldset"
      hint="A semantic group with a legend. A Field.Root inside it inherits the group's disabled state, so one flag disables the whole section."
    >
      <Row>
        <Switch.Root checked={saving} onCheckedChange={(next) => setSaving(next)} style={switchShell(saving())}>
          <Switch.Thumb style={{ width: 18, height: 18, borderRadius: 9, backgroundColor: "#ffffff", marginLeft: saving() ? 18 : 0 }} />
        </Switch.Root>
        <Muted text={saving() ? "saving: the fieldset is disabled" : "editable"} />
      </Row>
      <Fieldset.Root
        disabled={saving}
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 10,
          padding: 14,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
          opacity: saving() ? 0.55 : 1,
        }}
      >
        <Fieldset.Legend>
          <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>Account</Text>
        </Fieldset.Legend>
        <Fieldset.Description>
          <Muted text="Both fields inherit the group's disabled state." />
        </Fieldset.Description>
        <Field.Root style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <Field.Label>
            <Muted text="Name" />
          </Field.Label>
          <Field.Control value={name} onInput={(event) => setName(event.value ?? "")} style={[inputStyle(), { width: 260 }]} />
        </Field.Root>
        <Field.Root style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <Field.Label>
            <Muted text="Organisation" />
          </Field.Label>
          <Field.Control value={org} onInput={(event) => setOrg(event.value ?? "")} style={[inputStyle(), { width: 260 }]} />
        </Field.Root>
      </Fieldset.Root>
      <Note text={"disabled " + String(saving()) + ' · name "' + name() + '" · org "' + org() + '"'} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Form
 *
 * The ui package binds no `Form` compound of its own: a form is a Fieldset of Field roots whose
 * `validationMode` defaults to `"onSubmit"`, plus the submit edge an `Input` reports.
 * -------------------------------------------------------------------------------------------- */

const plans = ["hobby", "pro", "team"];

export function FormDemo(): NativeNode {
  const [email, setEmail] = createSignal("");
  const [plan, setPlan] = createSignal<string | undefined>("pro");
  const [terms, setTerms] = createSignal(false);
  const [submitted, setSubmitted] = createSignal("nothing submitted yet");
  const [attempts, setAttempts] = createSignal(0);
  const [showErrors, setShowErrors] = createSignal(false);

  const emailValid = (): boolean => email().includes("@");
  const formValid = (): boolean => emailValid() && terms();

  const submit = (): void => {
    setAttempts(attempts() + 1);
    if (!formValid()) {
      setShowErrors(true);
      setSubmitted("rejected: fix the fields below");
      return;
    }
    setShowErrors(false);
    setSubmitted("accepted: " + email() + " on the " + (plan() ?? "") + " plan");
  };

  return (
    <Panel
      title="Form"
      hint="validationMode defaults to onSubmit, so nothing is marked invalid until the form is really submitted. Return inside the field submits it too."
    >
      <Fieldset.Root
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 12,
          padding: 14,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
        }}
      >
        <Fieldset.Legend>
          <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>Sign up</Text>
        </Fieldset.Legend>

        <Field.Root
          required
          invalid={() => showErrors() && !emailValid()}
          touched={() => attempts() > 0}
          filled={() => email().length > 0}
          validationMessage={() => "An address is required"}
          style={{ display: "flex", flexDirection: "column", gap: 4 }}
        >
          <Field.Label>
            <Muted text="Email" />
          </Field.Label>
          <Field.Control
            value={email}
            placeholder="you@example.com"
            onInput={(event) => setEmail(event.value ?? "")}
            onSubmit={() => submit()}
            style={[inputStyle(), { width: 280 }]}
          />
          <Show when={() => showErrors() && !emailValid()}>
            <Text style={{ fontSize: 12, color: p().danger }}>An address containing @ is required</Text>
          </Show>
        </Field.Root>

        <Field.Root style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <Field.Label passive>
            <Muted text="Plan" />
          </Field.Label>
          <RadioGroup.Root value={plan} onValueChange={(next) => setPlan(next)} required style={{ display: "flex", flexDirection: "row", gap: 10 }}>
            <For each={() => plans}>
              {(value) => (
                <Radio.Root
                  value={value}
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    alignItems: "center",
                    gap: 6,
                    height: 26,
                    paddingLeft: 8,
                    paddingRight: 10,
                    borderRadius: 8,
                    hover: { backgroundColor: p().controlHover },
                    focus: { outline: "2px solid " + p().accent },
                  }}
                >
                  <Radio.Indicator
                    style={{
                      width: 14,
                      height: 14,
                      borderRadius: 7,
                      borderWidth: plan() === value ? 4 : 1,
                      borderColor: plan() === value ? p().accent : p().border,
                      backgroundColor: p().control,
                    }}
                  />
                  <Label text={value} />
                </Radio.Root>
              )}
            </For>
          </RadioGroup.Root>
        </Field.Root>

        <Checkbox.Root
          checked={(): CheckedState => terms()}
          onCheckedChange={(next) => setTerms(next)}
          style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, height: 26 }}
        >
          <Checkbox.Indicator
            style={{
              width: 16,
              height: 16,
              borderRadius: 5,
              borderWidth: 1,
              borderColor: terms() ? p().accent : p().border,
              backgroundColor: terms() ? p().accent : p().control,
            }}
          />
          <Label text="I accept the terms" />
        </Checkbox.Root>
        <Show when={() => showErrors() && !terms()}>
          <Text style={{ fontSize: 12, color: p().danger }}>The terms must be accepted</Text>
        </Show>

        <Row>
          <Btn label="Submit" primary onClick={() => submit()} />
          <Btn
            label="Reset"
            onClick={() => {
              setEmail("");
              setPlan("pro");
              setTerms(false);
              setShowErrors(false);
              setAttempts(0);
              setSubmitted("nothing submitted yet");
            }}
          />
        </Row>
      </Fieldset.Root>
      <Note text={"attempts " + String(attempts()) + " · " + submitted()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Input
 * -------------------------------------------------------------------------------------------- */

export function InputDemo(): NativeNode {
  const [text, setText] = createSignal("");
  const [secret, setSecret] = createSignal("");
  const [notes, setNotes] = createSignal("Two\nlines");
  const [submits, setSubmits] = createSignal(0);
  return (
    <Panel
      title="Input"
      hint="Controlled native editors. Update value from event.value in onInput; Return on a single-line input reports onSubmit."
    >
      <Col>
        <Muted text="text · Return submits" />
        <Input
          value={text()}
          placeholder="Type and press Return"
          onInput={(event) => setText(event.value ?? "")}
          onSubmit={() => setSubmits(submits() + 1)}
          style={[inputStyle(), { width: 300 }]}
        />
        <Muted text="password" />
        <Input type="password" value={secret()} placeholder="Secret" onInput={(event) => setSecret(event.value ?? "")} style={[inputStyle(), { width: 300 }]} />
        <Muted text="multiline" />
        <Input multiline value={notes()} onInput={(event) => setNotes(event.value ?? "")} style={[inputStyle(), { width: 300, height: 72, paddingTop: 6 }]} />
      </Col>
      <Note
        text={'text "' + text() + '" · password ' + String(secret().length) + " chars · notes " + String(notes().split("\n").length) + " lines · submits " + String(submits())}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Number field
 * -------------------------------------------------------------------------------------------- */

function NumberFieldScrubCursor(): NativeNode {
  const field = useNumberFieldState();
  return <NumberField.ScrubAreaCursor style={{ width: 12, height: 12, borderRadius: 6, backgroundColor: field().scrubbing ? p().accent : p().border }} />;
}

function NumberFieldStateLine(): NativeNode {
  const field = useNumberFieldState();
  return <Note text={"scrubbing " + String(field().scrubbing) + " · readOnly " + String(field().readOnly) + " · required " + String(field().required)} />;
}

export function NumberFieldDemo(): NativeNode {
  const [quantity, setQuantity] = createSignal<number | undefined>(8);
  const [valid, setValid] = createSignal(true);
  const [committed, setCommitted] = createSignal<number | undefined>(undefined);
  return (
    <Panel
      title="Number field"
      hint="The core parses, clamps, snaps, and formats. Alt is the small step, Shift the large one, the wheel scrubs while focused, and Return is the commit boundary."
    >
      <NumberField.Root
        value={quantity}
        min={0}
        max={99}
        step={1}
        smallStep={0.5}
        largeStep={10}
        precision={1}
        snapOnStep
        required
        scrubDirection="horizontal"
        onValueChange={(next, isValid) => {
          setQuantity(next);
          setValid(isValid);
        }}
        onValueCommitted={(next) => setCommitted(next)}
      >
        <NumberField.Group>
          <Row>
            <NumberField.Decrement style={[controlStyle(), { width: 32, paddingLeft: 0, paddingRight: 0 }]}>
              <Text style={{ fontSize: 14, color: p().ink }}>−</Text>
            </NumberField.Decrement>
            <NumberField.Input style={[inputStyle(), { width: 84, textAlign: "center" }]} />
            <NumberField.Increment style={[controlStyle(), { width: 32, paddingLeft: 0, paddingRight: 0 }]}>
              <Text style={{ fontSize: 14, color: p().ink }}>+</Text>
            </NumberField.Increment>
            <NumberField.ScrubArea
              style={{
                width: 44,
                height: 30,
                borderRadius: 8,
                borderColor: p().border,
                borderWidth: 1,
                alignItems: "center",
                justifyContent: "center",
                cursor: "ew-resize",
              }}
            >
              <NumberFieldScrubCursor />
            </NumberField.ScrubArea>
          </Row>
        </NumberField.Group>
        <NumberFieldStateLine />
      </NumberField.Root>
      <Note
        text={"value " + String(quantity() ?? "—") + " · valid " + String(valid()) + " · committed " + (committed() === undefined ? "—" : String(committed()))}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * OTP field
 * -------------------------------------------------------------------------------------------- */

export function OtpFieldDemo(): NativeNode {
  const [code, setCode] = createSignal("");
  const [completed, setCompleted] = createSignal("not yet");
  const slot: Style[] = [inputStyle(), { width: 38, height: 44, paddingLeft: 0, paddingRight: 0, fontSize: 17, textAlign: "center" }];
  return (
    <Panel
      title="OTP field"
      hint="Six one-character slots. Typing advances, a paste distributes, Backspace clears then walks back, and the completion edge is reported once."
    >
      <OtpField.Root
        length={6}
        value={code}
        validationType="numeric"
        required
        onValueChange={(next) => setCode(next)}
        onComplete={(next) => setCompleted(next)}
        style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6 }}
      >
        <OtpField.Input index={0} style={slot} />
        <OtpField.Input index={1} style={slot} />
        <OtpField.Input index={2} style={slot} />
        <OtpField.Separator index={2}>
          <Text style={{ fontSize: 15, color: p().faint }}>–</Text>
        </OtpField.Separator>
        <OtpField.Input index={3} style={slot} />
        <OtpField.Input index={4} style={slot} />
        <OtpField.Input index={5} style={slot} />
      </OtpField.Root>
      <Note text={'code "' + code() + '" · completed ' + completed()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Date field
 * -------------------------------------------------------------------------------------------- */

function segmentStyle(width: number): Style[] {
  return [inputStyle(), { width, textAlign: "center" }];
}

export function DateFieldDemo(): NativeNode {
  const [due, setDue] = createSignal<string | undefined>("2026-09-03");
  const [iso, setIso] = createSignal<string | undefined>("2026-01-15");
  return (
    <Panel
      title="Date field"
      hint="Segmented civil dates. Up/Down step a segment, typing rolls into the next one, and the core clamps into min/max before reporting."
    >
      <Row>
        <Muted text="mdy" />
        <DateField.Root
          value={due}
          format="mdy"
          min="2026-01-01"
          max="2026-12-31"
          onValueChange={(next) => setDue(next)}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <DateField.Segment segment="month" style={segmentStyle(46)} />
          <DateField.Segment segment="day" style={segmentStyle(46)} />
          <DateField.Segment segment="year" style={segmentStyle(68)} />
        </DateField.Root>
      </Row>
      <Row>
        <Muted text="ymd" />
        <DateField.Root value={iso} onValueChange={(next) => setIso(next)} style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <DateField.Segment segment="year" style={segmentStyle(68)} />
          <DateField.Segment segment="month" style={segmentStyle(46)} />
          <DateField.Segment segment="day" style={segmentStyle(46)} />
        </DateField.Root>
      </Row>
      <Note text={"due " + (due() ?? "—") + " · iso " + (iso() ?? "—")} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Time field
 * -------------------------------------------------------------------------------------------- */

export function TimeFieldDemo(): NativeNode {
  const [at, setAt] = createSignal<string | undefined>("09:30");
  const [precise, setPrecise] = createSignal<string | undefined>("14:05:30");
  return (
    <Panel title="Time field" hint="Civil times with no time zone. Without showSeconds the core owns no seconds segment at all.">
      <Row>
        <Muted text="12-hour" />
        <TimeField.Root value={at} hour12 onValueChange={(next) => setAt(next)} style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <TimeField.Segment segment="hour" style={segmentStyle(46)} />
          <TimeField.Segment segment="minute" style={segmentStyle(46)} />
          <TimeField.Segment segment="period" style={segmentStyle(46)} />
        </TimeField.Root>
      </Row>
      <Row>
        <Muted text="24-hour + seconds" />
        <TimeField.Root value={precise} showSeconds onValueChange={(next) => setPrecise(next)} style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <TimeField.Segment segment="hour" style={segmentStyle(46)} />
          <TimeField.Segment segment="minute" style={segmentStyle(46)} />
          <TimeField.Segment segment="second" style={segmentStyle(46)} />
        </TimeField.Root>
      </Row>
      <Note text={"at " + (at() ?? "—") + " · precise " + (precise() ?? "—")} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Calendar
 * -------------------------------------------------------------------------------------------- */

const weekdayNames = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/** Six weeks of ISO dates covering the month, Monday first. */
function monthGrid(month: string): string[][] {
  const parts = month.split("-");
  const year = Number(parts[0] ?? "2026");
  const monthNumber = Number(parts[1] ?? "9");
  const first = new Date(Date.UTC(year, monthNumber - 1, 1));
  const weekday = first.getUTCDay();
  const start = weekday === 0 ? 6 : weekday - 1;
  const weeks: string[][] = [];
  for (let week = 0; week < 6; week += 1) {
    const days: string[] = [];
    for (let column = 0; column < 7; column += 1) {
      const date = new Date(Date.UTC(year, monthNumber - 1, 1 - start + week * 7 + column));
      days.push(date.toISOString().slice(0, 10));
    }
    weeks.push(days);
  }
  return weeks;
}

export function CalendarDemo(): NativeNode {
  const [day, setDay] = createSignal<string | undefined>("2026-09-03");
  const [month, setMonth] = createSignal("2026-09");
  const [focused, setFocused] = createSignal("2026-09-03");
  const grid = (): string[][] => monthGrid(month());
  return (
    <Panel
      title="Calendar"
      hint="ISO civil dates with no time zone. The grid is application-declared; the single Tab stop, arrow navigation, and month changes are the core's."
    >
      <Calendar.Root
        value={day}
        min="2026-01-01"
        max="2026-12-31"
        firstWeekday={0}
        onValueChange={(next) => setDay(next)}
        onMonthChange={(next) => setMonth(next)}
        onFocusChange={(next) => setFocused(next)}
        style={{ display: "flex", flexDirection: "column", gap: 3 }}
      >
        <View style={{ display: "flex", flexDirection: "row", gap: 3 }}>
          <For each={() => weekdayNames}>
            {(name) => (
              <View style={{ width: 32, height: 20, alignItems: "center", justifyContent: "center" }}>
                <Text style={{ fontSize: 10, color: p().faint, fontWeight: 700 }}>{name}</Text>
              </View>
            )}
          </For>
        </View>
        <For each={grid}>
          {(week, index) => (
            <Calendar.Week index={index()} style={{ display: "flex", flexDirection: "row", gap: 3 }}>
              <For each={() => week}>
                {(date) => (
                  <Calendar.Day
                    day={date}
                    style={{
                      width: 32,
                      height: 28,
                      borderRadius: 7,
                      alignItems: "center",
                      justifyContent: "center",
                      backgroundColor: date === day() ? p().accent : p().panelAlt,
                      hover: { backgroundColor: date === day() ? p().accent : p().controlHover },
                      focus: { outline: "2px solid " + p().accent },
                      outlineOffset: 1,
                    }}
                  >
                    <Text style={{ fontSize: 11, color: date === day() ? p().onAccent : date.slice(0, 7) === month() ? p().ink : p().faint }}>
                      {date.slice(8)}
                    </Text>
                  </Calendar.Day>
                )}
              </For>
            </Calendar.Week>
          )}
        </For>
      </Calendar.Root>
      <Note text={"selected " + (day() ?? "—") + " · showing " + month() + " · tab stop " + focused()} />
    </Panel>
  );
}
