use std::sync::Arc;

mod models {
    pub struct CounterState {
        pub value: i64,
    }
}

mod observers {
    use crate::models::CounterState;

    pub trait CounterObserver: Send + Sync {
        fn on_state(&self, state: CounterState);
    }
}

struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn crate::observers::CounterObserver>) -> i64 {
        drop(observer);
        1
    }

    pub fn get_state(&self) -> crate::models::CounterState {
        crate::models::CounterState { value: 1 }
    }
}

fn main() {
    let envelope: serde_json::Value =
        serde_json::from_str(<CounterViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata())
            .unwrap();
    let metadata: cross_kit::VmMetadata = serde_json::from_value(envelope["ir"].clone()).unwrap();
    assert_eq!(metadata.state_type.as_deref(), Some("CounterState"));
    assert_eq!(
        metadata.observer.as_ref().unwrap().rust_type,
        "CounterObserver"
    );
}
