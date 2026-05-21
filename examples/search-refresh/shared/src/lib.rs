use std::sync::Arc;

#[cfg(test)]
use std::sync::Mutex;

pub use cross_kit::CkVmMetadata;
use cross_kit::{StateStore, SubscriptionId, vm_bridge};

uniffi::setup_scaffolding!();

// Search Refresh demonstrates long-running work without exposing async APIs
// through Cross-Kit. UI calls synchronous actions (`submit`, `tick`, `cancel`)
// and observes state fields such as loading/progress/results/error.
const TICK_PROGRESS: i64 = 50;

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub rank: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum SearchError {
    EmptyQuery,
    Network { code: i64, message: String },
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SearchState {
    pub query: String,
    pub is_loading: bool,
    pub progress: i64,
    pub results: Vec<SearchResult>,
    pub error: Option<SearchError>,
    pub can_submit: bool,
    pub can_cancel: bool,
}

#[uniffi::export(with_foreign)]
pub trait SearchObserver: Send + Sync {
    fn on_state(&self, state: SearchState);
}

#[derive(Clone, Debug)]
struct ActiveSearch {
    id: i64,
    query: String,
}

#[derive(Clone, Debug)]
struct StoreState {
    query: String,
    is_loading: bool,
    progress: i64,
    results: Vec<SearchResult>,
    error: Option<SearchError>,
    next_request_id: i64,
    active: Option<ActiveSearch>,
}

#[derive(uniffi::Object)]
pub struct SearchViewModel {
    state: StateStore<StoreState, dyn SearchObserver>,
}

// State mode intentionally keeps the platform surface small: one observable
// state object plus action methods. Failure is modeled as typed state, not as a
// Swift `throw` or Kotlin `suspend` result.
#[vm_bridge(mode = "state")]
#[uniffi::export]
impl SearchViewModel {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: StateStore::new(StoreState::new()),
        })
    }

    pub fn update_query(&self, query: String) {
        self.mutate_notify(|state| {
            // Editing the query invalidates the in-flight request. This mirrors
            // how a real search token/cancellation check would prevent stale
            // results from winning after the UI has moved on.
            state.query = query;
            state.is_loading = false;
            state.progress = 0;
            state.results.clear();
            state.error = None;
            state.active = None;
            state.to_search_state()
        });
    }

    pub fn submit(&self) {
        self.mutate_notify(|state| {
            let query = state.query.trim().to_string();
            if query.is_empty() {
                state.is_loading = false;
                state.progress = 0;
                state.results.clear();
                state.error = Some(SearchError::EmptyQuery);
                state.active = None;
                return state.to_search_state();
            }

            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.query = query.clone();
            state.is_loading = true;
            state.progress = 0;
            state.results.clear();
            state.error = None;
            state.active = Some(ActiveSearch {
                id: request_id,
                query,
            });
            state.to_search_state()
        });
    }

    pub fn tick(&self) {
        self.mutate_optional_notify(|state| {
            let Some(active) = state.active.clone() else {
                return None;
            };
            state.progress = (state.progress + TICK_PROGRESS).min(100);
            if state.progress < 100 {
                Some(state.to_search_state())
            } else {
                complete_active(state, active.id, &active.query);
                Some(state.to_search_state())
            }
        });
    }

    pub fn cancel(&self) {
        self.mutate_optional_notify(|state| {
            state.active.as_ref()?;
            state.is_loading = false;
            state.progress = 0;
            state.error = Some(SearchError::Cancelled);
            state.active = None;
            Some(state.to_search_state())
        });
    }

    pub fn get_state(&self) -> SearchState {
        self.state.read_with(StoreState::to_search_state)
    }

    pub fn subscribe(&self, observer: Arc<dyn SearchObserver>) -> SubscriptionId {
        // Generated root containers rely on this replay to show a consistent
        // idle/loading/error state as soon as the bridge is created.
        self.state.subscribe_replay_with(
            observer,
            StoreState::to_search_state,
            |observer, state| observer.on_state(state),
        )
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.state.unsubscribe(id);
    }
}

impl SearchViewModel {
    fn mutate_notify(&self, mutate: impl FnOnce(&mut StoreState) -> SearchState) -> SearchState {
        self.state
            .update_with_notify(mutate, |observer, state| observer.on_state(state))
    }

    fn mutate_optional_notify(
        &self,
        mutate: impl FnOnce(&mut StoreState) -> Option<SearchState>,
    ) -> Option<SearchState> {
        self.state
            .try_update_with_notify(mutate, |observer, state| observer.on_state(state))
    }

    #[cfg(test)]
    fn active_request_id_for_tests(&self) -> Option<i64> {
        self.state
            .read_with(|state| state.active.as_ref().map(|active| active.id))
    }

    #[cfg(test)]
    fn complete_request_for_tests(&self, request_id: i64) {
        self.mutate_optional_notify(|state| {
            let Some(active) = state.active.clone() else {
                return None;
            };
            complete_active(state, request_id, &active.query);
            Some(state.to_search_state())
        });
    }
}

impl StoreState {
    fn new() -> Self {
        Self {
            query: String::new(),
            is_loading: false,
            progress: 0,
            results: Vec::new(),
            error: None,
            next_request_id: 1,
            active: None,
        }
    }

    fn to_search_state(&self) -> SearchState {
        SearchState {
            query: self.query.clone(),
            is_loading: self.is_loading,
            progress: self.progress,
            results: self.results.clone(),
            error: self.error.clone(),
            can_submit: !self.is_loading && !self.query.trim().is_empty(),
            can_cancel: self.is_loading,
        }
    }
}

fn complete_active(state: &mut StoreState, request_id: i64, query: &str) {
    if state.active.as_ref().map(|active| active.id) != Some(request_id) {
        return;
    }

    state.is_loading = false;
    state.progress = 100;
    state.active = None;
    if query.eq_ignore_ascii_case("network") || query.eq_ignore_ascii_case("fail") {
        state.results.clear();
        state.error = Some(SearchError::Network {
            code: 503,
            message: "temporary search failure".to_string(),
        });
    } else {
        state.error = None;
        state.results = search_results(query);
    }
}

fn search_results(query: &str) -> Vec<SearchResult> {
    vec![
        SearchResult {
            title: format!("{query} guide"),
            snippet: "Overview and setup notes".to_string(),
            rank: 1,
        },
        SearchResult {
            title: format!("{query} integration"),
            snippet: "Platform bridge usage details".to_string(),
            rank: 2,
        },
        SearchResult {
            title: format!("{query} troubleshooting"),
            snippet: "Common validation and retry cases".to_string(),
            rank: 3,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingObserver {
        states: Mutex<Vec<SearchState>>,
    }

    impl RecordingObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                states: Mutex::new(Vec::new()),
            })
        }

        fn states(&self) -> Vec<SearchState> {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl SearchObserver for RecordingObserver {
        fn on_state(&self, state: SearchState) {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(state);
        }
    }

    #[test]
    fn starts_idle_with_empty_query() {
        let vm = SearchViewModel::new();

        let state = vm.get_state();
        assert_eq!(state.query, "");
        assert!(!state.is_loading);
        assert_eq!(state.progress, 0);
        assert!(state.results.is_empty());
        assert_eq!(state.error, None);
        assert!(!state.can_submit);
        assert!(!state.can_cancel);
    }

    #[test]
    fn update_query_sets_submit_availability_without_platform_rules() {
        let vm = SearchViewModel::new();

        vm.update_query("  bridge  ".to_string());

        let state = vm.get_state();
        assert_eq!(state.query, "  bridge  ");
        assert!(state.can_submit);
        assert!(!state.can_cancel);
    }

    #[test]
    fn empty_submit_reports_typed_error_without_loading() {
        let vm = SearchViewModel::new();

        vm.update_query("  ".to_string());
        vm.submit();

        let state = vm.get_state();
        assert_eq!(state.error, Some(SearchError::EmptyQuery));
        assert!(!state.is_loading);
        assert_eq!(state.progress, 0);
        assert!(state.results.is_empty());
    }

    #[test]
    fn submit_enters_loading_and_tick_completes_success() {
        let vm = SearchViewModel::new();

        vm.update_query("rust".to_string());
        vm.submit();
        let state = vm.get_state();
        assert!(state.is_loading);
        assert_eq!(state.progress, 0);
        assert!(state.can_cancel);

        vm.tick();
        let state = vm.get_state();
        assert!(state.is_loading);
        assert_eq!(state.progress, 50);
        assert!(state.results.is_empty());

        vm.tick();
        let state = vm.get_state();
        assert!(!state.is_loading);
        assert_eq!(state.progress, 100);
        assert_eq!(state.results.len(), 3);
        assert_eq!(state.results[0].title, "rust guide");
        assert_eq!(state.error, None);
        assert!(state.can_submit);
        assert!(!state.can_cancel);
    }

    #[test]
    fn progress_notifications_are_monotonic_for_active_operation() {
        let vm = SearchViewModel::new();
        let observer = RecordingObserver::new();
        vm.subscribe(observer.clone());

        vm.update_query("rust".to_string());
        vm.submit();
        vm.tick();
        vm.tick();

        let progress = observer
            .states()
            .into_iter()
            .filter(|state| state.query == "rust")
            .map(|state| state.progress)
            .collect::<Vec<_>>();
        assert_eq!(progress, vec![0, 0, 50, 100]);
    }

    #[test]
    fn network_failure_preserves_structured_error_fields() {
        let vm = SearchViewModel::new();

        vm.update_query("network".to_string());
        vm.submit();
        vm.tick();
        vm.tick();

        let state = vm.get_state();
        assert_eq!(
            state.error,
            Some(SearchError::Network {
                code: 503,
                message: "temporary search failure".to_string(),
            })
        );
        assert!(!state.is_loading);
        assert!(state.results.is_empty());
        assert!(state.can_submit);
    }

    #[test]
    fn cancel_without_active_search_is_noop() {
        let vm = SearchViewModel::new();
        vm.update_query("rust".to_string());
        let before = vm.get_state();

        vm.cancel();

        let after = vm.get_state();
        assert_eq!(after, before);
        assert_eq!(after.error, None);
        assert!(after.can_submit);
        assert!(!after.can_cancel);
    }

    #[test]
    fn noop_tick_and_cancel_do_not_notify_observers() {
        let vm = SearchViewModel::new();
        let observer = RecordingObserver::new();
        vm.subscribe(observer.clone());

        vm.tick();
        vm.cancel();

        assert_eq!(observer.states().len(), 1);
    }

    #[test]
    fn cancel_sets_cancelled_state_and_ignores_stale_completion() {
        let vm = SearchViewModel::new();

        vm.update_query("rust".to_string());
        vm.submit();
        let request_id = vm.active_request_id_for_tests().unwrap();
        vm.cancel();
        vm.complete_request_for_tests(request_id);

        let state = vm.get_state();
        assert_eq!(state.error, Some(SearchError::Cancelled));
        assert!(!state.is_loading);
        assert_eq!(state.progress, 0);
        assert!(state.results.is_empty());
    }

    #[test]
    fn consecutive_submit_ignores_old_result_for_new_query() {
        let vm = SearchViewModel::new();

        vm.update_query("first".to_string());
        vm.submit();
        let first_request_id = vm.active_request_id_for_tests().unwrap();
        vm.update_query("second".to_string());
        vm.submit();
        vm.complete_request_for_tests(first_request_id);

        let state = vm.get_state();
        assert_eq!(state.query, "second");
        assert!(state.is_loading);
        assert!(state.results.is_empty());

        vm.tick();
        vm.tick();
        let state = vm.get_state();
        assert_eq!(state.results[0].title, "second guide");
    }

    #[test]
    fn editing_query_during_loading_invalidates_inflight_search() {
        let vm = SearchViewModel::new();

        vm.update_query("old".to_string());
        vm.submit();
        let request_id = vm.active_request_id_for_tests().unwrap();
        vm.update_query("new".to_string());
        vm.complete_request_for_tests(request_id);

        let state = vm.get_state();
        assert_eq!(state.query, "new");
        assert!(!state.is_loading);
        assert_eq!(state.progress, 0);
        assert!(state.results.is_empty());
        assert_eq!(state.error, None);
        assert!(state.can_submit);
    }

    #[test]
    fn editing_query_after_success_clears_stale_results() {
        let vm = SearchViewModel::new();

        vm.update_query("old".to_string());
        vm.submit();
        vm.tick();
        vm.tick();
        assert_eq!(vm.get_state().results[0].title, "old guide");

        vm.update_query("new".to_string());

        let state = vm.get_state();
        assert_eq!(state.query, "new");
        assert!(state.results.is_empty());
        assert_eq!(state.error, None);
        assert!(state.can_submit);
    }

    #[test]
    fn subscription_replays_current_state_and_unsubscribe_stops_updates() {
        let vm = SearchViewModel::new();
        vm.update_query("rust".to_string());
        let observer = RecordingObserver::new();

        let id = vm.subscribe(observer.clone());
        vm.submit();
        vm.unsubscribe(id);
        vm.tick();

        let states = observer.states();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].query, "rust");
        assert!(!states[0].is_loading);
        assert!(states[1].is_loading);
    }
}
