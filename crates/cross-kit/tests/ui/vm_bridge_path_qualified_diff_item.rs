use std::sync::Arc;

mod types {
    pub enum ListDiff {
        Insert { index: i64, item: ListItem },
    }

    pub struct ListItem {
        pub id: i64,
    }
}

trait ListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<types::ListDiff>);
}

struct ListViewModel;

#[cross_kit::vm_bridge(
    mode = "diff_list",
    diff = types::ListDiff,
    item = types::ListItem
)]
impl ListViewModel {
    pub fn subscribe(&self, observer: Arc<dyn ListObserver>) -> i64 {
        drop(observer);
        1
    }

    pub fn append_now(&self) -> types::ListItem {
        types::ListItem { id: 1 }
    }
}

fn main() {
    let envelope: serde_json::Value =
        serde_json::from_str(<ListViewModel as cross_kit::CkVmMetadata>::ck_vm_metadata()).unwrap();
    let metadata: cross_kit::VmMetadata = serde_json::from_value(envelope["ir"].clone()).unwrap();
    assert_eq!(metadata.diff_type.as_deref(), Some("ListDiff"));
    assert_eq!(metadata.list_item_type.as_deref(), Some("ListItem"));
}
