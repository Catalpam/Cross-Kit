trait CounterObserver: Send + Sync {
    fn on_state(&self, state: i32);
}

struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn subscribe(&self, callback: std::sync::Arc<dyn CounterObserver>) -> i64 {
        drop(callback);
        1
    }

    pub fn get_state(&self) -> i32 {
        0
    }
}

fn main() {}
