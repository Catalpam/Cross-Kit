//! Runtime entry point for Rust SDKs that integrate with Cross-Kit.
//!
//! Rust SDK crates should depend on this crate instead of depending on
//! Cross-Kit internal crates directly.

#![warn(missing_docs)]

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;

pub use cross_kit_core as core;
pub use cross_kit_core::{
    ArgMetadata, FactoryMetadata, MetadataValidationError, MethodMetadata, ObserverMetadata,
    VM_METADATA_SCHEMA_VERSION, VmMetadata, VmMode,
};
/// Marks a UniFFI-exported Rust VM `impl` for Cross-Kit platform bridge generation.
///
/// The macro emits versioned VM metadata consumed by the iOS and Android
/// packagers. For a state VM, the shortest supported form is
/// `#[cross_kit::vm_bridge(mode = "state")]`; the macro infers the VM type,
/// state type, observer type, `get_state`, `subscribe`, and `unsubscribe`
/// contract from public methods on the annotated impl.
///
/// Diff-list VMs use `mode = "diff_list"` plus `diff = DiffType` and
/// `item = ItemType`. Child VMs created from a root VM can declare
/// `factory = RootViewModel::make_child_vm`, which lets generated root
/// containers own the root and child bridge lifecycle.
///
/// Internal bridge methods such as `new`, `subscribe`, `unsubscribe`, and
/// `get_state` are used by generated code and are not intended to be called by
/// SwiftUI or Compose business code directly.
pub use cross_kit_macros::ck_vm_bridge as vm_bridge;

/// Stable subscription identifier used by generated platform bridges.
pub type SubscriptionId = i64;

/// Thread-safe observer collection for state and diff notifications.
///
/// `notify` snapshots observers before invoking callbacks, so callbacks can
/// subscribe or unsubscribe without re-entering this set's lock.
pub struct ObserverSet<O: ?Sized> {
    inner: Arc<Mutex<ObserverSetInner<O>>>,
}

struct ObserverSetInner<O: ?Sized> {
    observers: HashMap<SubscriptionId, Arc<O>>,
    next_id: SubscriptionId,
}

impl<O: ?Sized> Clone for ObserverSet<O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<O: ?Sized> Default for ObserverSet<O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O: ?Sized> ObserverSet<O> {
    /// Creates an empty observer set.
    ///
    /// Subscription ids start at `1` for each set. Creating a set does not
    /// establish any platform subscription; generated or hand-written VM code
    /// calls [`subscribe`](Self::subscribe) when a platform bridge is attached.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ObserverSetInner {
                observers: HashMap::new(),
                next_id: 1,
            })),
        }
    }

    /// Registers an observer and returns the id used to remove it later.
    ///
    /// This method only stores the observer. It does not replay current state
    /// or initial list diffs; replay belongs to the VM or store layer because
    /// only that layer knows the current snapshot and observer callback method.
    pub fn subscribe(&self, observer: Arc<O>) -> SubscriptionId {
        let mut inner = self.inner.lock().expect("observer set lock poisoned");
        let id = inner.next_id;
        inner.next_id += 1;
        inner.observers.insert(id, observer);
        id
    }

    /// Removes a previously registered observer.
    ///
    /// Returns `true` when the id existed. Calling this more than once with the
    /// same id is allowed and returns `false` after the first removal, which
    /// lets generated platform bridges implement idempotent `close()` methods.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let mut inner = self.inner.lock().expect("observer set lock poisoned");
        inner.observers.remove(&id).is_some()
    }

    /// Notifies all observers using a snapshot of the current subscription set.
    ///
    /// The internal lock is not held while `f` runs. Callback code may safely
    /// subscribe or unsubscribe observers without deadlocking the observer set.
    pub fn notify(&self, mut f: impl FnMut(&Arc<O>)) {
        let observers = self.snapshot();
        Self::notify_snapshot(&observers, |observer| f(observer));
    }

    /// Returns the currently subscribed observers as a detached snapshot.
    ///
    /// Use this when notification needs data prepared outside the observer
    /// lock. Mutations to the set after this call do not affect the returned
    /// snapshot.
    pub fn snapshot(&self) -> Vec<Arc<O>> {
        let observers = {
            let inner = self.inner.lock().expect("observer set lock poisoned");
            inner.observers.values().cloned().collect::<Vec<_>>()
        };
        observers
    }

    /// Invokes a callback for every observer in a previously captured snapshot.
    ///
    /// This helper keeps notification code consistent across state and
    /// diff-list VMs while making the lock-free notification pattern explicit.
    pub fn notify_snapshot(observers: &[Arc<O>], mut f: impl FnMut(&Arc<O>)) {
        for observer in observers {
            f(observer);
        }
    }

    /// Returns whether the set currently has no observers.
    ///
    /// This is mainly useful in tests and diagnostics; VM notification code can
    /// call [`notify`](Self::notify) directly even when the set is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of currently registered observers.
    ///
    /// The value is a moment-in-time count and can change immediately after the
    /// method returns if another thread subscribes or unsubscribes.
    pub fn len(&self) -> usize {
        let inner = self.inner.lock().expect("observer set lock poisoned");
        inner.observers.len()
    }
}

/// Shared state and observer runtime for simple state-driven VMs.
///
/// `StateStore` is intended for Rust SDK code that follows Cross-Kit's state
/// VM pattern: store Rust-owned state, expose a `get_state` method for
/// generated bridges, replay state on subscription, and notify observers after
/// mutations. It keeps the state lock and observer collection in one reusable
/// helper while still letting each VM choose the observer callback method.
pub struct StateStore<S, O: ?Sized> {
    inner: Arc<Mutex<StateStoreInner<S>>>,
    observers: ObserverSet<O>,
    observer_sequence: Arc<Mutex<()>>,
    callback_gate: CallbackGate,
}

struct StateStoreInner<S> {
    state: S,
    event_version: u64,
    notification_version: u64,
    pending_replays: usize,
    events: Vec<StateStoreEvent<S>>,
}

#[derive(Clone)]
struct StateStoreEvent<S> {
    version: u64,
    state: S,
}

struct StateReplayWindow<S> {
    inner: Arc<Mutex<StateStoreInner<S>>>,
    active: bool,
}

#[derive(Clone)]
struct CallbackGate {
    inner: Arc<CallbackGateInner>,
}

struct CallbackGateInner {
    state: Mutex<CallbackGateState>,
    available: Condvar,
}

struct CallbackGateState {
    owner: Option<ThreadId>,
    depth: usize,
}

struct CallbackGateGuard {
    gate: CallbackGate,
}

impl<S> StateReplayWindow<S> {
    fn begin(inner: Arc<Mutex<StateStoreInner<S>>>, locked_inner: &mut StateStoreInner<S>) -> Self {
        locked_inner.pending_replays += 1;
        Self {
            inner,
            active: true,
        }
    }

    fn end_with_locked_inner(&mut self, locked_inner: &mut StateStoreInner<S>) {
        if self.active {
            locked_inner.pending_replays -= 1;
            if locked_inner.pending_replays == 0 {
                locked_inner.events.clear();
            }
            self.active = false;
        }
    }
}

impl<S> Drop for StateReplayWindow<S> {
    fn drop(&mut self) {
        if self.active {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            inner.pending_replays -= 1;
            if inner.pending_replays == 0 {
                inner.events.clear();
            }
        }
    }
}

impl CallbackGate {
    fn new() -> Self {
        Self {
            inner: Arc::new(CallbackGateInner {
                state: Mutex::new(CallbackGateState {
                    owner: None,
                    depth: 0,
                }),
                available: Condvar::new(),
            }),
        }
    }

    fn enter(&self) -> CallbackGateGuard {
        let current = std::thread::current().id();
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while state.owner.as_ref().is_some_and(|owner| owner != &current) {
            state = self
                .inner
                .available
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        state.owner = Some(current);
        state.depth += 1;
        CallbackGateGuard { gate: self.clone() }
    }
}

impl Drop for CallbackGateGuard {
    fn drop(&mut self) {
        let mut state = self
            .gate
            .inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.depth -= 1;
        if state.depth == 0 {
            state.owner = None;
            self.gate.inner.available.notify_all();
        }
    }
}

impl<S, O: ?Sized> Clone for StateStore<S, O> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            observers: self.observers.clone(),
            observer_sequence: self.observer_sequence.clone(),
            callback_gate: self.callback_gate.clone(),
        }
    }
}

impl<S, O: ?Sized> StateStore<S, O> {
    /// Creates a store from an initial state value.
    ///
    /// The store starts with no observers. The first subscription id returned
    /// by [`subscribe_replay`](Self::subscribe_replay) is `1`.
    pub fn new(state: S) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StateStoreInner {
                state,
                event_version: 0,
                notification_version: 0,
                pending_replays: 0,
                events: Vec::new(),
            })),
            observers: ObserverSet::new(),
            observer_sequence: Arc::new(Mutex::new(())),
            callback_gate: CallbackGate::new(),
        }
    }

    /// Reads the current state through a caller-provided projection.
    ///
    /// The state lock is held only while `read` runs. Use this when the VM
    /// stores private state and needs to derive a public state record without
    /// cloning the private store first.
    pub fn read_with<R>(&self, read: impl FnOnce(&S) -> R) -> R {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        read(&inner.state)
    }

    /// Registers an observer, immediately replays the current state, and
    /// returns the subscription id.
    ///
    /// `replay` is called before the observer becomes part of the steady-state
    /// notification set. The state lock is not held while platform callback
    /// code runs, so replay callbacks may call back into the VM without
    /// deadlocking the state store.
    ///
    /// Subscription replay is serialized with [`update_notify`](Self::update_notify)
    /// and [`update_with_notify`](Self::update_with_notify): if state changes
    /// during replay, the subscriber receives those snapshots before the
    /// subscription id is returned.
    pub fn subscribe_replay(
        &self,
        observer: Arc<O>,
        replay: impl FnMut(&Arc<O>, S),
    ) -> SubscriptionId
    where
        S: Clone,
    {
        self.subscribe_replay_with(observer, Clone::clone, replay)
    }

    /// Registers an observer and replays a projected state snapshot.
    ///
    /// Use this variant when the VM stores private state but exposes a derived
    /// platform state record. The snapshot projection runs while the state lock
    /// is held; the replay callback runs after the lock is released.
    ///
    /// The replay window records concurrent state notifications and drains them
    /// before registering the observer. This avoids the common lost-update race
    /// where a subscriber replays an old snapshot and misses the next notify.
    pub fn subscribe_replay_with<R>(
        &self,
        observer: Arc<O>,
        mut snapshot: impl FnMut(&S) -> R,
        mut replay: impl FnMut(&Arc<O>, R),
    ) -> SubscriptionId
    where
        S: Clone,
    {
        let (state, mut replay_after, mut replay_window) = {
            let _observer_sequence = self
                .observer_sequence
                .lock()
                .expect("state store observer sequence lock poisoned");
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let state = snapshot(&inner.state);
            let replay_after = inner.event_version;
            let replay_window = StateReplayWindow::begin(self.inner.clone(), &mut inner);
            (state, replay_after, replay_window)
        };
        self.replay_observer(&observer, state, &mut replay);

        loop {
            let (end_version, replay_states, id) = {
                let _observer_sequence = self
                    .observer_sequence
                    .lock()
                    .expect("state store observer sequence lock poisoned");
                let mut inner = self
                    .inner
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let replay_states = inner
                    .events
                    .iter()
                    .filter(|event| event.version > replay_after)
                    .map(|event| snapshot(&event.state))
                    .collect::<Vec<_>>();
                let end_version = inner.event_version;
                if replay_states.is_empty() {
                    replay_window.end_with_locked_inner(&mut inner);
                    let id = self.observers.subscribe(observer.clone());
                    (end_version, replay_states, Some(id))
                } else {
                    (end_version, replay_states, None)
                }
            };

            if let Some(id) = id {
                return id;
            }
            for state in replay_states {
                self.replay_observer(&observer, state, &mut replay);
            }
            replay_after = end_version;
        }
    }

    /// Removes a previously registered observer.
    ///
    /// Returns `true` if an observer was removed. Repeated calls with the same
    /// id are allowed and return `false` after the first removal.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.observers.unsubscribe(id)
    }

    fn replay_observer<R>(&self, observer: &Arc<O>, state: R, replay: &mut impl FnMut(&Arc<O>, R)) {
        let _callback = self.callback_gate.enter();
        replay(observer, state);
    }
}

impl<S: Clone, O: ?Sized> StateStore<S, O> {
    fn record_event(inner: &mut StateStoreInner<S>, state: S) {
        inner.event_version += 1;
        if inner.pending_replays > 0 {
            inner.events.push(StateStoreEvent {
                version: inner.event_version,
                state,
            });
        }
    }

    fn record_notification_event(inner: &mut StateStoreInner<S>, state: S) {
        Self::record_event(inner, state);
        inner.notification_version += 1;
    }

    /// Returns a clone of the current state.
    pub fn read(&self) -> S {
        self.read_with(Clone::clone)
    }

    /// Mutates state and returns the cloned post-mutation state.
    ///
    /// The observer set is not notified automatically, but active subscription
    /// replay windows still record the new state so new observers do not get
    /// stuck on an older snapshot. Use [`update_notify`](Self::update_notify)
    /// when a mutation should emit a state callback to already subscribed
    /// observers.
    pub fn update(&self, mutate: impl FnOnce(&mut S)) -> S {
        self.update_with(|state| {
            mutate(state);
            state.clone()
        })
    }

    /// Mutates the state and returns a caller-provided result.
    ///
    /// This is the lower-level mutation primitive for VMs that need to update
    /// state without notifying current observers. Active subscription replay
    /// windows still see the cloned post-mutation state.
    pub fn update_with<R>(&self, mutate: impl FnOnce(&mut S) -> R) -> R {
        let _observer_sequence = self
            .observer_sequence
            .lock()
            .expect("state store observer sequence lock poisoned");
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let value = mutate(&mut inner.state);
        let state = inner.state.clone();
        Self::record_event(&mut inner, state);
        value
    }

    /// Mutates state, notifies observers with the post-mutation snapshot, and
    /// returns that snapshot.
    ///
    /// The state lock is released before callbacks run. Each observer receives
    /// a clone of the same state value. If a callback re-enters the VM and
    /// publishes a newer state, the older notification round stops before
    /// sending stale state to the remaining observers.
    pub fn update_notify(&self, mutate: impl FnOnce(&mut S), notify: impl FnMut(&Arc<O>, S)) -> S {
        self.update_with_notify(
            |state| {
                mutate(state);
                state.clone()
            },
            notify,
        )
    }

    /// Mutates state, derives a public notification value, and notifies the
    /// current observers with that value.
    ///
    /// This is the state-store primitive for VMs that keep private Rust state
    /// but expose a derived platform state. Concurrent subscription replay is
    /// reconciled using cloned private state snapshots and the subscriber's
    /// projection function.
    pub fn update_with_notify<R: Clone>(
        &self,
        mutate: impl FnOnce(&mut S) -> R,
        notify: impl FnMut(&Arc<O>, R),
    ) -> R {
        let (value, observers, version) = {
            let _observer_sequence = self
                .observer_sequence
                .lock()
                .expect("state store observer sequence lock poisoned");
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let value = mutate(&mut inner.state);
            let state = inner.state.clone();
            Self::record_notification_event(&mut inner, state);
            (value, self.observers.snapshot(), inner.notification_version)
        };
        self.notify_observers(observers, version, value.clone(), notify);
        value
    }

    /// Optionally mutates state and notifies observers only when the mutation
    /// returns `Some`.
    ///
    /// This is useful for no-op actions such as cancelling when no request is
    /// active. The returned value is the cloned post-mutation state when a
    /// notification happened.
    pub fn try_update_notify(
        &self,
        mutate: impl FnOnce(&mut S) -> Option<()>,
        notify: impl FnMut(&Arc<O>, S),
    ) -> Option<S> {
        self.try_update_with_notify(
            |state| {
                mutate(state)?;
                Some(state.clone())
            },
            notify,
        )
    }

    /// Optionally mutates state, derives a public notification value, and only
    /// notifies observers when the mutation returns `Some`.
    ///
    /// Use this for state-driven no-op actions where the platform should not
    /// receive a redundant callback.
    pub fn try_update_with_notify<R: Clone>(
        &self,
        mutate: impl FnOnce(&mut S) -> Option<R>,
        notify: impl FnMut(&Arc<O>, R),
    ) -> Option<R> {
        let (value, observers, version) = {
            let _observer_sequence = self
                .observer_sequence
                .lock()
                .expect("state store observer sequence lock poisoned");
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let value = mutate(&mut inner.state)?;
            let state = inner.state.clone();
            Self::record_notification_event(&mut inner, state);
            (value, self.observers.snapshot(), inner.notification_version)
        };
        self.notify_observers(observers, version, value.clone(), notify);
        Some(value)
    }

    fn notify_observers<R: Clone>(
        &self,
        observers: Vec<Arc<O>>,
        notification_version: u64,
        value: R,
        mut notify: impl FnMut(&Arc<O>, R),
    ) {
        for observer in observers {
            let _callback = self.callback_gate.enter();
            if !self.is_current_notification_version(notification_version) {
                break;
            }
            notify(&observer, value.clone());
        }
    }

    fn is_current_notification_version(&self, version: u64) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .notification_version
            == version
    }
}

/// Metadata emitted by Cross-Kit VM bridge macros.
///
/// Generated metadata is consumed by the Cross-Kit CLI, code generators, and
/// platform packagers. Runtime SDK crates usually only need this trait so their
/// metadata binary can collect VM descriptions.
pub trait CkVmMetadata {
    /// Returns the target-independent VM metadata JSON emitted by the macro.
    ///
    /// Metadata binaries usually collect several of these strings with
    /// [`metadata_json`] and print the resulting JSON array for the CLI.
    fn ck_vm_metadata() -> &'static str;
}

/// Builds the metadata JSON array emitted by SDK metadata binaries.
///
/// Each input string must already be a valid JSON object produced by
/// [`CkVmMetadata::ck_vm_metadata`]. The function joins those objects without
/// escaping so code generators receive the original VM metadata shape.
pub fn metadata_json(metadata: &[&str]) -> String {
    format!("[{}]", metadata.join(","))
}

/// Generates a metadata binary `main` function from an explicit VM list.
///
/// The generated binary prints a JSON array consumed by `cross-kit-cli ios
/// package` and `cross-kit-cli android package`.
#[macro_export]
macro_rules! metadata_main {
    () => {
        compile_error!("metadata_main! requires at least one VM type");
    };
    ($($vm_ty:ty),+ $(,)?) => {
        fn main() {
            let metadata = [
                $(<$vm_ty as $crate::CkVmMetadata>::ck_vm_metadata()),+
            ];
            println!("{}", $crate::metadata_json(&metadata));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{CkVmMetadata, ObserverSet, StateStore, metadata_json};
    use std::sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::time::Duration;

    struct ManualMetadata;

    impl CkVmMetadata for ManualMetadata {
        fn ck_vm_metadata() -> &'static str {
            r#"{"name":"manual"}"#
        }
    }

    #[test]
    fn metadata_trait_is_available_to_runtime_users() {
        let metadata: serde_json::Value =
            serde_json::from_str(ManualMetadata::ck_vm_metadata()).unwrap();
        assert_eq!(metadata["name"], "manual");
    }

    #[test]
    fn metadata_json_builds_valid_array_without_escaping_metadata() {
        let metadata = metadata_json(&[r#"{"name":"first"}"#, r#"{"name":"second"}"#]);
        let parsed: serde_json::Value = serde_json::from_str(&metadata).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["name"], "first");
        assert_eq!(parsed[1]["name"], "second");
    }

    mod generated_metadata_binary {
        struct FirstViewModel;
        struct SecondViewModel;

        impl crate::CkVmMetadata for FirstViewModel {
            fn ck_vm_metadata() -> &'static str {
                r#"{"name":"FirstViewModel"}"#
            }
        }

        impl crate::CkVmMetadata for SecondViewModel {
            fn ck_vm_metadata() -> &'static str {
                r#"{"name":"SecondViewModel"}"#
            }
        }

        crate::metadata_main!(FirstViewModel, SecondViewModel);

        #[test]
        fn metadata_main_runs_without_extra_user_namespace_items() {
            main();
        }
    }

    #[test]
    fn observer_set_returns_unique_ids_and_tracks_len() {
        let observers = ObserverSet::<dyn Fn() + Send + Sync>::default();
        let cloned_observers = observers.clone();

        let first = observers.subscribe(Arc::new(|| {}));
        let second = observers.subscribe(Arc::new(|| {}));

        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(cloned_observers.len(), 2);
        assert_eq!(observers.len(), 2);
        assert!(!observers.is_empty());
    }

    #[test]
    fn observer_set_unsubscribe_stops_future_notifications() {
        let observers = ObserverSet::<dyn Fn() + Send + Sync>::new();
        let hits = Arc::new(Mutex::new(0));
        let first_hits = hits.clone();
        let first = observers.subscribe(Arc::new(move || {
            *first_hits.lock().unwrap() += 1;
        }));
        let second_hits = hits.clone();
        observers.subscribe(Arc::new(move || {
            *second_hits.lock().unwrap() += 10;
        }));

        observers.notify(|observer| observer());
        assert_eq!(*hits.lock().unwrap(), 11);
        *hits.lock().unwrap() = 0;
        assert!(observers.unsubscribe(first));
        assert!(!observers.unsubscribe(first));
        observers.notify(|observer| observer());

        assert_eq!(*hits.lock().unwrap(), 10);
    }

    #[test]
    fn observer_set_notify_allows_unsubscribe_from_callback() {
        let observers = ObserverSet::<dyn Fn() + Send + Sync>::new();
        let weak_set: Arc<Mutex<Weak<ObserverSet<dyn Fn() + Send + Sync>>>> =
            Arc::new(Mutex::new(Weak::new()));
        let remove_id = Arc::new(Mutex::new(0));
        let remove_id_for_callback = remove_id.clone();
        let weak_set_for_callback = weak_set.clone();
        let id = observers.subscribe(Arc::new(move || {
            if let Some(set) = weak_set_for_callback.lock().unwrap().upgrade() {
                set.unsubscribe(*remove_id_for_callback.lock().unwrap());
            }
        }));
        *remove_id.lock().unwrap() = id;
        let shared_set = Arc::new(observers);
        *weak_set.lock().unwrap() = Arc::downgrade(&shared_set);

        shared_set.notify(|observer| observer());

        assert!(shared_set.is_empty());
    }

    #[test]
    fn observer_set_notify_uses_snapshot_for_new_subscribers() {
        let observers = Arc::new(ObserverSet::<dyn Fn() + Send + Sync>::new());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let subscribed_second = Arc::new(AtomicBool::new(false));

        let observers_for_callback = observers.clone();
        let calls_for_callback = calls.clone();
        let subscribed_second_for_callback = subscribed_second.clone();
        observers.subscribe(Arc::new(move || {
            calls_for_callback.lock().unwrap().push("first");
            if !subscribed_second_for_callback.swap(true, Ordering::SeqCst) {
                let calls_for_second = calls_for_callback.clone();
                observers_for_callback.subscribe(Arc::new(move || {
                    calls_for_second.lock().unwrap().push("second");
                }));
            }
        }));

        observers.notify(|observer| observer());
        assert_eq!(calls.lock().unwrap().as_slice(), &["first"]);

        calls.lock().unwrap().clear();
        observers.notify(|observer| observer());
        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls.contains(&"first"));
        assert!(calls.contains(&"second"));
    }

    #[test]
    fn observer_set_notify_allows_reentrant_notify() {
        let observers = Arc::new(ObserverSet::<dyn Fn() + Send + Sync>::new());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let reentered = Arc::new(AtomicBool::new(false));

        let observers_for_callback = observers.clone();
        let calls_for_callback = calls.clone();
        let reentered_for_callback = reentered.clone();
        observers.subscribe(Arc::new(move || {
            calls_for_callback.lock().unwrap().push("call");
            if !reentered_for_callback.swap(true, Ordering::SeqCst) {
                observers_for_callback.notify(|observer| observer());
            }
        }));

        observers.notify(|observer| observer());

        assert_eq!(calls.lock().unwrap().as_slice(), &["call", "call"]);
    }

    #[test]
    fn observer_set_empty_notify_is_noop() {
        let observers = ObserverSet::<dyn Fn() + Send + Sync>::new();

        observers.notify(|observer| observer());
        assert!(observers.is_empty());
    }

    #[test]
    fn state_store_reads_and_updates_state() {
        let store = StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(3);
        let cloned_store = store.clone();

        assert_eq!(store.read(), 3);
        assert_eq!(cloned_store.update(|state| *state += 4), 7);
        assert_eq!(store.read_with(|state| *state * 2), 14);
    }

    #[test]
    fn state_store_replays_current_state_on_subscribe() {
        let store = StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(10);
        store.update(|state| *state += 1);
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_observer = states.clone();
        let observer = Arc::new(move |state| states_for_observer.lock().unwrap().push(state));

        let id = store.subscribe_replay(observer, |observer, state| observer(state));

        assert_eq!(id, 1);
        assert_eq!(states.lock().unwrap().as_slice(), &[11]);
    }

    #[test]
    fn state_store_replays_projected_state_on_subscribe() {
        #[derive(Clone)]
        struct PrivateState {
            value: i32,
        }

        let store = StateStore::<PrivateState, dyn Fn(String) + Send + Sync>::new(PrivateState {
            value: 7,
        });
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_observer = states.clone();
        let observer =
            Arc::new(move |state: String| states_for_observer.lock().unwrap().push(state));

        let id = store.subscribe_replay_with(
            observer,
            |state| format!("value={}", state.value),
            |observer, state| observer(state),
        );

        assert_eq!(id, 1);
        assert_eq!(states.lock().unwrap().as_slice(), &["value=7".to_string()]);
    }

    #[test]
    fn state_store_replays_reentrant_state_change_before_subscribe_returns() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let states = Arc::new(Mutex::new(Vec::new()));
        let triggered = Arc::new(AtomicBool::new(false));

        let store_for_observer = store.clone();
        let states_for_observer = states.clone();
        let triggered_for_observer = triggered.clone();
        let observer = Arc::new(move |state| {
            states_for_observer.lock().unwrap().push(state);
            if state == 0 && !triggered_for_observer.swap(true, Ordering::SeqCst) {
                store_for_observer
                    .update_notify(|state| *state = 1, |observer, state| observer(state));
            }
        });

        let id = store.subscribe_replay(observer, |observer, state| observer(state));
        store.update_notify(|state| *state = 2, |observer, state| observer(state));

        assert_eq!(id, 1);
        assert_eq!(states.lock().unwrap().as_slice(), &[0, 1, 2]);
    }

    #[test]
    fn state_store_replays_silent_update_during_initial_replay() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let states = Arc::new(Mutex::new(Vec::new()));
        let triggered = Arc::new(AtomicBool::new(false));

        let store_for_observer = store.clone();
        let states_for_observer = states.clone();
        let triggered_for_observer = triggered.clone();
        let observer = Arc::new(move |state| {
            states_for_observer.lock().unwrap().push(state);
            if state == 0 && !triggered_for_observer.swap(true, Ordering::SeqCst) {
                store_for_observer.update(|state| *state = 1);
            }
        });

        let id = store.subscribe_replay(observer, |observer, state| observer(state));

        assert_eq!(id, 1);
        assert_eq!(states.lock().unwrap().as_slice(), &[0, 1]);
    }

    #[test]
    fn state_store_silent_update_does_not_stop_current_notification_round() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let triggered = Arc::new(AtomicBool::new(false));

        let first_store = store.clone();
        let first_calls = calls.clone();
        let first_triggered = triggered.clone();
        store.subscribe_replay(
            Arc::new(move |state| {
                first_calls.lock().unwrap().push(("first", state));
                if state == 1 && !first_triggered.swap(true, Ordering::SeqCst) {
                    first_store.update(|state| *state = 2);
                }
            }),
            |_, _| {},
        );

        let second_calls = calls.clone();
        store.subscribe_replay(
            Arc::new(move |state| {
                second_calls.lock().unwrap().push(("second", state));
            }),
            |_, _| {},
        );

        store.update_notify(|state| *state = 1, |observer, state| observer(state));

        let calls = calls.lock().unwrap().clone();
        assert!(calls.contains(&("first", 1)));
        assert!(calls.contains(&("second", 1)));
        assert_eq!(store.read(), 2);
    }

    #[test]
    fn state_store_closes_replay_window_when_replay_panics() {
        let store = StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            store.subscribe_replay(Arc::new(|_state| {}), |_observer, _state| {
                panic!("platform replay failed");
            });
        }));

        assert!(result.is_err());
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_observer = states.clone();
        let id = store.subscribe_replay(
            Arc::new(move |state| states_for_observer.lock().unwrap().push(state)),
            |observer, state| observer(state),
        );
        store.update_notify(|state| *state = 1, |observer, state| observer(state));

        assert_eq!(id, 1);
        assert_eq!(states.lock().unwrap().as_slice(), &[0, 1]);
    }

    #[test]
    fn state_store_update_notify_uses_post_mutation_snapshot() {
        let store = StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0);
        let first_states = Arc::new(Mutex::new(Vec::new()));
        let first_states_for_observer = first_states.clone();
        store.subscribe_replay(
            Arc::new(move |state| first_states_for_observer.lock().unwrap().push(state)),
            |_, _| {},
        );
        let second_states = Arc::new(Mutex::new(Vec::new()));
        let second_states_for_observer = second_states.clone();
        store.subscribe_replay(
            Arc::new(move |state| second_states_for_observer.lock().unwrap().push(state)),
            |_, _| {},
        );

        let updated = store.update_notify(|state| *state = 42, |observer, state| observer(state));

        assert_eq!(updated, 42);
        assert_eq!(first_states.lock().unwrap().as_slice(), &[42]);
        assert_eq!(second_states.lock().unwrap().as_slice(), &[42]);
    }

    #[test]
    fn state_store_unsubscribe_stops_future_notifications() {
        let store = StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(1);
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_observer = states.clone();
        let id = store.subscribe_replay(
            Arc::new(move |state| states_for_observer.lock().unwrap().push(state)),
            |observer, state| observer(state),
        );

        store.update_notify(|state| *state += 1, |observer, state| observer(state));
        assert!(store.unsubscribe(id));
        assert!(!store.unsubscribe(id));
        store.update_notify(|state| *state += 1, |observer, state| observer(state));

        assert_eq!(states.lock().unwrap().as_slice(), &[1, 2]);
    }

    #[test]
    fn state_store_try_update_notify_skips_noop_actions() {
        let store = StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(5);
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_observer = states.clone();
        store.subscribe_replay(
            Arc::new(move |state| states_for_observer.lock().unwrap().push(state)),
            |observer, state| observer(state),
        );

        let skipped = store.try_update_notify(
            |_state| None,
            |observer: &Arc<dyn Fn(i32) + Send + Sync>, state| observer(state),
        );
        let updated = store.try_update_notify(
            |state| {
                *state += 1;
                Some(())
            },
            |observer, state| observer(state),
        );

        assert_eq!(skipped, None);
        assert_eq!(updated, Some(6));
        assert_eq!(states.lock().unwrap().as_slice(), &[5, 6]);
    }

    #[test]
    fn state_store_try_update_notify_stops_stale_round_after_reentrant_update() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let reentered = Arc::new(AtomicBool::new(false));

        for _ in 0..3 {
            let store_for_observer = store.clone();
            let calls_for_observer = calls.clone();
            let reentered_for_observer = reentered.clone();
            store.subscribe_replay(
                Arc::new(move |state| {
                    calls_for_observer.lock().unwrap().push(state);
                    if state == 1 && !reentered_for_observer.swap(true, Ordering::SeqCst) {
                        store_for_observer.try_update_notify(
                            |state| {
                                *state = 2;
                                Some(())
                            },
                            |observer, state| observer(state),
                        );
                    }
                }),
                |_, _| {},
            );
        }

        store.try_update_notify(
            |state| {
                *state = 1;
                Some(())
            },
            |observer, state| observer(state),
        );

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.iter().filter(|state| **state == 1).count(), 1);
        assert_eq!(calls.iter().filter(|state| **state == 2).count(), 3);
        assert!(calls.windows(2).all(|window| window[0] <= window[1]));
    }

    #[test]
    fn state_store_notifies_outside_state_lock() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let states = Arc::new(Mutex::new(Vec::new()));
        let reentered = Arc::new(AtomicBool::new(false));

        let store_for_observer = store.clone();
        let states_for_observer = states.clone();
        let reentered_for_observer = reentered.clone();
        store.subscribe_replay(
            Arc::new(move |state| {
                states_for_observer.lock().unwrap().push(state);
                if state == 1 && !reentered_for_observer.swap(true, Ordering::SeqCst) {
                    store_for_observer
                        .update_notify(|state| *state = 2, |observer, state| observer(state));
                }
            }),
            |_, _| {},
        );

        store.update_notify(|state| *state = 1, |observer, state| observer(state));

        assert_eq!(store.read(), 2);
        assert_eq!(states.lock().unwrap().as_slice(), &[1, 2]);
    }

    #[test]
    fn state_store_serializes_reentrant_notifications_for_all_observers() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let reentered = Arc::new(AtomicBool::new(false));

        let first_store = store.clone();
        let first_calls = calls.clone();
        let first_reentered = reentered.clone();
        store.subscribe_replay(
            Arc::new(move |state| {
                first_calls.lock().unwrap().push(("first", state));
                if state == 1 && !first_reentered.swap(true, Ordering::SeqCst) {
                    first_store
                        .update_notify(|state| *state = 2, |observer, state| observer(state));
                }
            }),
            |_, _| {},
        );

        let second_calls = calls.clone();
        store.subscribe_replay(
            Arc::new(move |state| {
                second_calls.lock().unwrap().push(("second", state));
            }),
            |_, _| {},
        );

        store.update_notify(|state| *state = 1, |observer, state| observer(state));

        let calls = calls.lock().unwrap().clone();
        for observer_name in ["first", "second"] {
            let values = calls
                .iter()
                .filter_map(|(name, value)| (*name == observer_name).then_some(*value))
                .collect::<Vec<_>>();
            assert_eq!(values.last(), Some(&2));
            assert!(values.windows(2).all(|window| window[0] <= window[1]));
        }
    }

    #[test]
    fn state_store_serializes_concurrent_notification_delivery() {
        let store = Arc::new(StateStore::<i32, dyn Fn(i32) + Send + Sync>::new(0));
        let states = Arc::new(Mutex::new(Vec::new()));
        let states_for_observer = states.clone();
        store.subscribe_replay(
            Arc::new(move |state| states_for_observer.lock().unwrap().push(state)),
            |_, _| {},
        );

        let (start_tx, start_rx) = mpsc::channel();
        let store_for_thread = store.clone();
        let concurrent = std::thread::spawn(move || {
            start_rx.recv().unwrap();
            store_for_thread.update_notify(|state| *state = 2, |observer, state| observer(state));
        });
        let started = Arc::new(AtomicBool::new(false));
        let started_for_notify = started.clone();

        store.update_notify(
            |state| *state = 1,
            |observer, state| {
                if state == 1 && !started_for_notify.swap(true, Ordering::SeqCst) {
                    start_tx.send(()).unwrap();
                    std::thread::sleep(Duration::from_millis(100));
                }
                observer(state);
            },
        );
        concurrent.join().unwrap();

        assert_eq!(states.lock().unwrap().as_slice(), &[1, 2]);
    }
}
