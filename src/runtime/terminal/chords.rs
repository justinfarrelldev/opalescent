//! Deterministic companion terminal chord router.
//!
//! The router stores no application command payloads. It authenticates input by
//! the hidden stream identity from [`TerminalCapabilities`] and enforces the
//! hidden delivery ordinal carried by every [`TerminalInputEvent`].

#![cfg_attr(
    not(test),
    expect(
        clippy::missing_docs_in_private_items,
        reason = "Task 32 deterministic router keeps extensive private state matching proposal terms"
    )
)]
#![expect(
    clippy::struct_excessive_bools,
    reason = "Chord modifier records intentionally mirror fixed proposal booleans"
)]
#![expect(
    clippy::use_debug,
    reason = "Structured enum display is diagnostic-only for task-32 runtime errors"
)]
#![expect(
    clippy::missing_const_for_fn,
    reason = "Keeping helper constructors ordinary functions avoids unstable const Vec assumptions"
)]
#![expect(
    clippy::needless_pass_by_value,
    reason = "Router construction consumes authenticated capability snapshots by value at API boundary"
)]
#![expect(
    clippy::unnecessary_wraps,
    reason = "Process helpers keep Result shape aligned with fallible future allocation paths"
)]
#![expect(
    clippy::unused_self,
    reason = "Deadline helper remains a method to preserve router-owned timeout semantics"
)]

extern crate alloc;

use super::model::{
    TerminalCapabilities, TerminalFeatureCapability, TerminalInputEvent, TerminalInputEventKind,
    TerminalKeyOccurrence, TerminalLogicalKey, TerminalModifiers, TerminalNamedKey,
    TerminalOrdinaryFeature,
};
use crate::runtime::timer::{MonotonicDeadline, monotonic_clock_now};
use alloc::vec::Vec;
use core::fmt;

/// Chord trigger policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalChordTrigger {
    /// Match only press occurrences.
    Press,
    /// Match press and repeat occurrences.
    PressOrRepeat,
    /// Match only release occurrences.
    Release,
}

/// Application-constructible chord modifier record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalChordModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// Lock-key participation mask for chord matching.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalLockModifierMask {
    pub caps_lock: bool,
    pub num_lock: bool,
}

/// Application-constructible chord key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalChordKey {
    Control { code: u8 },
    Named { key: TerminalNamedKey },
    Function { number: i32 },
    EnhancedText { text: alloc::string::String },
}

/// One immutable chord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalChord {
    key: TerminalChordKey,
    modifiers: TerminalChordModifiers,
    lock_mask: TerminalLockModifierMask,
    trigger: TerminalChordTrigger,
}

impl TerminalChord {
    #[must_use]
    pub const fn new(
        key: TerminalChordKey,
        modifiers: TerminalChordModifiers,
        trigger: TerminalChordTrigger,
    ) -> Self {
        Self {
            key,
            modifiers,
            lock_mask: TerminalLockModifierMask {
                caps_lock: false,
                num_lock: false,
            },
            trigger,
        }
    }

    #[must_use]
    pub const fn with_lock_modifier_mask(mut self, lock_mask: TerminalLockModifierMask) -> Self {
        self.lock_mask = lock_mask;
        self
    }
}

/// Immutable bounded chord sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalChordSequence {
    chords: Vec<TerminalChord>,
}

impl TerminalChordSequence {
    const MAX_ABSOLUTE_LEN: usize = 256;

    pub fn single(chord: TerminalChord) -> Self {
        Self {
            chords: alloc::vec![chord],
        }
    }

    pub fn append(mut self, chord: TerminalChord) -> Result<Self, TerminalChordValidationError> {
        if self.chords.len() >= Self::MAX_ABSOLUTE_LEN {
            return Err(TerminalChordValidationError::SequenceTooLong { limit: 256 });
        }
        self.chords.push(chord);
        Ok(self)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.chords.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chords.is_empty()
    }
}

/// Prefix conflict policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalChordPrefixPolicy {
    RejectAmbiguousPrefixes,
    HigherPriorityWins,
    LongestThenPriority,
}

/// Linked text policy for activated chord keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalChordTextPolicy {
    PreserveLinkedText,
    SuppressLinkedText,
}

/// Router construction and registration policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalChordRouterPolicy {
    pub maximum_registrations: usize,
    pub maximum_sequence_length: usize,
    pub maximum_buffered_events: usize,
    pub sequence_timeout_milliseconds: u64,
    pub prefix_policy: TerminalChordPrefixPolicy,
}

impl Default for TerminalChordRouterPolicy {
    fn default() -> Self {
        Self {
            maximum_registrations: 64,
            maximum_sequence_length: 16,
            maximum_buffered_events: 128,
            sequence_timeout_milliseconds: 25,
            prefix_policy: TerminalChordPrefixPolicy::RejectAmbiguousPrefixes,
        }
    }
}

/// Opaque immutable binding identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalChordBindingId {
    router_id: u64,
    ordinal: u64,
}

impl TerminalChordBindingId {
    #[must_use]
    pub const fn ordinal(self) -> u64 {
        self.ordinal
    }
}

/// Chord validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalChordValidationError {
    EmptySequence,
    SequenceTooLong { limit: usize },
    RegistrationLimitReached { limit: usize },
    DuplicateBinding { existing: TerminalChordBindingId },
    PrefixAmbiguity,
    EnhancedKeyIdentityRequired,
    ReleaseEventsRequired,
    BindingIdentifierExhausted,
    BufferCapacityBelowCorrelatedLimit { configured: usize, required: usize },
}

impl fmt::Display for TerminalChordValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TerminalChordValidationError {}

/// Mutation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalChordMutationResult {
    Unregistered {
        binding_id: TerminalChordBindingId,
        released_input: TerminalChordReleasedInput,
    },
    Replaced {
        binding_id: TerminalChordBindingId,
        released_input: TerminalChordReleasedInput,
    },
}

/// Mutation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalChordMutationError {
    BindingNotFound,
    WrongRouter,
    CorrelatedGroupPending,
}

impl fmt::Display for TerminalChordMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TerminalChordMutationError {}

/// Process failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalChordProcessError {
    BufferedCapacityExceeded {
        configured_limit: usize,
        released_input: TerminalChordReleasedInput,
    },
    WrongInputStream,
    DeliveryAlreadyConsumed,
    DeliveryOutOfOrder,
}

impl fmt::Display for TerminalChordProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TerminalChordProcessError {}

/// Immutable released input carrier.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalChordReleasedInput {
    events: Vec<TerminalInputEvent>,
}

impl TerminalChordReleasedInput {
    #[must_use]
    pub fn len(&self) -> i64 {
        i64::try_from(self.events.len()).unwrap_or(i64::MAX)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    #[must_use]
    pub fn at(&self, index: i64) -> Option<TerminalInputEvent> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.events.get(index).cloned())
    }

    fn from_events(events: Vec<TerminalInputEvent>) -> Self {
        Self { events }
    }
}

/// Router output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalChordRouterOutput {
    Pending {
        deadline: MonotonicDeadline,
    },
    ReleasedInput {
        input: TerminalChordReleasedInput,
    },
    Activated {
        binding_id: TerminalChordBindingId,
        occurrence: TerminalKeyOccurrence,
        released_input: TerminalChordReleasedInput,
    },
    Idle,
    AwaitingCorrelatedInput,
}

/// Reset reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalChordResetReason {
    ApplicationRequested,
    FocusLost,
    InputReset,
    Cancelled,
    EndOfInput,
    Pause,
}

#[derive(Debug, Clone)]
struct Registration {
    id: TerminalChordBindingId,
    sequence: TerminalChordSequence,
    priority: i32,
    text_policy: TerminalChordTextPolicy,
}

/// Affine chord router state owner.
#[derive(Debug)]
pub struct TerminalChordRouter {
    router_id: u64,
    stream_id: u64,
    correlated_limit: usize,
    enhanced_key_identity_enabled: bool,
    key_release_events_enabled: bool,
    policy: TerminalChordRouterPolicy,
    registrations: Vec<Registration>,
    next_binding_ordinal: u64,
    expected_delivery_ordinal: u64,
    buffered: Vec<TerminalInputEvent>,
    matched: Vec<TerminalChord>,
    pending_deadline: Option<MonotonicDeadline>,
}

impl TerminalChordRouter {
    pub fn new(
        capabilities: TerminalCapabilities,
        policy: TerminalChordRouterPolicy,
    ) -> Result<Self, TerminalChordValidationError> {
        let correlated_limit = usize::try_from(capabilities.hidden_correlated_event_limit().get())
            .unwrap_or(usize::MAX);
        let enhanced_key_identity_enabled = matches!(
            capabilities.feature(TerminalOrdinaryFeature::EnhancedKeyIdentity),
            TerminalFeatureCapability::Enabled { .. }
        );
        let key_release_events_enabled = matches!(
            capabilities.feature(TerminalOrdinaryFeature::KeyReleaseEvents),
            TerminalFeatureCapability::Enabled { .. }
        );
        if policy.maximum_buffered_events < correlated_limit {
            return Err(
                TerminalChordValidationError::BufferCapacityBelowCorrelatedLimit {
                    configured: policy.maximum_buffered_events,
                    required: correlated_limit,
                },
            );
        }
        Ok(Self {
            router_id: capabilities.hidden_stream_id(),
            stream_id: capabilities.hidden_stream_id(),
            correlated_limit,
            enhanced_key_identity_enabled,
            key_release_events_enabled,
            policy,
            registrations: Vec::new(),
            next_binding_ordinal: 1,
            expected_delivery_ordinal: 1,
            buffered: Vec::new(),
            matched: Vec::new(),
            pending_deadline: None,
        })
    }

    pub fn register(
        &mut self,
        sequence: TerminalChordSequence,
        priority: i32,
        text_policy: TerminalChordTextPolicy,
    ) -> Result<TerminalChordBindingId, TerminalChordValidationError> {
        self.validate_registration(&sequence, priority, None)?;
        let ordinal = self.next_binding_ordinal;
        let next = ordinal
            .checked_add(1)
            .ok_or(TerminalChordValidationError::BindingIdentifierExhausted)?;
        let id = TerminalChordBindingId {
            router_id: self.router_id,
            ordinal,
        };
        self.registrations.push(Registration {
            id,
            sequence,
            priority,
            text_policy,
        });
        self.next_binding_ordinal = next;
        Ok(id)
    }

    pub fn unregister(
        &mut self,
        binding_id: TerminalChordBindingId,
    ) -> Result<TerminalChordMutationResult, TerminalChordMutationError> {
        self.validate_binding_router(binding_id)?;
        let Some(position) = self
            .registrations
            .iter()
            .position(|reg| reg.id == binding_id)
        else {
            return Err(TerminalChordMutationError::BindingNotFound);
        };
        self.registrations.remove(position);
        let released_input = self.clear_buffered();
        Ok(TerminalChordMutationResult::Unregistered {
            binding_id,
            released_input,
        })
    }

    pub fn replace(
        &mut self,
        binding_id: TerminalChordBindingId,
        sequence: TerminalChordSequence,
        priority: i32,
        text_policy: TerminalChordTextPolicy,
    ) -> Result<TerminalChordMutationResult, TerminalChordMutationError> {
        self.validate_binding_router(binding_id)?;
        let Some(position) = self
            .registrations
            .iter()
            .position(|reg| reg.id == binding_id)
        else {
            return Err(TerminalChordMutationError::BindingNotFound);
        };
        self.validate_registration(&sequence, priority, Some(binding_id))
            .map_err(|_err| TerminalChordMutationError::BindingNotFound)?;
        self.registrations[position] = Registration {
            id: binding_id,
            sequence,
            priority,
            text_policy,
        };
        let released_input = self.clear_buffered();
        Ok(TerminalChordMutationResult::Replaced {
            binding_id,
            released_input,
        })
    }

    pub fn process(
        &mut self,
        event: TerminalInputEvent,
    ) -> Result<TerminalChordRouterOutput, TerminalChordProcessError> {
        self.validate_event_order(&event)?;
        let reservation = self.required_reservation(&event);
        if self.buffered.len().saturating_add(reservation) > self.policy.maximum_buffered_events {
            let released_input = self.clear_buffered();
            return Err(TerminalChordProcessError::BufferedCapacityExceeded {
                configured_limit: self.policy.maximum_buffered_events,
                released_input,
            });
        }
        self.expected_delivery_ordinal = self.expected_delivery_ordinal.saturating_add(1);
        self.process_accepted(event)
    }

    pub fn expire_sync(&mut self) -> TerminalChordRouterOutput {
        let Some(deadline) = self.pending_deadline else {
            return TerminalChordRouterOutput::Idle;
        };
        if !deadline.is_expired_by(monotonic_clock_now()) {
            return TerminalChordRouterOutput::Pending { deadline };
        }
        self.pending_deadline = None;
        self.matched.clear();
        TerminalChordRouterOutput::ReleasedInput {
            input: self.clear_buffered(),
        }
    }

    pub fn reset(&mut self, _reason: TerminalChordResetReason) -> TerminalChordReleasedInput {
        self.clear_buffered()
    }

    fn process_accepted(
        &mut self,
        event: TerminalInputEvent,
    ) -> Result<TerminalChordRouterOutput, TerminalChordProcessError> {
        let event_kind = event.kind().clone();
        let TerminalInputEventKind::Key {
            key,
            occurrence,
            modifiers,
            ..
        } = event_kind
        else {
            self.buffered.push(event);
            return Ok(TerminalChordRouterOutput::ReleasedInput {
                input: self.clear_buffered(),
            });
        };
        let event_chord = chord_from_event(&key, occurrence, modifiers);
        self.buffered.push(event);
        let Some(event_chord) = event_chord else {
            return Ok(TerminalChordRouterOutput::ReleasedInput {
                input: self.clear_buffered(),
            });
        };
        self.matched.push(event_chord);

        let mut complete_matches = self
            .registrations
            .iter()
            .filter(|reg| reg.sequence.chords == self.matched)
            .collect::<Vec<_>>();
        complete_matches.sort_by_key(|reg| core::cmp::Reverse(reg.priority));
        let has_prefix = self.registrations.iter().any(|reg| {
            starts_with(&reg.sequence.chords, &self.matched)
                && reg.sequence.len() > self.matched.len()
        });

        if let Some(registration) = complete_matches.first().copied() {
            if has_prefix
                && self.policy.prefix_policy == TerminalChordPrefixPolicy::LongestThenPriority
            {
                let deadline = self.next_deadline();
                self.pending_deadline = Some(deadline);
                return Ok(TerminalChordRouterOutput::Pending { deadline });
            }
            let binding_id = registration.id;
            let text_policy = registration.text_policy;
            let released_input = if text_policy == TerminalChordTextPolicy::PreserveLinkedText {
                self.clear_buffered()
            } else {
                self.buffered.clear();
                self.matched.clear();
                self.pending_deadline = None;
                TerminalChordReleasedInput::default()
            };
            return Ok(TerminalChordRouterOutput::Activated {
                binding_id,
                occurrence,
                released_input,
            });
        }
        if has_prefix {
            let deadline = self.next_deadline();
            self.pending_deadline = Some(deadline);
            return Ok(TerminalChordRouterOutput::Pending { deadline });
        }
        Ok(TerminalChordRouterOutput::ReleasedInput {
            input: self.clear_buffered(),
        })
    }

    fn validate_event_order(
        &self,
        event: &TerminalInputEvent,
    ) -> Result<(), TerminalChordProcessError> {
        if event.hidden_stream_id() != self.stream_id {
            return Err(TerminalChordProcessError::WrongInputStream);
        }
        let ordinal = event.hidden_delivery_ordinal();
        if ordinal < self.expected_delivery_ordinal {
            return Err(TerminalChordProcessError::DeliveryAlreadyConsumed);
        }
        if ordinal > self.expected_delivery_ordinal {
            return Err(TerminalChordProcessError::DeliveryOutOfOrder);
        }
        Ok(())
    }

    fn validate_binding_router(
        &self,
        binding_id: TerminalChordBindingId,
    ) -> Result<(), TerminalChordMutationError> {
        if binding_id.router_id != self.router_id {
            return Err(TerminalChordMutationError::WrongRouter);
        }
        Ok(())
    }

    fn validate_registration(
        &self,
        sequence: &TerminalChordSequence,
        priority: i32,
        replacing: Option<TerminalChordBindingId>,
    ) -> Result<(), TerminalChordValidationError> {
        if sequence.is_empty() {
            return Err(TerminalChordValidationError::EmptySequence);
        }
        if sequence.len() > self.policy.maximum_sequence_length {
            return Err(TerminalChordValidationError::SequenceTooLong {
                limit: self.policy.maximum_sequence_length,
            });
        }
        if replacing.is_none() && self.registrations.len() >= self.policy.maximum_registrations {
            return Err(TerminalChordValidationError::RegistrationLimitReached {
                limit: self.policy.maximum_registrations,
            });
        }
        for chord in &sequence.chords {
            if let TerminalChordKey::EnhancedText { .. } = chord.key {
                if !self.enhanced_key_identity_enabled {
                    return Err(TerminalChordValidationError::EnhancedKeyIdentityRequired);
                }
            }
            if chord.trigger == TerminalChordTrigger::Release && !self.key_release_events_enabled {
                return Err(TerminalChordValidationError::ReleaseEventsRequired);
            }
        }
        for reg in &self.registrations {
            if Some(reg.id) == replacing {
                continue;
            }
            if reg.sequence.chords == sequence.chords && reg.priority == priority {
                return Err(TerminalChordValidationError::DuplicateBinding { existing: reg.id });
            }
            let prefix_overlap = starts_with(&reg.sequence.chords, &sequence.chords)
                || starts_with(&sequence.chords, &reg.sequence.chords);
            if prefix_overlap
                && self.policy.prefix_policy == TerminalChordPrefixPolicy::RejectAmbiguousPrefixes
            {
                return Err(TerminalChordValidationError::PrefixAmbiguity);
            }
        }
        Ok(())
    }

    fn required_reservation(&self, event: &TerminalInputEvent) -> usize {
        if matches!(event.kind(), &TerminalInputEventKind::Key { .. }) {
            self.correlated_limit
        } else {
            1
        }
    }

    fn clear_buffered(&mut self) -> TerminalChordReleasedInput {
        let events = core::mem::take(&mut self.buffered);
        self.matched.clear();
        self.pending_deadline = None;
        TerminalChordReleasedInput::from_events(events)
    }

    fn next_deadline(&self) -> MonotonicDeadline {
        monotonic_clock_now()
    }
}

fn starts_with(full: &[TerminalChord], prefix: &[TerminalChord]) -> bool {
    full.len() >= prefix.len() && full.iter().zip(prefix).all(|(left, right)| left == right)
}

fn chord_from_event(
    key: &TerminalLogicalKey,
    occurrence: TerminalKeyOccurrence,
    modifiers: TerminalModifiers,
) -> Option<TerminalChord> {
    let trigger = match occurrence {
        TerminalKeyOccurrence::Press { .. } => TerminalChordTrigger::Press,
        TerminalKeyOccurrence::Repeat { .. } => TerminalChordTrigger::PressOrRepeat,
        TerminalKeyOccurrence::Release => TerminalChordTrigger::Release,
    };
    let chord_key = match *key {
        TerminalLogicalKey::Control { code } => TerminalChordKey::Control { code: code.get() },
        TerminalLogicalKey::Named { key } => TerminalChordKey::Named { key },
        TerminalLogicalKey::Function { number } => TerminalChordKey::Function {
            number: number.get(),
        },
        TerminalLogicalKey::Text { ref logical_text } => TerminalChordKey::EnhancedText {
            text: logical_text.as_str().to_owned(),
        },
    };
    Some(TerminalChord::new(
        chord_key,
        TerminalChordModifiers {
            shift: modifiers.shift,
            control: modifiers.control,
            alt: modifiers.alt,
            super_key: modifiers.super_key,
        },
        trigger,
    ))
}
