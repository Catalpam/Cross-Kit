use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{Datelike, TimeZone, Utc};
pub use cross_kit::CkVmMetadata;
use cross_kit::vm_bridge;

uniffi::setup_scaffolding!();

#[derive(Clone, Debug, uniffi::Record)]
pub struct CounterState {
    pub value: i32,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct AppState {
    pub counter: CounterState,
    pub list_len: i64,
    pub last_item: Option<ListItem>,
    pub route: Option<Route>,
}

#[derive(Clone, Debug, uniffi::Record)]
pub struct ActionLogEntry {
    pub name: String,
    pub timestamp_ms: i64,
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

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum Route {
    ListDetail { id: i64, date_cn: String },
    Summary,
}

#[uniffi::export(with_foreign)]
pub trait CounterObserver: Send + Sync {
    fn on_state(&self, state: CounterState);
}

#[uniffi::export(with_foreign)]
pub trait ListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<ListDiff>);
}

#[uniffi::export(with_foreign)]
pub trait AppObserver: Send + Sync {
    fn on_state(&self, state: AppState);
}

#[derive(Clone)]
struct Store {
    inner: Arc<Mutex<StoreInner>>,
}

struct StoreInner {
    counter: CounterState,
    list: Vec<ListItem>,
    next_list_id: i64,
    route: Option<Route>,
    app_observers: HashMap<i64, Arc<dyn AppObserver>>,
    counter_observers: HashMap<i64, Arc<dyn CounterObserver>>,
    list_observers: HashMap<i64, Arc<dyn ListObserver>>,
    action_log: Vec<ActionLogEntry>,
    next_observer_id: i64,
}

impl Store {
    fn new(initial_counter: i32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(StoreInner {
                counter: CounterState {
                    value: initial_counter,
                },
                list: Vec::new(),
                next_list_id: 1,
                route: None,
                app_observers: HashMap::new(),
                counter_observers: HashMap::new(),
                list_observers: HashMap::new(),
                action_log: Vec::new(),
                next_observer_id: 1,
            })),
        }
    }

    fn app_state(inner: &StoreInner) -> AppState {
        AppState {
            counter: inner.counter.clone(),
            list_len: inner.list.len() as i64,
            last_item: inner.list.last().cloned(),
            route: inner.route.clone(),
        }
    }

    fn log_action(inner: &mut StoreInner, name: &str) {
        inner.action_log.push(ActionLogEntry {
            name: name.to_string(),
            timestamp_ms: now_timestamp_ms(),
        });
    }

    fn next_observer_id(inner: &mut StoreInner) -> i64 {
        let id = inner.next_observer_id;
        inner.next_observer_id += 1;
        id
    }

    fn subscribe_app(&self, observer: Arc<dyn AppObserver>) -> i64 {
        let (id, state) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            let id = Self::next_observer_id(&mut inner);
            inner.app_observers.insert(id, observer.clone());
            (id, Self::app_state(&inner))
        };
        observer.on_state(state);
        id
    }

    fn unsubscribe_app(&self, id: i64) {
        let mut inner = self.inner.lock().expect("store lock poisoned");
        inner.app_observers.remove(&id);
    }

    fn subscribe_counter(&self, observer: Arc<dyn CounterObserver>) -> i64 {
        let (id, state) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            let id = Self::next_observer_id(&mut inner);
            inner.counter_observers.insert(id, observer.clone());
            (id, inner.counter.clone())
        };
        observer.on_state(state);
        id
    }

    fn unsubscribe_counter(&self, id: i64) {
        let mut inner = self.inner.lock().expect("store lock poisoned");
        inner.counter_observers.remove(&id);
    }

    fn subscribe_list(&self, observer: Arc<dyn ListObserver>) -> i64 {
        let (id, diffs) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            let id = Self::next_observer_id(&mut inner);
            inner.list_observers.insert(id, observer.clone());
            let diffs = inner
                .list
                .iter()
                .enumerate()
                .map(|(index, item)| ListDiff::Insert {
                    index: index as i64,
                    item: item.clone(),
                })
                .collect::<Vec<_>>();
            (id, diffs)
        };
        if !diffs.is_empty() {
            observer.on_diffs(diffs);
        }
        id
    }

    fn unsubscribe_list(&self, id: i64) {
        let mut inner = self.inner.lock().expect("store lock poisoned");
        inner.list_observers.remove(&id);
    }

    fn counter_increment(&self) -> CounterState {
        let (counter_state, list_diffs, app_state, app_observers, counter_observers, list_observers) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "counter_increment");
            inner.counter.value += 1;
            let mut list_diffs = Vec::new();
            if inner.counter.value % 3 == 0 {
                let timestamp_ms = now_timestamp_ms();
                let item = ListItem {
                    id: inner.next_list_id,
                    timestamp_ms,
                    date_cn: date_cn_from_timestamp_ms(timestamp_ms),
                };
                inner.next_list_id += 1;
                let index = inner.list.len() as i64;
                inner.list.push(item.clone());
                list_diffs.push(ListDiff::Insert { index, item: item.clone() });
                inner.route = Some(Route::ListDetail { id: item.id, date_cn: item.date_cn.clone() });
            }
            let counter_state = inner.counter.clone();
            let app_state = Some(Self::app_state(&inner));
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let counter_observers = inner.counter_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = if list_diffs.is_empty() {
                Vec::new()
            } else {
                inner.list_observers.values().cloned().collect::<Vec<_>>()
            };
            (counter_state, list_diffs, app_state, app_observers, counter_observers, list_observers)
        };

        for observer in counter_observers {
            observer.on_state(counter_state.clone());
        }
        if let Some(state) = app_state {
            for observer in app_observers {
                observer.on_state(state.clone());
            }
        }
        if !list_diffs.is_empty() {
            for observer in list_observers {
                observer.on_diffs(list_diffs.clone());
            }
        }
        counter_state
    }

    fn clear_route(&self) {
        let (app_state, app_observers) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            if inner.route.is_none() {
                return;
            }
            Self::log_action(&mut inner, "clear_route");
            inner.route = None;
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            (app_state, app_observers)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
    }

    fn route_summary(&self) -> bool {
        let (app_state, app_observers, changed) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            if inner.list.is_empty() {
                return false;
            }
            if inner.route.is_some() {
                return false;
            }
            Self::log_action(&mut inner, "route_summary");
            inner.route = Some(Route::Summary);
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            (app_state, app_observers, true)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
        changed
    }

    fn list_len(&self) -> i64 {
        let inner = self.inner.lock().expect("store lock poisoned");
        inner.list.len() as i64
    }

    fn list_append_now(&self) -> ListItem {
        let timestamp_ms = now_timestamp_ms();
        self.list_insert_with_timestamp(self.list_len(), timestamp_ms)
            .expect("append should always succeed")
    }

    fn list_insert_now(&self, index: i64) -> Option<ListItem> {
        let timestamp_ms = now_timestamp_ms();
        self.list_insert_with_timestamp(index, timestamp_ms)
    }

    fn list_insert_with_timestamp(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        let (item, list_diffs, app_state, app_observers, list_observers) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "list_insert_with_timestamp");
            let prev_len = inner.list.len();
            let idx = to_insert_index(index, inner.list.len())?;
            let item = ListItem {
                id: inner.next_list_id,
                timestamp_ms,
                date_cn: date_cn_from_timestamp_ms(timestamp_ms),
            };
            inner.next_list_id += 1;
            inner.list.insert(idx, item.clone());
            if prev_len < 2 && inner.list.len() >= 2 && inner.route.is_none() {
                Self::log_action(&mut inner, "route_summary");
                inner.route = Some(Route::Summary);
            }
            let list_diffs = vec![ListDiff::Insert { index, item: item.clone() }];
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = inner.list_observers.values().cloned().collect::<Vec<_>>();
            (item, list_diffs, app_state, app_observers, list_observers)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
        for observer in list_observers {
            observer.on_diffs(list_diffs.clone());
        }
        Some(item)
    }

    fn list_update_with_timestamp(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        let (item, list_diffs, app_state, app_observers, list_observers) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "list_update_with_timestamp");
            let idx = to_index(index, inner.list.len())?;
            let updated = ListItem {
                id: inner.list[idx].id,
                timestamp_ms,
                date_cn: date_cn_from_timestamp_ms(timestamp_ms),
            };
            inner.list[idx] = updated.clone();
            let list_diffs = vec![ListDiff::Update { index, item: updated.clone() }];
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = inner.list_observers.values().cloned().collect::<Vec<_>>();
            (updated, list_diffs, app_state, app_observers, list_observers)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
        for observer in list_observers {
            observer.on_diffs(list_diffs.clone());
        }
        Some(item)
    }

    fn list_remove_at(&self, index: i64) -> Option<ListItem> {
        let (item, list_diffs, app_state, app_observers, list_observers) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "list_remove_at");
            let idx = to_index(index, inner.list.len())?;
            let removed = inner.list.remove(idx);
            let list_diffs = vec![ListDiff::Remove { index, id: removed.id }];
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = inner.list_observers.values().cloned().collect::<Vec<_>>();
            (removed, list_diffs, app_state, app_observers, list_observers)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
        for observer in list_observers {
            observer.on_diffs(list_diffs.clone());
        }
        Some(item)
    }

    fn list_move_item(&self, from: i64, to: i64) -> bool {
        let (list_diffs, app_state, app_observers, list_observers, success) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "list_move_item");
            let len = inner.list.len();
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
            let item = inner.list.remove(from_idx);
            let adjusted_to = if from_idx < to_idx { to_idx - 1 } else { to_idx };
            inner.list.insert(adjusted_to, item);
            let list_diffs = vec![ListDiff::Move { from, to }];
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = inner.list_observers.values().cloned().collect::<Vec<_>>();
            (list_diffs, app_state, app_observers, list_observers, true)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
        for observer in list_observers {
            observer.on_diffs(list_diffs.clone());
        }
        success
    }

    fn list_sort_by_timestamp_desc(&self) -> bool {
        let (list_diffs, app_state, app_observers, list_observers, changed) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "list_sort_by_timestamp_desc");
            let mut desired = inner.list.clone();
            desired.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
            if desired == inner.list {
                return false;
            }

            let mut diffs = Vec::new();
            for target_index in 0..desired.len() {
                let desired_id = desired[target_index].id;
                let current_index = inner
                    .list
                    .iter()
                    .position(|item| item.id == desired_id)
                    .expect("item should exist");
                if current_index != target_index {
                    let item = inner.list.remove(current_index);
                    inner.list.insert(target_index, item);
                    diffs.push(ListDiff::Move {
                        from: current_index as i64,
                        to: target_index as i64,
                    });
                }
            }

            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = inner.list_observers.values().cloned().collect::<Vec<_>>();
            (diffs, app_state, app_observers, list_observers, true)
        };

        if !list_diffs.is_empty() {
            for observer in list_observers {
                observer.on_diffs(list_diffs.clone());
            }
            for observer in app_observers {
                observer.on_state(app_state.clone());
            }
        }
        changed
    }

    fn list_apply_diffs(&self, diffs: Vec<ListDiff>) -> bool {
        if diffs.is_empty() {
            return true;
        }

        let (app_state, app_observers, list_observers, ok) = {
            let mut inner = self.inner.lock().expect("store lock poisoned");
            Self::log_action(&mut inner, "list_apply_diffs");
            for diff in &diffs {
                match diff {
                    ListDiff::Insert { index, item } => {
                        let idx = match to_insert_index(*index, inner.list.len()) {
                            Some(value) => value,
                            None => return false,
                        };
                        inner.list.insert(idx, item.clone());
                    }
                    ListDiff::Update { index, item } => {
                        let idx = match to_index(*index, inner.list.len()) {
                            Some(value) => value,
                            None => return false,
                        };
                        inner.list[idx] = item.clone();
                    }
                    ListDiff::Remove { index, .. } => {
                        let idx = match to_index(*index, inner.list.len()) {
                            Some(value) => value,
                            None => return false,
                        };
                        inner.list.remove(idx);
                    }
                    ListDiff::Move { from, to } => {
                        let len = inner.list.len();
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
                        let item = inner.list.remove(from_idx);
                        let adjusted_to = if from_idx < to_idx { to_idx - 1 } else { to_idx };
                        inner.list.insert(adjusted_to, item);
                    }
                }
            }
            let app_state = Self::app_state(&inner);
            let app_observers = inner.app_observers.values().cloned().collect::<Vec<_>>();
            let list_observers = inner.list_observers.values().cloned().collect::<Vec<_>>();
            (app_state, app_observers, list_observers, true)
        };

        for observer in app_observers {
            observer.on_state(app_state.clone());
        }
        for observer in list_observers {
            observer.on_diffs(diffs.clone());
        }
        ok
    }

    fn action_log(&self) -> Vec<ActionLogEntry> {
        let inner = self.inner.lock().expect("store lock poisoned");
        inner.action_log.clone()
    }
}

#[derive(uniffi::Object)]
pub struct AppViewModel {
    store: Store,
}

#[vm_bridge(mode = "state")]
#[uniffi::export]
impl AppViewModel {
    #[uniffi::constructor]
    pub fn new(initial: i32) -> Arc<Self> {
        Arc::new(Self {
            store: Store::new(initial),
        })
    }

    pub fn subscribe(&self, observer: Arc<dyn AppObserver>) -> i64 {
        self.store.subscribe_app(observer)
    }

    pub fn unsubscribe(&self, id: i64) {
        self.store.unsubscribe_app(id)
    }

    pub fn get_state(&self) -> AppState {
        let inner = self.store.inner.lock().expect("store lock poisoned");
        Store::app_state(&inner)
    }

    pub fn clear_route(&self) {
        self.store.clear_route();
    }

    pub fn request_summary(&self) -> bool {
        self.store.route_summary()
    }

    pub fn make_counter_vm(&self) -> Arc<CounterViewModel> {
        Arc::new(CounterViewModel {
            store: self.store.clone(),
        })
    }

    pub fn make_list_vm(&self) -> Arc<ListViewModel> {
        Arc::new(ListViewModel {
            store: self.store.clone(),
        })
    }

    pub fn action_log(&self) -> Vec<ActionLogEntry> {
        self.store.action_log()
    }
}

#[derive(uniffi::Object)]
pub struct CounterViewModel {
    store: Store,
}

#[vm_bridge(
    mode = "state",
    factory = AppViewModel::make_counter_vm
)]
#[uniffi::export]
impl CounterViewModel {
    pub fn subscribe(&self, observer: Arc<dyn CounterObserver>) -> i64 {
        self.store.subscribe_counter(observer)
    }

    pub fn unsubscribe(&self, id: i64) {
        self.store.unsubscribe_counter(id)
    }

    pub fn increment(&self) -> CounterState {
        self.store.counter_increment()
    }

    pub fn get_state(&self) -> CounterState {
        let inner = self.store.inner.lock().expect("store lock poisoned");
        inner.counter.clone()
    }
}

#[derive(uniffi::Object)]
pub struct ListViewModel {
    store: Store,
}

#[vm_bridge(
    mode = "diff_list",
    diff = ListDiff,
    item = ListItem,
    factory = AppViewModel::make_list_vm
)]
#[uniffi::export]
impl ListViewModel {
    pub fn subscribe(&self, observer: Arc<dyn ListObserver>) -> i64 {
        self.store.subscribe_list(observer)
    }

    pub fn unsubscribe(&self, id: i64) {
        self.store.unsubscribe_list(id)
    }

    pub fn len(&self) -> i64 {
        self.store.list_len()
    }

    pub fn append_now(&self) -> ListItem {
        self.store.list_append_now()
    }

    pub fn insert_now(&self, index: i64) -> Option<ListItem> {
        self.store.list_insert_now(index)
    }

    pub fn insert_with_timestamp(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        self.store.list_insert_with_timestamp(index, timestamp_ms)
    }

    pub fn update_with_timestamp(&self, index: i64, timestamp_ms: i64) -> Option<ListItem> {
        self.store.list_update_with_timestamp(index, timestamp_ms)
    }

    pub fn remove_at(&self, index: i64) -> Option<ListItem> {
        self.store.list_remove_at(index)
    }

    pub fn move_item(&self, from: i64, to: i64) -> bool {
        self.store.list_move_item(from, to)
    }

    pub fn sort_by_timestamp_desc(&self) -> bool {
        self.store.list_sort_by_timestamp_desc()
    }

    pub fn apply_diffs(&self, diffs: Vec<ListDiff>) -> bool {
        self.store.list_apply_diffs(diffs)
    }
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
    use serde_json::Value;
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

    #[derive(Debug)]
    struct AppObserverSink {
        states: Mutex<Vec<AppState>>,
    }

    impl AppObserverSink {
        fn new() -> Self {
            Self {
                states: Mutex::new(Vec::new()),
            }
        }
    }

    impl AppObserver for AppObserverSink {
        fn on_state(&self, state: AppState) {
            self.states.lock().unwrap().push(state);
        }
    }

    #[test]
    fn counter_emits_state_changes_and_triggers_route() {
        let app = AppViewModel::new(2);
        let counter_vm = app.make_counter_vm();
        let list_vm = app.make_list_vm();

        let counter_observer = Arc::new(CounterObserverSink::new());
        counter_vm.subscribe(counter_observer.clone());
        assert_eq!(counter_observer.states.lock().unwrap().as_slice(), &[2]);

        let app_observer = Arc::new(AppObserverSink::new());
        app.subscribe(app_observer.clone());

        let state = counter_vm.increment();
        assert_eq!(state.value, 3);
        assert_eq!(list_vm.len(), 1);

        let states = app_observer.states.lock().unwrap();
        assert!(states.iter().any(|state| matches!(state.route, Some(Route::ListDetail { .. }))));
    }

    #[test]
    fn counter_increment_without_route_does_not_touch_list() {
        let app = AppViewModel::new(0);
        let counter_vm = app.make_counter_vm();
        let list_vm = app.make_list_vm();

        let counter_observer = Arc::new(CounterObserverSink::new());
        let counter_id = counter_vm.subscribe(counter_observer.clone());
        counter_vm.increment();
        counter_vm.unsubscribe(counter_id);

        assert_eq!(counter_vm.get_state().value, 1);
        assert_eq!(list_vm.len(), 0);
    }

    #[test]
    fn app_clear_route_and_unsubscribe() {
        let app = AppViewModel::new(2);
        let app_observer = Arc::new(AppObserverSink::new());
        let app_id = app.subscribe(app_observer.clone());

        app.clear_route();
        let counter_vm = app.make_counter_vm();
        counter_vm.increment();
        assert!(app_observer
            .states
            .lock()
            .unwrap()
            .iter()
            .any(|state| state.route.is_some()));

        app.clear_route();
        assert!(app_observer
            .states
            .lock()
            .unwrap()
            .iter()
            .any(|state| state.route.is_none()));

        app.unsubscribe(app_id);
    }

    #[test]
    fn app_request_summary_requires_items() {
        let app = AppViewModel::new(0);
        let list_vm = app.make_list_vm();
        let app_observer = Arc::new(AppObserverSink::new());
        app.subscribe(app_observer.clone());

        assert!(!app.request_summary());
        list_vm.append_now();
        assert!(app.request_summary());

        assert!(app_observer
            .states
            .lock()
            .unwrap()
            .iter()
            .any(|state| matches!(state.route, Some(Route::Summary))));
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
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let _ = vm.insert_with_timestamp(0, 1_000).unwrap();
        let _ = vm.insert_with_timestamp(1, 2_000).unwrap();

        let observer = Arc::new(ListObserverSink::new());
        let list_id = vm.subscribe(observer.clone());

        let calls = observer.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].len(), 2);
        assert!(matches!(calls[0][0], ListDiff::Insert { .. }));
        if let ListDiff::Insert { index, item } = &calls[0][0] {
            assert_eq!(*index, 0);
            assert_eq!(item.timestamp_ms, 1_000);
        }
        vm.unsubscribe(list_id);
    }

    #[test]
    fn list_updates_notify_app_observer() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let app_observer = Arc::new(AppObserverSink::new());
        app.subscribe(app_observer.clone());

        vm.insert_with_timestamp(0, 1_000);
        vm.insert_with_timestamp(1, 2_000);
        assert!(vm.sort_by_timestamp_desc());
        vm.apply_diffs(Vec::new());
        vm.remove_at(0);

        let states = app_observer.states.lock().unwrap();
        assert!(states.iter().any(|state| state.list_len == 0));
    }

    #[test]
    fn list_insert_update_remove_emit_diffs() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let observer = Arc::new(ListObserverSink::new());
        let list_id = vm.subscribe(observer.clone());

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
        vm.unsubscribe(list_id);
    }

    #[test]
    fn list_move_and_sort_emit_moves() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        vm.insert_with_timestamp(0, 1_000).unwrap();
        vm.insert_with_timestamp(1, 3_000).unwrap();
        vm.insert_with_timestamp(2, 2_000).unwrap();

        let observer = Arc::new(ListObserverSink::new());
        let list_id = vm.subscribe(observer.clone());
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
        vm.unsubscribe(list_id);
    }

    #[test]
    fn list_apply_batch_diffs() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let observer = Arc::new(ListObserverSink::new());
        let list_id = vm.subscribe(observer.clone());

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
        vm.unsubscribe(list_id);
    }

    #[test]
    fn list_invalid_indices_fail() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        assert!(vm.update_with_timestamp(0, 1).is_none());
        assert!(vm.remove_at(0).is_none());
        assert!(!vm.move_item(-1, 0));
        assert!(vm.insert_with_timestamp(-1, 1).is_none());
    }

    #[test]
    fn list_append_and_insert_now_use_current_time() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let item = vm.append_now();
        assert_eq!(vm.len(), 1);
        assert!(!item.date_cn.is_empty());

        let item2 = vm.insert_now(1).unwrap();
        assert_eq!(vm.len(), 2);
        assert!(item2.timestamp_ms >= item.timestamp_ms || item2.timestamp_ms <= item.timestamp_ms);
    }

    #[test]
    fn list_move_same_index_is_noop() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        vm.insert_with_timestamp(0, 1_000).unwrap();
        assert!(vm.move_item(0, 0));
    }

    #[test]
    fn list_move_invalid_target_fails() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        vm.insert_with_timestamp(0, 1_000).unwrap();
        assert!(!vm.move_item(0, 1));
    }

    #[test]
    fn list_sort_no_change_returns_false() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        vm.insert_with_timestamp(0, 3_000).unwrap();
        vm.insert_with_timestamp(1, 2_000).unwrap();
        vm.insert_with_timestamp(2, 1_000).unwrap();
        assert!(!vm.sort_by_timestamp_desc());
    }

    #[test]
    fn list_apply_diffs_empty_is_true() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        assert!(vm.apply_diffs(Vec::new()));
    }

    #[test]
    fn list_apply_diffs_invalid_insert_fails() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let diff = ListDiff::Insert {
            index: 1,
            item: ListItem {
                id: 1,
                timestamp_ms: 1,
                date_cn: date_cn_from_timestamp_ms(1),
            },
        };
        assert!(!vm.apply_diffs(vec![diff]));
    }

    #[test]
    fn list_apply_diffs_invalid_update_fails() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let diff = ListDiff::Update {
            index: 0,
            item: ListItem {
                id: 1,
                timestamp_ms: 1,
                date_cn: date_cn_from_timestamp_ms(1),
            },
        };
        assert!(!vm.apply_diffs(vec![diff]));
    }

    #[test]
    fn list_apply_diffs_invalid_remove_fails() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let diff = ListDiff::Remove { index: 0, id: 1 };
        assert!(!vm.apply_diffs(vec![diff]));
    }

    #[test]
    fn list_apply_diffs_invalid_move_fails() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let diff = ListDiff::Move { from: 0, to: 1 };
        assert!(!vm.apply_diffs(vec![diff]));
    }

    #[test]
    fn list_apply_diffs_move_invalid_target_fails() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let item = ListItem {
            id: 1,
            timestamp_ms: 1,
            date_cn: date_cn_from_timestamp_ms(1),
        };
        assert!(vm.apply_diffs(vec![ListDiff::Insert { index: 0, item }]));
        assert!(!vm.apply_diffs(vec![ListDiff::Move { from: 0, to: 1 }]));
    }

    #[test]
    fn list_apply_diffs_move_same_index_is_noop() {
        let app = AppViewModel::new(0);
        let vm = app.make_list_vm();
        let item = ListItem {
            id: 1,
            timestamp_ms: 1,
            date_cn: date_cn_from_timestamp_ms(1),
        };
        assert!(vm.apply_diffs(vec![ListDiff::Insert { index: 0, item }]));
        assert!(vm.apply_diffs(vec![ListDiff::Move { from: 0, to: 0 }]));
    }

    #[test]
    fn date_cn_fallback_for_invalid_timestamp() {
        let date = date_cn_from_timestamp_ms(i64::MAX);
        assert_eq!(date, "1970年01月01日");
    }

    #[test]
    fn vm_metadata_includes_app_counter_and_list() {
        let app: Value = serde_json::from_str(AppViewModel::ck_vm_metadata()).unwrap();
        assert_eq!(app["vm_type"], "AppViewModel");
        assert_eq!(app["mode"], "state");
        assert_eq!(app["observer"], "AppObserver");

        let counter: Value = serde_json::from_str(CounterViewModel::ck_vm_metadata()).unwrap();
        assert_eq!(counter["vm_type"], "CounterViewModel");
        assert_eq!(counter["mode"], "state");
        assert_eq!(counter["observer"], "CounterObserver");

        let list: Value = serde_json::from_str(ListViewModel::ck_vm_metadata()).unwrap();
        assert_eq!(list["vm_type"], "ListViewModel");
        assert_eq!(list["mode"], "diff_list");
        assert_eq!(list["observer"], "ListObserver");
        assert!(list["methods"].as_array().unwrap().iter().any(|m| m["name"] == "move_item"));
    }

    #[test]
    fn action_log_records_entries() {
        let app = AppViewModel::new(1);
        let counter_vm = app.make_counter_vm();
        counter_vm.increment();
        let list_vm = app.make_list_vm();
        list_vm.append_now();

        let log = app.action_log();
        assert!(log.iter().any(|entry| entry.name == "counter_increment"));
        assert!(log.iter().any(|entry| entry.name == "list_insert_with_timestamp"));

        let state = app.get_state();
        assert!(state.list_len >= 1);
    }
}
