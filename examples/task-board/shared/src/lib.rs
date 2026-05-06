use std::sync::{Arc, Mutex};

pub use cross_kit::CkVmMetadata;
use cross_kit::{ObserverSet, SubscriptionId, vm_bridge};

uniffi::setup_scaffolding!();

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum TaskFilter {
    All,
    Open,
    Done,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TaskItem {
    pub id: i64,
    pub title: String,
    pub done: bool,
    pub position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum TaskDiff {
    Insert { index: i64, item: TaskItem },
    Update { index: i64, item: TaskItem },
    Remove { index: i64, id: i64 },
    Move { from: i64, to: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TaskBoardState {
    pub filter: TaskFilter,
    pub total_count: i64,
    pub open_count: i64,
    pub done_count: i64,
    pub visible_count: i64,
    pub can_clear_done: bool,
    pub last_error: Option<String>,
}

#[uniffi::export(with_foreign)]
pub trait TaskBoardObserver: Send + Sync {
    fn on_state(&self, state: TaskBoardState);
}

#[uniffi::export(with_foreign)]
pub trait TaskListObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<TaskDiff>);
}

#[derive(Clone)]
struct Store {
    inner: Arc<Mutex<StoreInner>>,
    board_observers: ObserverSet<dyn TaskBoardObserver>,
    list_observers: ObserverSet<dyn TaskListObserver>,
}

struct StoreInner {
    tasks: Vec<TaskItem>,
    filter: TaskFilter,
    next_id: i64,
    last_error: Option<String>,
}

#[derive(uniffi::Object)]
pub struct TaskBoardViewModel {
    store: Store,
}

#[derive(uniffi::Object)]
pub struct TaskListViewModel {
    store: Store,
}

#[vm_bridge(mode = "state")]
#[uniffi::export]
impl TaskBoardViewModel {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Store::new(),
        })
    }

    pub fn make_task_list_vm(self: Arc<Self>) -> Arc<TaskListViewModel> {
        Arc::new(TaskListViewModel {
            store: self.store.clone(),
        })
    }

    pub fn set_filter(&self, filter: TaskFilter) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            inner.filter = filter;
            let new_visible = visible_tasks(inner);
            (replace_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn get_state(&self) -> TaskBoardState {
        self.store.state()
    }

    pub fn subscribe(&self, observer: Arc<dyn TaskBoardObserver>) -> SubscriptionId {
        let state = self.get_state();
        let id = self.store.board_observers.subscribe(observer.clone());
        observer.on_state(state);
        id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.store.board_observers.unsubscribe(id);
    }
}

#[vm_bridge(
    mode = "diff_list",
    diff = TaskDiff,
    item = TaskItem,
    factory = TaskBoardViewModel::make_task_list_vm
)]
#[uniffi::export]
impl TaskListViewModel {
    pub fn add_task(&self, title: String) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            let title = title.trim().to_string();
            if title.is_empty() {
                inner.last_error = Some("Task title is required".to_string());
                return (Vec::new(), true);
            }
            inner.last_error = None;
            let id = inner.next_id;
            inner.next_id += 1;
            inner.tasks.push(TaskItem {
                id,
                title,
                done: false,
                position: inner.tasks.len() as i64,
            });
            normalize_positions(&mut inner.tasks);
            let new_visible = visible_tasks(inner);
            (replace_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn toggle_done(&self, id: i64) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            match inner.tasks.iter_mut().find(|task| task.id == id) {
                Some(task) => {
                    task.done = !task.done;
                    inner.last_error = None;
                }
                None => inner.last_error = Some("Task not found".to_string()),
            }
            let new_visible = visible_tasks(inner);
            (changed_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn rename_task(&self, id: i64, title: String) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            let title = title.trim().to_string();
            if title.is_empty() {
                inner.last_error = Some("Task title is required".to_string());
                return (Vec::new(), true);
            }
            match inner.tasks.iter_mut().find(|task| task.id == id) {
                Some(task) => {
                    task.title = title;
                    inner.last_error = None;
                }
                None => inner.last_error = Some("Task not found".to_string()),
            }
            let new_visible = visible_tasks(inner);
            (changed_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn delete_task(&self, id: i64) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            let original_len = inner.tasks.len();
            inner.tasks.retain(|task| task.id != id);
            if inner.tasks.len() == original_len {
                inner.last_error = Some("Task not found".to_string());
            } else {
                inner.last_error = None;
            }
            normalize_positions(&mut inner.tasks);
            let new_visible = visible_tasks(inner);
            (replace_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn move_visible(&self, from: i64, to: i64) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            let len = old_visible.len() as i64;
            if from < 0 || to < 0 || from >= len || to >= len {
                inner.last_error = Some("Move index is out of range".to_string());
                return (Vec::new(), true);
            }
            if from == to {
                inner.last_error = None;
                return (Vec::new(), true);
            }
            reorder_visible_tasks(&mut inner.tasks, &old_visible, from as usize, to as usize);
            normalize_positions(&mut inner.tasks);
            inner.last_error = None;
            (vec![TaskDiff::Move { from, to }], true)
        });
    }

    pub fn clear_done(&self) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            inner.tasks.retain(|task| !task.done);
            normalize_positions(&mut inner.tasks);
            inner.last_error = None;
            let new_visible = visible_tasks(inner);
            (replace_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn add_sample_batch(&self) {
        self.store.mutate(|inner| {
            let old_visible = visible_tasks(inner);
            for title in ["Plan", "Build", "Review"] {
                let id = inner.next_id;
                inner.next_id += 1;
                inner.tasks.push(TaskItem {
                    id,
                    title: title.to_string(),
                    done: false,
                    position: inner.tasks.len() as i64,
                });
            }
            normalize_positions(&mut inner.tasks);
            inner.last_error = None;
            let new_visible = visible_tasks(inner);
            (replace_visible_diffs(&old_visible, &new_visible), true)
        });
    }

    pub fn subscribe(&self, observer: Arc<dyn TaskListObserver>) -> SubscriptionId {
        let visible = self.store.visible_tasks();
        let id = self.store.list_observers.subscribe(observer.clone());
        if !visible.is_empty() {
            observer.on_diffs(
                visible
                    .into_iter()
                    .enumerate()
                    .map(|(index, item)| TaskDiff::Insert {
                        index: index as i64,
                        item,
                    })
                    .collect(),
            );
        }
        id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.store.list_observers.unsubscribe(id);
    }
}

impl Store {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StoreInner {
                tasks: Vec::new(),
                filter: TaskFilter::All,
                next_id: 1,
                last_error: None,
            })),
            board_observers: ObserverSet::new(),
            list_observers: ObserverSet::new(),
        }
    }

    fn state(&self) -> TaskBoardState {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        board_state(&inner)
    }

    fn visible_tasks(&self) -> Vec<TaskItem> {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        visible_tasks(&inner)
    }

    fn mutate(&self, mutate: impl FnOnce(&mut StoreInner) -> (Vec<TaskDiff>, bool)) {
        let (state, diffs, notify_state) = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let (diffs, notify_state) = mutate(&mut inner);
            (board_state(&inner), diffs, notify_state)
        };
        if notify_state {
            let observers = self.board_observers.snapshot();
            ObserverSet::notify_snapshot(&observers, |observer| observer.on_state(state.clone()));
        }
        if !diffs.is_empty() {
            let observers = self.list_observers.snapshot();
            ObserverSet::notify_snapshot(&observers, |observer| observer.on_diffs(diffs.clone()));
        }
    }
}

fn board_state(inner: &StoreInner) -> TaskBoardState {
    let done_count = inner.tasks.iter().filter(|task| task.done).count() as i64;
    let total_count = inner.tasks.len() as i64;
    TaskBoardState {
        filter: inner.filter.clone(),
        total_count,
        open_count: total_count - done_count,
        done_count,
        visible_count: visible_tasks(inner).len() as i64,
        can_clear_done: done_count > 0,
        last_error: inner.last_error.clone(),
    }
}

fn visible_tasks(inner: &StoreInner) -> Vec<TaskItem> {
    inner
        .tasks
        .iter()
        .filter(|task| match inner.filter {
            TaskFilter::All => true,
            TaskFilter::Open => !task.done,
            TaskFilter::Done => task.done,
        })
        .cloned()
        .collect()
}

fn replace_visible_diffs(old: &[TaskItem], new: &[TaskItem]) -> Vec<TaskDiff> {
    let mut diffs = Vec::new();
    for (index, item) in old.iter().enumerate().rev() {
        diffs.push(TaskDiff::Remove {
            index: index as i64,
            id: item.id,
        });
    }
    for (index, item) in new.iter().cloned().enumerate() {
        diffs.push(TaskDiff::Insert {
            index: index as i64,
            item,
        });
    }
    diffs
}

fn changed_visible_diffs(old: &[TaskItem], new: &[TaskItem]) -> Vec<TaskDiff> {
    if old.len() == new.len()
        && old.iter().zip(new).all(|(old, new)| old.id == new.id)
        && old.iter().zip(new).filter(|(old, new)| old != new).count() == 1
    {
        let (index, item) = new
            .iter()
            .enumerate()
            .find(|(index, item)| old[*index] != **item)
            .expect("single changed item");
        return vec![TaskDiff::Update {
            index: index as i64,
            item: item.clone(),
        }];
    }
    replace_visible_diffs(old, new)
}

fn reorder_visible_tasks(tasks: &mut [TaskItem], old_visible: &[TaskItem], from: usize, to: usize) {
    let mut visible_ids: Vec<i64> = old_visible.iter().map(|task| task.id).collect();
    let moved_id = visible_ids.remove(from);
    visible_ids.insert(to, moved_id);

    let reordered_visible: Vec<TaskItem> = visible_ids
        .into_iter()
        .filter_map(|id| tasks.iter().find(|task| task.id == id).cloned())
        .collect();
    let old_visible_ids: Vec<i64> = old_visible.iter().map(|task| task.id).collect();
    let mut reordered = reordered_visible.into_iter();
    for task in tasks.iter_mut() {
        if old_visible_ids.contains(&task.id) {
            *task = reordered
                .next()
                .expect("visible replacement count should match old visible count");
        }
    }
}

fn normalize_positions(tasks: &mut [TaskItem]) {
    for (index, task) in tasks.iter_mut().enumerate() {
        task.position = index as i64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingBoardObserver {
        states: Mutex<Vec<TaskBoardState>>,
    }

    struct RecordingListObserver {
        diffs: Mutex<Vec<Vec<TaskDiff>>>,
    }

    impl RecordingBoardObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                states: Mutex::new(Vec::new()),
            })
        }

        fn states(&self) -> Vec<TaskBoardState> {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl RecordingListObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                diffs: Mutex::new(Vec::new()),
            })
        }

        fn diffs(&self) -> Vec<Vec<TaskDiff>> {
            self.diffs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl TaskBoardObserver for RecordingBoardObserver {
        fn on_state(&self, state: TaskBoardState) {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(state);
        }
    }

    impl TaskListObserver for RecordingListObserver {
        fn on_diffs(&self, diffs: Vec<TaskDiff>) {
            self.diffs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(diffs);
        }
    }

    #[test]
    fn starts_empty_with_all_filter() {
        let board = TaskBoardViewModel::new();
        let state = board.get_state();

        assert_eq!(state.filter, TaskFilter::All);
        assert_eq!(state.total_count, 0);
        assert_eq!(state.open_count, 0);
        assert_eq!(state.done_count, 0);
        assert_eq!(state.visible_count, 0);
        assert!(!state.can_clear_done);
    }

    #[test]
    fn add_task_trims_title_and_emits_insert() {
        let (board, list, observer) = subscribed_board();

        list.add_task("  Write tests  ".to_string());

        let state = board.get_state();
        assert_eq!(state.total_count, 1);
        assert_eq!(state.open_count, 1);
        assert_eq!(state.last_error, None);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![TaskDiff::Insert {
                index: 0,
                item: TaskItem {
                    id: 1,
                    title: "Write tests".to_string(),
                    done: false,
                    position: 0,
                },
            }]
        );
    }

    #[test]
    fn empty_title_sets_error_without_list_diff() {
        let (board, list, observer) = subscribed_board();

        list.add_task(" ".to_string());

        assert_eq!(
            board.get_state().last_error.as_deref(),
            Some("Task title is required")
        );
        assert!(observer.diffs().is_empty());
    }

    #[test]
    fn toggle_updates_counters_and_filter_visibility() {
        let (board, list, observer) = subscribed_board();
        list.add_task("One".to_string());
        board.set_filter(TaskFilter::Open);
        list.toggle_done(1);

        let state = board.get_state();
        assert_eq!(state.open_count, 0);
        assert_eq!(state.done_count, 1);
        assert_eq!(state.visible_count, 0);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![TaskDiff::Remove { index: 0, id: 1 }]
        );
    }

    #[test]
    fn toggle_in_all_filter_emits_update_diff() {
        let (_board, list, observer) = subscribed_board();
        list.add_task("One".to_string());

        list.toggle_done(1);

        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![TaskDiff::Update {
                index: 0,
                item: TaskItem {
                    id: 1,
                    title: "One".to_string(),
                    done: true,
                    position: 0,
                },
            }]
        );
    }

    #[test]
    fn rename_task_trims_title_and_emits_update_diff() {
        let (board, list, observer) = subscribed_board();
        list.add_task("One".to_string());

        list.rename_task(1, "  Renamed  ".to_string());

        assert_eq!(board.get_state().last_error, None);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![TaskDiff::Update {
                index: 0,
                item: TaskItem {
                    id: 1,
                    title: "Renamed".to_string(),
                    done: false,
                    position: 0,
                },
            }]
        );
    }

    #[test]
    fn rename_rejects_empty_title_without_list_diff() {
        let (board, list, observer) = subscribed_board();
        list.add_task("One".to_string());
        let before = observer.diffs().len();

        list.rename_task(1, " ".to_string());

        assert_eq!(
            board.get_state().last_error.as_deref(),
            Some("Task title is required")
        );
        assert_eq!(observer.diffs().len(), before);
    }

    #[test]
    fn done_filter_shows_toggled_items() {
        let (board, list, observer) = subscribed_board();
        list.add_task("One".to_string());
        list.toggle_done(1);
        board.set_filter(TaskFilter::Done);

        assert_eq!(board.get_state().visible_count, 1);
        assert!(matches!(
            observer.diffs().last().unwrap().as_slice(),
            [TaskDiff::Remove { .. }, TaskDiff::Insert { .. }]
        ));
    }

    #[test]
    fn delete_task_removes_and_recomputes_positions() {
        let (board, list, observer) = subscribed_board();
        list.add_sample_batch();
        list.delete_task(2);

        let state = board.get_state();
        assert_eq!(state.total_count, 2);
        assert_eq!(state.open_count, 2);
        assert!(
            observer
                .diffs()
                .last()
                .unwrap()
                .iter()
                .any(|diff| matches!(diff, TaskDiff::Remove { id: 2, .. }))
        );
    }

    #[test]
    fn move_visible_emits_move_and_preserves_count() {
        let (board, list, observer) = subscribed_board();
        list.add_sample_batch();
        list.move_visible(2, 0);

        assert_eq!(board.get_state().total_count, 3);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![TaskDiff::Move { from: 2, to: 0 }]
        );
    }

    #[test]
    fn move_visible_first_to_last_reorders_subscription_snapshot() {
        let board = TaskBoardViewModel::new();
        let list = board.clone().make_task_list_vm();
        list.add_sample_batch();

        list.move_visible(0, 2);

        let observer = RecordingListObserver::new();
        list.subscribe(observer.clone());
        let replay = observer.diffs().pop().unwrap();
        let titles: Vec<String> = replay
            .into_iter()
            .filter_map(|diff| match diff {
                TaskDiff::Insert { item, .. } => Some(item.title),
                _ => None,
            })
            .collect();
        assert_eq!(titles, vec!["Build", "Review", "Plan"]);
    }

    #[test]
    fn move_visible_under_open_filter_preserves_hidden_done_slots() {
        let board = TaskBoardViewModel::new();
        let list = board.clone().make_task_list_vm();
        list.add_sample_batch();
        list.toggle_done(2);
        board.set_filter(TaskFilter::Open);

        list.move_visible(0, 1);

        board.set_filter(TaskFilter::All);
        let observer = RecordingListObserver::new();
        list.subscribe(observer.clone());
        let replay = observer.diffs().pop().unwrap();
        let titles: Vec<String> = replay
            .into_iter()
            .filter_map(|diff| match diff {
                TaskDiff::Insert { item, .. } => Some(item.title),
                _ => None,
            })
            .collect();
        assert_eq!(titles, vec!["Review", "Build", "Plan"]);
    }

    #[test]
    fn move_invalid_index_sets_error_without_diff() {
        let (board, list, observer) = subscribed_board();
        list.add_task("One".to_string());
        let before = observer.diffs().len();

        list.move_visible(2, 0);

        assert_eq!(
            board.get_state().last_error.as_deref(),
            Some("Move index is out of range")
        );
        assert_eq!(observer.diffs().len(), before);
    }

    #[test]
    fn clear_done_removes_done_tasks_and_updates_state() {
        let (board, list, _observer) = subscribed_board();
        list.add_sample_batch();
        list.toggle_done(1);
        list.toggle_done(3);

        list.clear_done();

        let state = board.get_state();
        assert_eq!(state.total_count, 1);
        assert_eq!(state.done_count, 0);
        assert!(!state.can_clear_done);
    }

    #[test]
    fn subscribe_replays_visible_tasks_as_inserts() {
        let board = TaskBoardViewModel::new();
        let list = board.clone().make_task_list_vm();
        list.add_sample_batch();
        board.set_filter(TaskFilter::Open);
        let observer = RecordingListObserver::new();

        list.subscribe(observer.clone());

        let diffs = observer.diffs();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].len(), 3);
        assert!(matches!(diffs[0][0], TaskDiff::Insert { index: 0, .. }));
    }

    #[test]
    fn board_subscribe_pushes_current_state_and_unsubscribe_stops_updates() {
        let board = TaskBoardViewModel::new();
        let list = board.clone().make_task_list_vm();
        let observer = RecordingBoardObserver::new();

        let id = board.subscribe(observer.clone());
        list.add_task("One".to_string());
        board.unsubscribe(id);
        list.add_task("Two".to_string());

        let states = observer.states();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].total_count, 0);
        assert_eq!(states[1].total_count, 1);
    }

    fn subscribed_board() -> (
        Arc<TaskBoardViewModel>,
        Arc<TaskListViewModel>,
        Arc<RecordingListObserver>,
    ) {
        let board = TaskBoardViewModel::new();
        let list = board.clone().make_task_list_vm();
        let observer = RecordingListObserver::new();
        list.subscribe(observer.clone());
        (board, list, observer)
    }
}
