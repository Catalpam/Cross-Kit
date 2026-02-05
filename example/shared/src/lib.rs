use std::sync::{Arc, Mutex};

use chrono::{Datelike, TimeZone, Utc};

uniffi::setup_scaffolding!();

#[derive(Clone, Debug, uniffi::Record)]
pub struct CounterState {
    pub value: i32,
}

#[uniffi::export(with_foreign)]
pub trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

#[derive(uniffi::Object)]
pub struct CounterViewModel {
    state: Arc<Mutex<CounterState>>,
    observers: Arc<Mutex<Vec<Arc<dyn CounterObserver>>>>,
}

#[uniffi::export]
impl CounterViewModel {
    #[uniffi::constructor]
    pub fn new(initial: i32) -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(CounterState { value: initial })),
            observers: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) {
        {
            let mut observers = self.observers.lock().expect("observer lock poisoned");
            observers.push(observer.clone());
        }
        let current = self.get_state();
        observer.on_state(current);
    }

    pub fn increment(&self) -> CounterState {
        let new_state = {
            let mut state = self.state.lock().expect("state lock poisoned");
            state.value += 1;
            state.clone()
        };
        self.notify_state(&new_state);
        new_state
    }

    pub fn get_state(&self) -> CounterState {
        let state = self.state.lock().expect("state lock poisoned");
        state.clone()
    }
}

impl CounterViewModel {
    fn notify_state(&self, state: &CounterState) {
        let observers = {
            let observers = self.observers.lock().expect("observer lock poisoned");
            observers.clone()
        };
        for observer in observers {
            observer.on_state(state.clone());
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ListItem {
    pub id: i64,
    pub timestamp_ms: i64,
    pub date_cn: String,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ListDiff {
    Insert { index: i64, item: ListItem },
    Update { index: i64, item: ListItem },
    Remove { index: i64, id: i64 },
    Move { from: i64, to: i64 },
}

#[uniffi::export(with_foreign)]
pub trait ListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<ListDiff>);
}

#[derive(uniffi::Object)]
pub struct ListViewModel {
    items: Arc<Mutex<Vec<ListItem>>>,
    observers: Arc<Mutex<Vec<Arc<dyn ListObserver>>>>,
    next_id: Arc<Mutex<i64>>,
}

#[uniffi::export]
impl ListViewModel {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            items: Arc::new(Mutex::new(Vec::new())),
            observers: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    pub fn subscribe(&self, observer: Arc<dyn ListObserver>) {
        {
            let mut observers = self.observers.lock().expect("observer lock poisoned");
            observers.push(observer.clone());
        }
        let initial_diffs = {
            let items = self.items.lock().expect("items lock poisoned");
            items
                .iter()
                .enumerate()
                .map(|(index, item)| ListDiff::Insert {
                    index: index as i64,
                    item: item.clone(),
                })
                .collect::<Vec<_>>()
        };
        if !initial_diffs.is_empty() {
            observer.on_diffs(initial_diffs);
        }
    }

    pub fn len(&self) -> i64 {
        let items = self.items.lock().expect("items lock poisoned");
        items.len() as i64
    }

    pub fn append_now(&self) -> ListItem {
        let timestamp_ms = now_timestamp_ms();
        self.insert_with_timestamp_internal(self.len(), timestamp_ms)
            .expect("append should always succeed")
    }

    pub fn insert_now(&self, index: i64) -> Option<ListItem> {
        let timestamp_ms = now_timestamp_ms();
        self.insert_with_timestamp_internal(index, timestamp_ms)
    }

    pub fn insert_with_timestamp(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        self.insert_with_timestamp_internal(index, timestamp_ms)
    }

    pub fn update_with_timestamp(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        let mut items = self.items.lock().expect("items lock poisoned");
        let idx = to_index(index, items.len())?;
        let updated = ListItem {
            id: items[idx].id,
            timestamp_ms,
            date_cn: date_cn_from_timestamp_ms(timestamp_ms),
        };
        items[idx] = updated.clone();
        drop(items);
        self.emit_diffs(vec![ListDiff::Update {
            index,
            item: updated.clone(),
        }]);
        Some(updated)
    }

    pub fn remove_at(&self, index: i64) -> Option<ListItem> {
        let mut items = self.items.lock().expect("items lock poisoned");
        let idx = to_index(index, items.len())?;
        let removed = items.remove(idx);
        drop(items);
        self.emit_diffs(vec![ListDiff::Remove {
            index,
            id: removed.id,
        }]);
        Some(removed)
    }

    pub fn move_item(&self, from: i64, to: i64) -> bool {
        let mut items = self.items.lock().expect("items lock poisoned");
        let len = items.len();
        let from_idx = match to_index(from, len) {
            Some(value) if value < len => value,
            _ => return false,
        };
        let to_idx = match to_index(to, len) {
            Some(value) if value < len => value,
            _ => return false,
        };
        if from_idx == to_idx {
            return true;
        }
        let item = items.remove(from_idx);
        let adjusted_to = if from_idx < to_idx { to_idx - 1 } else { to_idx };
        items.insert(adjusted_to, item);
        drop(items);
        self.emit_diffs(vec![ListDiff::Move { from, to }]);
        true
    }

    pub fn sort_by_timestamp_desc(&self) -> bool {
        let mut items = self.items.lock().expect("items lock poisoned");
        let mut desired = items.clone();
        desired.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        if desired == *items {
            return false;
        }

        let mut diffs = Vec::new();
        for target_index in 0..desired.len() {
            let desired_id = desired[target_index].id;
            let current_index = items
                .iter()
                .position(|item| item.id == desired_id)
                .expect("item should exist");
            if current_index != target_index {
                let item = items.remove(current_index);
                items.insert(target_index, item);
                diffs.push(ListDiff::Move {
                    from: current_index as i64,
                    to: target_index as i64,
                });
            }
        }
        drop(items);
        if !diffs.is_empty() {
            self.emit_diffs(diffs);
        }
        true
    }

    pub fn apply_diffs(&self, diffs: Vec<ListDiff>) -> bool {
        if diffs.is_empty() {
            return true;
        }
        let mut items = self.items.lock().expect("items lock poisoned");
        for diff in &diffs {
            match diff {
                ListDiff::Insert { index, item } => {
                    let idx = match to_insert_index(*index, items.len()) {
                        Some(value) => value,
                        None => return false,
                    };
                    items.insert(idx, item.clone());
                }
                ListDiff::Update { index, item } => {
                    let idx = match to_index(*index, items.len()) {
                        Some(value) => value,
                        None => return false,
                    };
                    items[idx] = item.clone();
                }
                ListDiff::Remove { index, .. } => {
                    let idx = match to_index(*index, items.len()) {
                        Some(value) => value,
                        None => return false,
                    };
                    items.remove(idx);
                }
                ListDiff::Move { from, to } => {
                    let len = items.len();
                    let from_idx = match to_index(*from, len) {
                        Some(value) => value,
                        None => return false,
                    };
                    let to_idx = match to_index(*to, len) {
                        Some(value) => value,
                        None => return false,
                    };
                    if from_idx == to_idx {
                        continue;
                    }
                    let item = items.remove(from_idx);
                    let adjusted_to = if from_idx < to_idx { to_idx - 1 } else { to_idx };
                    items.insert(adjusted_to, item);
                }
            }
        }
        drop(items);
        self.emit_diffs(diffs);
        true
    }
}

impl ListViewModel {
    fn insert_with_timestamp_internal(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        let mut items = self.items.lock().expect("items lock poisoned");
        let idx = to_insert_index(index, items.len())?;
        let item = ListItem {
            id: next_id(&self.next_id),
            timestamp_ms,
            date_cn: date_cn_from_timestamp_ms(timestamp_ms),
        };
        items.insert(idx, item.clone());
        drop(items);
        self.emit_diffs(vec![ListDiff::Insert {
            index,
            item: item.clone(),
        }]);
        Some(item)
    }

    fn emit_diffs(&self, diffs: Vec<ListDiff>) {
        let observers = {
            let observers = self.observers.lock().expect("observer lock poisoned");
            observers.clone()
        };
        for observer in observers {
            observer.on_diffs(diffs.clone());
        }
    }
}

fn next_id(counter: &Arc<Mutex<i64>>) -> i64 {
    let mut guard = counter.lock().expect("id lock poisoned");
    let id = *guard;
    *guard += 1;
    id
}

fn now_timestamp_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch");
    now.as_millis() as i64
}

fn date_cn_from_timestamp_ms(timestamp_ms: i64) -> String {
    let dt = Utc.timestamp_millis_opt(timestamp_ms).single().unwrap_or_else(|| {
        Utc.timestamp_millis_opt(0)
            .single()
            .expect("timestamp zero should be valid")
    });
    format!(
        "{:04}年{:02}月{:02}日",
        dt.year(),
        dt.month(),
        dt.day()
    )
}

fn to_index(index: i64, len: usize) -> Option<usize> {
    if index < 0 {
        return None;
    }
    let idx = index as usize;
    if idx >= len {
        None
    } else {
        Some(idx)
    }
}

fn to_insert_index(index: i64, len: usize) -> Option<usize> {
    if index < 0 {
        return None;
    }
    let idx = index as usize;
    if idx > len {
        None
    } else {
        Some(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct CounterObserverSink {
        states: Mutex<Vec<i32>>,
    }

    impl CounterObserverSink {
        fn new() -> Self {
            Self {
                states: Mutex::new(Vec::new()),
            }
        }
    }

    impl CounterObserver for CounterObserverSink {
        fn on_state(&self, state: CounterState) {
            self.states.lock().unwrap().push(state.value);
        }
    }

    #[test]
    fn counter_emits_state_changes() {
        let vm = CounterViewModel::new(3);
        let observer = Arc::new(CounterObserverSink::new());
        vm.subscribe(observer.clone());
        assert_eq!(observer.states.lock().unwrap().as_slice(), &[3]);
        let state = vm.increment();
        assert_eq!(state.value, 4);
        assert_eq!(observer.states.lock().unwrap().as_slice(), &[3, 4]);
    }

    #[derive(Debug)]
    struct ListObserverSink {
        calls: Mutex<Vec<Vec<ListDiff>>>,
        call_count: AtomicUsize,
    }

    impl ListObserverSink {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                call_count: AtomicUsize::new(0),
            }
        }
    }

    impl ListObserver for ListObserverSink {
        fn on_diffs(&self, diffs: Vec<ListDiff>) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            self.calls.lock().unwrap().push(diffs);
        }
    }

    #[test]
    fn list_subscribe_pushes_diff_inserts() {
        let vm = ListViewModel::new();
        let _ = vm.insert_with_timestamp(0, 1_000).unwrap();
        let _ = vm.insert_with_timestamp(1, 2_000).unwrap();

        let observer = Arc::new(ListObserverSink::new());
        vm.subscribe(observer.clone());

        let calls = observer.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].len(), 2);
        match &calls[0][0] {
            ListDiff::Insert { index, item } => {
                assert_eq!(*index, 0);
                assert_eq!(item.timestamp_ms, 1_000);
            }
            _ => panic!("expected insert diff"),
        }
    }

    #[test]
    fn list_insert_update_remove_emit_diffs() {
        let vm = ListViewModel::new();
        let observer = Arc::new(ListObserverSink::new());
        vm.subscribe(observer.clone());

        let item = vm.insert_with_timestamp(0, 10_000).unwrap();
        assert_eq!(item.id, 1);
        let updated = vm.update_with_timestamp(0, 20_000).unwrap();
        assert_eq!(updated.id, item.id);
        let removed = vm.remove_at(0).unwrap();
        assert_eq!(removed.id, item.id);

        let calls = observer.calls.lock().unwrap();
        assert_eq!(calls.len(), 3);
        assert!(matches!(calls[0][0], ListDiff::Insert { .. }));
        assert!(matches!(calls[1][0], ListDiff::Update { .. }));
        assert!(matches!(calls[2][0], ListDiff::Remove { .. }));
    }

    #[test]
    fn list_move_and_sort_emit_moves() {
        let vm = ListViewModel::new();
        vm.insert_with_timestamp(0, 1_000).unwrap();
        vm.insert_with_timestamp(1, 3_000).unwrap();
        vm.insert_with_timestamp(2, 2_000).unwrap();

        let observer = Arc::new(ListObserverSink::new());
        vm.subscribe(observer.clone());
        observer.calls.lock().unwrap().clear();

        assert!(vm.move_item(0, 2));
        assert_eq!(vm.len(), 3);

        let calls = observer.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(matches!(calls[0][0], ListDiff::Move { .. }));
        drop(calls);

        observer.calls.lock().unwrap().clear();
        assert!(vm.sort_by_timestamp_desc());
        let calls = observer.calls.lock().unwrap();
        assert!(!calls.is_empty());
        assert!(calls[0].iter().all(|d| matches!(d, ListDiff::Move { .. })));
    }

    #[test]
    fn list_apply_batch_diffs() {
        let vm = ListViewModel::new();
        let observer = Arc::new(ListObserverSink::new());
        vm.subscribe(observer.clone());

        let item_a = ListItem {
            id: 10,
            timestamp_ms: 1,
            date_cn: date_cn_from_timestamp_ms(1),
        };
        let item_b = ListItem {
            id: 11,
            timestamp_ms: 2,
            date_cn: date_cn_from_timestamp_ms(2),
        };

        let diffs = vec![
            ListDiff::Insert {
                index: 0,
                item: item_a.clone(),
            },
            ListDiff::Insert {
                index: 1,
                item: item_b.clone(),
            },
            ListDiff::Move { from: 0, to: 1 },
            ListDiff::Update {
                index: 0,
                item: ListItem {
                    id: 11,
                    timestamp_ms: 3,
                    date_cn: date_cn_from_timestamp_ms(3),
                },
            },
            ListDiff::Remove { index: 1, id: 10 },
        ];

        assert!(vm.apply_diffs(diffs.clone()));
        let calls = observer.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], diffs);
    }

    #[test]
    fn list_invalid_indices_fail() {
        let vm = ListViewModel::new();
        assert!(vm.update_with_timestamp(0, 1).is_none());
        assert!(vm.remove_at(0).is_none());
        assert!(!vm.move_item(-1, 0));
        assert!(vm.insert_with_timestamp(-1, 1).is_none());
    }
}
