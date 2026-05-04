struct CounterViewModel;

#[cross_kit::vm_bridge(mode = "state")]
impl CounterViewModel {
    pub fn subscribe(&self, observer: i32) -> i64 {
        let _ = observer;
        1
    }

    pub fn get_state(&self) -> i32 {
        0
    }
}

fn main() {}
