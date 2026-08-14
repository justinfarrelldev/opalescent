# Terminal Chords and Key Bindings

## Decision

Chords consume only `TerminalInputEvent.Key`. They never consume `TextInput`, composition, paste, raw bytes, or native input. `Key` is command identity. `TextInput` is the only insertion event, including Direct, Key-linked, and Composition-linked text. The terminal declarations remain proposal-only until constrained types, narrow refinement, structured errors, and `using` fixtures pass.

`TerminalLogicalKey.Text` is valid only when an enhanced backend independently reports layout-resolved command identity. Ordinary indistinguishable text is `TextInput` without a Key. There is no physical-key contract in v1.

## Required refinement form

```opal
if event is TerminalInputEvent.Key into key_event:
    try_chord(key_event)
    let logical_key = key_event.key
    if logical_key is TerminalLogicalKey.Named into named:
        try_named_chord(named.key)
```

This is the only required refinement form. It requires `Type.Variant`; direct identifier scrutinees narrow initially, bindings are immutable and branch-local, compound conditions do not narrow, and `is not` exposes no payload. Future variants may be ignored. No match syntax, exhaustive branches, or deferred cleanup syntax is proposed.

## Common constants and core types

`TerminalControlCode` is a constrained nominal `uint8`, 0 through 31 or 127. Useful constants are NUL 0, Tab 9, Enter 13, Escape 27, Ctrl-Q 17, Ctrl-S 19, and Delete 127. `TerminalFunctionKeyNumber` is constrained to 1 through 32767. `TerminalKeyRepeatCount` is 1 through 2147483647. These are not interchangeable with primitives or each other.

```opal
##
  Description: Chooses which key occurrence activates a chord.
  Location: terminal_chords.types.op.
##
public type TerminalChordTrigger:
    Press
    PressOrRepeat
    Release

##
  Description: Identifies a command gesture without treating committed text as input.
  Location: terminal_chords.types.op.
##
public type TerminalChordKey:
    Control:
        code: TerminalControlCode
    Named:
        key: TerminalNamedKey
    Function:
        number: TerminalFunctionKeyNumber
    EnhancedText:
        logical_text: TerminalCommittedText

##
  Description: Combines a key identity, modifiers, and trigger into one chord.
  Location: terminal_chords.types.op.
##
public type TerminalChord:
    key: TerminalChordKey
    modifiers: TerminalModifiers
    trigger: TerminalChordTrigger
```

`EnhancedText` registration requires negotiated `enhanced_key_identity` to be Enabled; application validation rejects it otherwise. It may only be registered when the application has opted into a backend that provides the matching enhanced logical identity. It never makes ordinary `TextInput` chord input. Chords have no paste, composition, raw-byte, committed-text insertion, physical-key, repeat-sentinel, or absence-sentinel field.

## Arbitrary chords, sequences, and evaluation

A chord accepts any constrained control code, named key, function number, or permitted enhanced text identity, plus the full modifiers structure and trigger. For example, Ctrl-Q exits, Shift-Tab moves backward, Alt-F4 closes a pane, and an enhanced layout identity can bind a command independently of text insertion.

```opal
let chord_accepts_occurrence = f(trigger: TerminalChordTrigger, occurrence: TerminalKeyOccurrence): boolean =>
    if trigger is TerminalChordTrigger.Press:
        if occurrence is TerminalKeyOccurrence.Press:
            return true
    if trigger is TerminalChordTrigger.PressOrRepeat:
        if occurrence is TerminalKeyOccurrence.Press:
            return true
        if occurrence is TerminalKeyOccurrence.Repeat:
            return true
    if trigger is TerminalChordTrigger.Release:
        if occurrence is TerminalKeyOccurrence.Release:
            return true
    return false

let chord_accepts_key = f(chord: TerminalChord, event: TerminalInputEvent): boolean =>
    if event is TerminalInputEvent.Key into key_event:
        let trigger = chord.trigger
        let occurrence = key_event.occurrence
        if chord_accepts_occurrence(trigger, occurrence):
            return chord_key_equals_event_key(chord.key, key_event.key) and chord.modifiers is key_event.modifiers
    return false
```

Sequences are ordered lists of `TerminalChord` values. Prefixes may overlap. A trie or equivalent state machine tracks all still-valid prefixes and resolves a completed binding by documented application priority. It clears on `InputReset`, `Cancelled`, `EndOfInput`, focus loss, pause boundary, or a constrained `TerminalInputSequenceTimeoutMilliseconds` expiry of 1 through 60000 milliseconds. No zero or negative timeout sentinel exists.

## Validation, errors, and language safety

Registration rejects empty sequences, duplicate bindings at the same priority, unresolved prefix ambiguity when the application did not choose a policy, a trigger unsupported by the negotiated capability, an enhanced-text binding without `enhanced_key_identity` Enabled, and an invalid constrained value. These are structured validation errors with bounded diagnostics, not string matching.

Language and keyboard layout safety come first. Bind command gestures through control, named, or function keys when possible. Do not infer commands from committed text. A layout may produce the same text through different keys, dead keys, compose sequences, IMEs, or paste. Composition updates are editor state, a composition commit is insertion, and paste is inserted or reviewed under paste policy. Command isolation applies only to `Paste` with trusted paste provenance, where the backend provides trusted framing or delimiter sanitization. An in-band bracketed-paste delimiter alone is not a security boundary. Without trusted provenance the runtime emits no `Paste` and claims no isolation. Malformed trusted paste remains quarantined as bounded `UnknownBytes` and cannot feed the chord state machine. Untrusted input is never rendered as terminal control output.

## Evolution

`TerminalInputEvent` is public and non-exhaustive with permanent inline stable variant IDs, ABI history, payload-layout hashes, representation version, and runtime-owned unknown-payload drop metadata. Generated metadata retains active and retired IDs, and retired IDs cannot be reused. Minor versions add variants only when boxed runtime-described payloads can retain, drop, and forward them. Changing or removing a variant or payload is major and ABI breaking. Chord code tests only known Key variants using `if ... is ... into`, so an unknown future event remains safe.
