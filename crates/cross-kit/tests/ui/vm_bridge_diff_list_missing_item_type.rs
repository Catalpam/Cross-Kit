trait ListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<ListDiff>);
}

enum ListDiff {
    Insert { index: i64, item: i32 },
}

struct ListViewModel;

#[cross_kit::vm_bridge(
    mode = "diff_list",
    diff_type = "ListDiff"
)]
impl ListViewModel {
    pub fn subscribe(&self, observer: std::sync::Arc<dyn ListObserver>) -> i64 {
        drop(observer);
        1
    }
}

fn main() {}
