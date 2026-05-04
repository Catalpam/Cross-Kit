enum ListDiff {
    Insert { index: i64, item: ListItem },
}

struct ListItem {
    id: i64,
}

trait ListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<ListDiff>);
}

struct ListViewModel;

#[cross_kit::vm_bridge(
    mode = "diff_list",
    diff = ListDiff,
    diff_type = "OtherDiff",
    item = ListItem
)]
impl ListViewModel {
    pub fn subscribe(&self, observer: std::sync::Arc<dyn ListObserver>) -> i64 {
        drop(observer);
        1
    }
}

fn main() {}
