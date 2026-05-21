use std::sync::Arc;

#[cfg(test)]
use std::sync::Mutex;

pub use cross_kit::CkVmMetadata;
use cross_kit::{StateStore, SubscriptionId, vm_bridge};

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
    state: StateStore<CounterState, dyn CounterObserver>,
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
            state: StateStore::new(CounterState { value: initial }),
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
        self.state.read()
    }

    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> SubscriptionId {
        // New subscribers receive the current state immediately so generated
        // bridges are usable right after construction without a separate load.
        self.state
            .subscribe_replay(observer, |observer, state| observer.on_state(state))
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.state.unsubscribe(id);
    }
}

impl CounterViewModel {
    fn update_by(&self, delta: i32) {
        self.state.update_notify(
            |state| state.value += delta,
            |observer, state| observer.on_state(state),
        );
    }

    fn set_value(&self, value: i32) {
        self.state.update_notify(
            |state| state.value = value,
            |observer, state| observer.on_state(state),
        );
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

    struct ReentrantObserver {
        vm: Mutex<Option<Arc<CounterViewModel>>>,
        states: Mutex<Vec<CounterState>>,
        triggered: Mutex<bool>,
    }

    impl ReentrantObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                vm: Mutex::new(None),
                states: Mutex::new(Vec::new()),
                triggered: Mutex::new(false),
            })
        }

        fn attach(&self, vm: Arc<CounterViewModel>) {
            *self
                .vm
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(vm);
        }

        fn states(&self) -> Vec<CounterState> {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl CounterObserver for ReentrantObserver {
        fn on_state(&self, state: CounterState) {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(state);
            let should_reenter = {
                let mut triggered = self
                    .triggered
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let should_reenter = !*triggered;
                if should_reenter {
                    *triggered = true;
                }
                should_reenter
            };
            if should_reenter {
                if let Some(vm) = self
                    .vm
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .clone()
                {
                    vm.increment();
                }
            }
        }
    }

    #[test]
    fn observer_callbacks_can_reenter_view_model_actions() {
        let vm = CounterViewModel::new(0);
        let observer = ReentrantObserver::new();
        observer.attach(vm.clone());

        vm.subscribe(observer.clone());

        assert_eq!(
            observer.states(),
            vec![CounterState { value: 0 }, CounterState { value: 1 }]
        );
        assert_eq!(vm.get_state(), CounterState { value: 1 });
    }
}
