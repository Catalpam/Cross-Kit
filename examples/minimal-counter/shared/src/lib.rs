use std::sync::{Arc, Mutex};

pub use cross_kit::CkVmMetadata;
use cross_kit::{ObserverSet, SubscriptionId, vm_bridge};

uniffi::setup_scaffolding!();

// Minimal Counter is the smallest Cross-Kit shape:
// one Rust-owned state record, one observer trait, and one exported VM. The
// generated platform libraries turn this into `kit.counter.state` plus a few
// synchronous action methods on iOS and Android.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CounterState {
    pub value: i32,
}

// Platform bridges implement this trait for us. Rust calls `on_state` after
// each mutation; SwiftUI/Compose only need to render the generated bridge state.
#[uniffi::export(with_foreign)]
pub trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

#[derive(uniffi::Object)]
pub struct CounterViewModel {
    state: Mutex<CounterState>,
    observers: ObserverSet<dyn CounterObserver>,
}

// `vm_bridge(mode = "state")` is the Cross-Kit contract for a state VM. The
// macro reads this impl, infers the bridge/state/observer names, and emits
// metadata consumed by the iOS and Android packagers.
#[vm_bridge(mode = "state")]
#[uniffi::export]
impl CounterViewModel {
    #[uniffi::constructor]
    pub fn new(initial: i32) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(CounterState { value: initial }),
            observers: ObserverSet::new(),
        })
    }

    pub fn increment(&self) {
        self.update_by(1);
    }

    pub fn decrement(&self) {
        self.update_by(-1);
    }

    pub fn reset(&self) {
        self.set_value(0);
    }

    pub fn get_state(&self) -> CounterState {
        self.locked_state()
    }

    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> SubscriptionId {
        let state = self.locked_state();
        let subscription_id = self.observers.subscribe(observer.clone());
        // New subscribers receive the current state immediately so generated
        // bridges are usable right after construction without a separate load.
        observer.on_state(state);
        subscription_id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.observers.unsubscribe(id);
    }
}

impl CounterViewModel {
    fn update_by(&self, delta: i32) {
        let state = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.value += delta;
            state.clone()
        };
        self.notify(state);
    }

    fn set_value(&self, value: i32) {
        let state = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.value = value;
            state.clone()
        };
        self.notify(state);
    }

    fn locked_state(&self) -> CounterState {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn notify(&self, state: CounterState) {
        let observers = self.observers.snapshot();
        ObserverSet::notify_snapshot(&observers, |observer| {
            observer.on_state(state.clone());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingObserver {
        states: Mutex<Vec<CounterState>>,
    }

    impl RecordingObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                states: Mutex::new(Vec::new()),
            })
        }

        fn states(&self) -> Vec<CounterState> {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl CounterObserver for RecordingObserver {
        fn on_state(&self, state: CounterState) {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(state);
        }
    }

    #[test]
    fn starts_from_constructor_initial_value() {
        let vm = CounterViewModel::new(7);

        assert_eq!(vm.get_state(), CounterState { value: 7 });
    }

    #[test]
    fn increment_decrement_and_reset_update_state() {
        let vm = CounterViewModel::new(2);

        vm.increment();
        vm.increment();
        vm.decrement();
        assert_eq!(vm.get_state(), CounterState { value: 3 });

        vm.reset();
        assert_eq!(vm.get_state(), CounterState { value: 0 });
    }

    #[test]
    fn reset_notifies_even_when_value_is_already_zero() {
        let vm = CounterViewModel::new(0);
        let observer = RecordingObserver::new();
        vm.subscribe(observer.clone());

        vm.reset();

        assert_eq!(
            observer.states(),
            vec![CounterState { value: 0 }, CounterState { value: 0 }]
        );
    }

    #[test]
    fn subscription_immediately_receives_current_state() {
        let vm = CounterViewModel::new(4);
        vm.increment();
        let observer = RecordingObserver::new();

        vm.subscribe(observer.clone());

        assert_eq!(observer.states(), vec![CounterState { value: 5 }]);
    }

    #[test]
    fn unsubscribe_stops_future_notifications() {
        let vm = CounterViewModel::new(1);
        let observer = RecordingObserver::new();
        let subscription_id = vm.subscribe(observer.clone());

        vm.increment();
        vm.unsubscribe(subscription_id);
        vm.increment();

        assert_eq!(
            observer.states(),
            vec![CounterState { value: 1 }, CounterState { value: 2 }]
        );
    }
}
