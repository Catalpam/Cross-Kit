trait CounterObserver: Send + Sync {
    fn on_state(&self, state: i32);
}

struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn subscribe(&self, observer: std::sync::Arc<dyn CounterObserver>) -> i64 {
        drop(observer);
        1
    }
}

fn main() {}
