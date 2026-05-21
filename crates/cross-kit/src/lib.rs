//! Runtime entry point for Rust SDKs that integrate with Cross-Kit.
//!
//! Rust SDK crates should depend on this crate instead of depending on
//! Cross-Kit internal crates directly.

#![warn(missing_docs)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    use super::{CkVmMetadata, ObserverSet, metadata_json};
    use std::sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    };

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
}
