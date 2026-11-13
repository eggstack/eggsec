//! The runtime's pending-approval registry.
//!
//! A manual operation whose policy demands a confirmation is **held** at that
//! point: the task does not proceed, and the decision is exposed to clients so
//! an operator can answer it. The runtime owns the hold, because the runtime
//! owns task lifecycle — the daemon owns the process, and `eggsec-policy` owns
//! authorisation semantics.
//!
//! # What the runtime stores, and what it does not
//!
//! The registry stores a [`PolicyPrompt`] — a message, a confirmation-class
//! *label*, and a flag. It does **not** store a `ConfirmationClass`, an
//! `EnforcementDecision`, or any other policy vocabulary: the runtime crate
//! stays policy-neutral and never depends on `eggsec-policy`.
//!
//! The gate returns an [`ApprovalAnswer`] — "approved" or "denied", with an
//! optional reason. It does not interpret what an approval permits. Only the
//! policy kernel decides that, and it does so on the caller's side of the
//! gate, by re-running the authorisation path with an override bounded by the
//! classes the kernel itself demanded.
//!
//! # Ordering
//!
//! A pending decision is registered **before** the prompt is announced. A
//! client that answers the instant it sees the prompt therefore never races
//! the registration.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::error::RuntimeError;
use crate::event::PolicyPrompt;
use crate::ids::{SessionId, TaskId};
use crate::runtime_sink::RuntimeEventSink;

/// The operator's answer to a pending policy decision.
///
/// The answer names *yes* or *no*. It never names the authorisation: the
/// classes that are permitted are computed by the policy kernel, not by the
/// client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalAnswer {
    /// The operator approved the pending decision.
    Approved { reason: Option<String> },
    /// The operator declined the pending decision.
    Denied { reason: Option<String> },
}

impl ApprovalAnswer {
    /// Builds an answer from the wire-shaped pair of flag and reason.
    pub fn from_parts(approved: bool, reason: Option<String>) -> Self {
        if approved {
            Self::Approved { reason }
        } else {
            Self::Denied { reason }
        }
    }

    /// Whether the operator approved.
    pub fn is_approved(&self) -> bool {
        matches!(self, Self::Approved { .. })
    }
}

/// A decision awaiting an answer.
#[derive(Debug)]
struct PendingApproval {
    /// Identifies this registration, so a dropped waiter removes only its own
    /// entry and never a later one registered for the same task.
    id: u64,
    prompt: PolicyPrompt,
    answer_tx: oneshot::Sender<ApprovalAnswer>,
}

/// The runtime's registry of pending policy decisions.
#[derive(Debug, Default)]
pub struct ApprovalRegistry {
    pending: Mutex<RegistryInner>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    entries: HashMap<(SessionId, TaskId), PendingApproval>,
    next_id: u64,
}

impl ApprovalRegistry {
    /// Registers a pending decision and returns its identifier.
    fn register(
        &self,
        key: (SessionId, TaskId),
        prompt: PolicyPrompt,
        answer_tx: oneshot::Sender<ApprovalAnswer>,
    ) -> u64 {
        let mut inner = self.lock();
        let id = inner.next_id;
        inner.next_id = inner.next_id.wrapping_add(1);
        inner.entries.insert(
            key,
            PendingApproval {
                id,
                prompt,
                answer_tx,
            },
        );
        id
    }

    /// Removes the entry for `key`, but only if it is the registration `id`.
    ///
    /// A waiter that is dropped (because its task was cancelled) removes its
    /// own entry here. The identifier check keeps a late drop from removing a
    /// decision that was registered afterwards.
    fn remove(&self, key: (SessionId, TaskId), id: u64) -> Option<PendingApproval> {
        let mut inner = self.lock();
        match inner.entries.get(&key) {
            Some(entry) if entry.id == id => inner.entries.remove(&key),
            _ => None,
        }
    }

    /// Removes and returns the entry for `key`, whatever its identifier.
    fn take(&self, key: (SessionId, TaskId)) -> Option<PendingApproval> {
        let mut inner = self.lock();
        inner.entries.remove(&key)
    }

    /// The prompt registered for `key`, if any.
    fn prompt_for(&self, key: (SessionId, TaskId)) -> Option<PolicyPrompt> {
        let inner = self.lock();
        inner.entries.get(&key).map(|entry| entry.prompt.clone())
    }

    /// The number of pending decisions.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.lock().entries.len()
    }

    /// Locks the registry, recovering from a poisoned mutex.
    ///
    /// A panic in a holder must not make the registry permanently unusable: a
    /// pending decision is a coordination record, not authority.
    fn lock(&self) -> MutexGuard<'_, RegistryInner> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Removes a registration when the waiter that owns it goes away.
struct RegistrationGuard {
    registry: Arc<ApprovalRegistry>,
    key: (SessionId, TaskId),
    id: u64,
}

impl Drop for RegistrationGuard {
    fn drop(&mut self) {
        if self.registry.remove(self.key, self.id).is_some() {
            tracing::debug!(
                session_id = %self.key.0,
                task_id = %self.key.1,
                "pending policy decision dropped with its waiter"
            );
        }
    }
}

/// A handle to the runtime's approval registry.
///
/// The handle is cheap to clone and is carried in the execution context, so an
/// executor can hold an operation at its confirmation point without knowing
/// anything about the daemon, the protocol, or the policy crate.
#[derive(Debug, Clone, Default)]
pub struct ApprovalGate {
    registry: Arc<ApprovalRegistry>,
}

impl ApprovalGate {
    /// Creates a gate over a fresh registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a pending decision, announces it, and awaits the answer.
    ///
    /// The decision is registered **before** the prompt is announced, so a
    /// client that answers immediately never finds nothing pending.
    ///
    /// The wait is bounded by the caller's cancellation token: a cancelled
    /// task drops its pending decision rather than leaving it behind.
    pub async fn request(
        &self,
        sink: &RuntimeEventSink,
        task_id: TaskId,
        prompt: PolicyPrompt,
        cancel: CancellationToken,
    ) -> Result<ApprovalAnswer, RuntimeError> {
        let session_id = sink.session_id();
        let key = (session_id, task_id);
        let (answer_tx, answer_rx) = oneshot::channel();
        let id = self.registry.register(key, prompt.clone(), answer_tx);

        // The guard removes the registration if this future is dropped before
        // the answer arrives — which is how a cancelled task stops holding a
        // decision.
        let _guard = RegistrationGuard {
            registry: Arc::clone(&self.registry),
            key,
            id,
        };

        sink.policy_decision_required(prompt);

        if cancel.is_cancelled() {
            return Err(RuntimeError::DispatchFailed(
                "task cancelled before its policy decision was answered".into(),
            ));
        }

        tokio::select! {
            answer = answer_rx => answer.map_err(|_| {
                RuntimeError::DispatchFailed(
                    "the pending policy decision was dropped before it was answered".into(),
                )
            }),
            _ = cancel.cancelled() => Err(RuntimeError::DispatchFailed(
                "task cancelled while awaiting a policy decision".into(),
            )),
        }
    }

    /// Answers a pending decision.
    ///
    /// Returns `Err` when nothing is pending for that task: a task that never
    /// asked, a decision that was already answered, or a decision whose holder
    /// was cancelled. A strict surface never registers a decision, so an
    /// answer for one lands here too.
    pub fn resolve(
        &self,
        session_id: SessionId,
        task_id: TaskId,
        answer: ApprovalAnswer,
    ) -> Result<(), RuntimeError> {
        let key = (session_id, task_id);
        let pending = self
            .registry
            .take(key)
            .ok_or_else(|| RuntimeError::NoPendingApproval(task_id.to_string()))?;
        if pending.answer_tx.send(answer).is_err() {
            // The holder went away between the removal and the send. The
            // decision no longer matters, so this is not an error for the
            // answerer — but it is worth recording.
            tracing::debug!(
                session_id = %session_id,
                task_id = %task_id,
                "policy decision answered after its holder was gone"
            );
        }
        Ok(())
    }

    /// The prompt registered for a task, if one is pending.
    pub fn pending(&self, session_id: SessionId, task_id: TaskId) -> Option<PolicyPrompt> {
        self.registry.prompt_for((session_id, task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RuntimeEvent;
    use tokio::sync::broadcast;

    /// A sink for a task, and the receiver that observes its events.
    ///
    /// The sink is built for the same task the request names, exactly as the
    /// runtime builds it — the announced prompt names the sink's own task.
    fn sink_and_receiver(
        task_id: TaskId,
        session_id: SessionId,
    ) -> (RuntimeEventSink, broadcast::Receiver<RuntimeEvent>) {
        let (tx, rx) = broadcast::channel(16);
        let sink = RuntimeEventSink::new(task_id, session_id, tx);
        (sink, rx)
    }

    fn prompt() -> PolicyPrompt {
        PolicyPrompt {
            message: "the target is outside the configured scope".into(),
            confirmation_class: Some("out-of-scope".into()),
            requires_explicit_approval: true,
        }
    }

    #[tokio::test]
    async fn a_registered_prompt_is_observable_and_the_answer_reaches_the_waiter() {
        let gate = ApprovalGate::new();
        let session_id = SessionId::new();
        let task_id = TaskId::new();
        let (sink, mut rx) = sink_and_receiver(task_id, session_id);
        let cancel = CancellationToken::new();

        let waiter = {
            let gate = gate.clone();
            let sink = sink.clone();
            let cancel = cancel.clone();
            tokio::spawn(async move { gate.request(&sink, task_id, prompt(), cancel).await })
        };

        // The prompt is announced, and it is announced after registration.
        let announced = rx.recv().await.expect("the prompt must be announced");
        match announced {
            RuntimeEvent::PolicyDecisionRequired {
                session_id: announced_session,
                task_id: announced_task,
                prompt: announced_prompt,
            } => {
                assert_eq!(announced_session, session_id);
                assert_eq!(announced_task, Some(task_id));
                assert_eq!(announced_prompt.message, prompt().message);
            }
            other => panic!("expected a policy decision event, got {other:?}"),
        }

        assert_eq!(
            gate.pending(session_id, task_id).map(|p| p.message),
            Some(prompt().message)
        );

        gate.resolve(
            session_id,
            task_id,
            ApprovalAnswer::Approved {
                reason: Some("confirmed by the operator".into()),
            },
        )
        .expect("the pending decision must resolve");

        let answer = waiter.await.expect("the waiter must not panic");
        assert_eq!(
            answer.expect("the answer must be delivered"),
            ApprovalAnswer::Approved {
                reason: Some("confirmed by the operator".into())
            }
        );
        assert!(gate.pending(session_id, task_id).is_none());
    }

    #[tokio::test]
    async fn a_second_answer_for_the_same_task_is_refused() {
        let gate = ApprovalGate::new();
        let session_id = SessionId::new();
        let task_id = TaskId::new();
        let (sink, mut rx) = sink_and_receiver(task_id, session_id);

        let waiter = {
            let gate = gate.clone();
            let sink = sink.clone();
            tokio::spawn(async move {
                gate.request(&sink, task_id, prompt(), CancellationToken::new())
                    .await
            })
        };
        let _ = rx.recv().await;

        gate.resolve(session_id, task_id, ApprovalAnswer::Denied { reason: None })
            .expect("the first answer resolves");
        let second = gate.resolve(
            session_id,
            task_id,
            ApprovalAnswer::Approved { reason: None },
        );
        assert!(
            matches!(second, Err(RuntimeError::NoPendingApproval(_))),
            "a second answer must be refused, got {second:?}"
        );

        let answer = waiter.await.expect("the waiter must not panic");
        assert_eq!(
            answer.expect("the answer must be delivered"),
            ApprovalAnswer::Denied { reason: None }
        );
    }

    #[tokio::test]
    async fn answering_a_task_that_never_asked_is_refused() {
        let gate = ApprovalGate::new();
        let result = gate.resolve(
            SessionId::new(),
            TaskId::new(),
            ApprovalAnswer::Approved { reason: None },
        );
        assert!(matches!(result, Err(RuntimeError::NoPendingApproval(_))));
    }

    #[tokio::test]
    async fn a_cancelled_waiter_drops_its_pending_decision() {
        let gate = ApprovalGate::new();
        let session_id = SessionId::new();
        let task_id = TaskId::new();
        let (sink, mut rx) = sink_and_receiver(task_id, session_id);
        let cancel = CancellationToken::new();

        let waiter = {
            let gate = gate.clone();
            let sink = sink.clone();
            let cancel = cancel.clone();
            tokio::spawn(async move { gate.request(&sink, task_id, prompt(), cancel).await })
        };
        let _ = rx.recv().await;
        assert!(gate.pending(session_id, task_id).is_some());

        cancel.cancel();
        let outcome = waiter.await.expect("the waiter must not panic");
        assert!(matches!(outcome, Err(RuntimeError::DispatchFailed(_))));

        // The decision does not outlive its holder.
        assert!(gate.pending(session_id, task_id).is_none());
        assert_eq!(gate.registry.len(), 0);
    }

    #[tokio::test]
    async fn a_dropped_waiter_drops_its_pending_decision() {
        let gate = ApprovalGate::new();
        let session_id = SessionId::new();
        let task_id = TaskId::new();
        let (sink, mut rx) = sink_and_receiver(task_id, session_id);

        {
            let holder = gate.clone();
            let sink = sink.clone();
            let waiter = tokio::spawn(async move {
                holder
                    .request(&sink, task_id, prompt(), CancellationToken::new())
                    .await
            });
            let _ = rx.recv().await;
            assert!(gate.pending(session_id, task_id).is_some());
            // Abort drops the future, which drops the registration.
            waiter.abort();
            let _ = waiter.await;
        }

        // Give the aborted task a chance to be dropped.
        tokio::task::yield_now().await;
        assert!(gate.pending(session_id, task_id).is_none());
        assert_eq!(gate.registry.len(), 0);
    }

    #[test]
    fn an_answer_reports_whether_it_was_approved() {
        assert!(ApprovalAnswer::from_parts(true, None).is_approved());
        assert!(!ApprovalAnswer::from_parts(false, None).is_approved());
        assert_eq!(
            ApprovalAnswer::from_parts(false, Some("no".into())),
            ApprovalAnswer::Denied {
                reason: Some("no".into())
            }
        );
    }
}
