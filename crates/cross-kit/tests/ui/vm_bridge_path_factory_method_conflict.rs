struct AppViewModel;
struct CounterState {
    value: i64,
}

trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

struct CounterViewModel;

#[cross_kit::vm_bridge(
    mode = "state",
    factory = AppViewModel::make_counter_vm,
    factory_method = "make_other_vm"
)]
impl CounterViewModel {
    pub fn subscribe(&self, observer: std::sync::Arc<dyn CounterObserver>) -> i64 {
        drop(observer);
        1
    }

    pub fn get_state(&self) -> CounterState {
        CounterState { value: 1 }
    }
}

fn main() {}
