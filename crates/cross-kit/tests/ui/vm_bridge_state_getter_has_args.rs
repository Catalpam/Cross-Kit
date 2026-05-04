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

    pub fn get_state(&self, force: bool) -> i32 {
        if force { 1 } else { 0 }
    }
}

fn main() {}
