use std::sync::{Arc, Mutex};

pub use cross_kit::CkVmMetadata;
use cross_kit::{ObserverSet, SubscriptionId, vm_bridge};

uniffi::setup_scaffolding!();

// Form Wizard shows the preferred Cross-Kit split for validation-heavy flows:
// Rust owns the route step, field validation, derived button state, and summary;
// platform code only binds inputs and renders the resulting state.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum FormStep {
    Profile,
    Security,
    Summary,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct FormWizardState {
    pub step: FormStep,
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub name_error: Option<String>,
    pub email_error: Option<String>,
    pub password_error: Option<String>,
    pub confirm_password_error: Option<String>,
    pub can_go_back: bool,
    pub can_go_next: bool,
    pub is_complete: bool,
    pub summary: String,
}

#[uniffi::export(with_foreign)]
pub trait FormWizardObserver: Send + Sync {
    fn on_state(&self, state: FormWizardState);
}

#[derive(uniffi::Object)]
pub struct FormWizardViewModel {
    state: Mutex<FormWizardState>,
    observers: ObserverSet<dyn FormWizardObserver>,
}

// The generated bridge exposes `state` as an observable platform property and
// forwards these public methods as synchronous UI actions.
#[vm_bridge(mode = "state")]
#[uniffi::export]
impl FormWizardViewModel {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(derive_state(FormWizardState {
                step: FormStep::Profile,
                name: String::new(),
                email: String::new(),
                password: String::new(),
                confirm_password: String::new(),
                name_error: None,
                email_error: None,
                password_error: None,
                confirm_password_error: None,
                can_go_back: false,
                can_go_next: false,
                is_complete: false,
                summary: String::new(),
            })),
            observers: ObserverSet::new(),
        })
    }

    pub fn update_name(&self, name: String) {
        self.update(|state| {
            state.name = name;
            state.is_complete = false;
        });
    }

    pub fn update_email(&self, email: String) {
        self.update(|state| {
            state.email = email;
            state.is_complete = false;
        });
    }

    pub fn update_password(&self, password: String) {
        self.update(|state| {
            state.password = password;
            state.is_complete = false;
        });
    }

    pub fn update_confirm(&self, confirm: String) {
        self.update(|state| {
            state.confirm_password = confirm;
            state.is_complete = false;
        });
    }

    pub fn next(&self) {
        self.update(|state| {
            if !state.can_go_next {
                return;
            }
            match state.step {
                FormStep::Profile => state.step = FormStep::Security,
                FormStep::Security => state.step = FormStep::Summary,
                FormStep::Summary => state.is_complete = true,
            }
        });
    }

    pub fn back(&self) {
        self.update(|state| {
            state.is_complete = false;
            state.step = match state.step {
                FormStep::Profile => FormStep::Profile,
                FormStep::Security => FormStep::Profile,
                FormStep::Summary => FormStep::Security,
            };
        });
    }

    pub fn get_state(&self) -> FormWizardState {
        self.locked_state()
    }

    pub fn subscribe(&self, observer: Arc<dyn FormWizardObserver>) -> SubscriptionId {
        let state = self.locked_state();
        let subscription_id = self.observers.subscribe(observer.clone());
        // Immediate replay is part of the example contract: views can render
        // the first frame from Rust-owned derived state, not duplicated defaults.
        observer.on_state(state);
        subscription_id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.observers.unsubscribe(id);
    }
}

impl FormWizardViewModel {
    fn update(&self, mutate: impl FnOnce(&mut FormWizardState)) {
        let state = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            mutate(&mut state);
            *state = derive_state(state.clone());
            state.clone()
        };
        self.notify(state);
    }

    fn locked_state(&self) -> FormWizardState {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn notify(&self, state: FormWizardState) {
        let observers = self.observers.snapshot();
        ObserverSet::notify_snapshot(&observers, |observer| {
            observer.on_state(state.clone());
        });
    }
}

fn derive_state(mut state: FormWizardState) -> FormWizardState {
    state.name_error = validate_name(&state.name);
    state.email_error = validate_email(&state.email);
    state.password_error = validate_password(&state.password);
    state.confirm_password_error =
        validate_confirm_password(&state.password, &state.confirm_password);
    state.can_go_back = state.step != FormStep::Profile;
    state.can_go_next = match state.step {
        FormStep::Profile => state.name_error.is_none() && state.email_error.is_none(),
        FormStep::Security => {
            state.password_error.is_none() && state.confirm_password_error.is_none()
        }
        FormStep::Summary => {
            state.name_error.is_none()
                && state.email_error.is_none()
                && state.password_error.is_none()
                && state.confirm_password_error.is_none()
                && !state.is_complete
        }
    };
    state.summary = if state.name.trim().is_empty() || state.email.trim().is_empty() {
        String::new()
    } else {
        format!("{} <{}>", state.name.trim(), state.email.trim())
    };
    state
}

fn validate_name(name: &str) -> Option<String> {
    if name.trim().len() < 2 {
        Some("Name must be at least 2 characters".to_string())
    } else {
        None
    }
}

fn validate_email(email: &str) -> Option<String> {
    let email = email.trim();
    if email.is_empty() {
        return Some("Email is required".to_string());
    }
    let Some((local, domain)) = email.split_once('@') else {
        return Some("Email must be valid".to_string());
    };
    let labels = domain.split('.').collect::<Vec<_>>();
    let has_no_spaces = !email.chars().any(char::is_whitespace);
    let has_single_at = !domain.contains('@');
    let has_local_part = !local.is_empty();
    let has_valid_domain = labels.len() >= 2 && labels.iter().all(|label| !label.is_empty());
    if has_single_at && has_local_part && has_valid_domain && has_no_spaces {
        None
    } else {
        Some("Email must be valid".to_string())
    }
}

fn validate_password(password: &str) -> Option<String> {
    if password.len() < 8 {
        Some("Password must be at least 8 characters".to_string())
    } else {
        None
    }
}

fn validate_confirm_password(password: &str, confirm_password: &str) -> Option<String> {
    if confirm_password.is_empty() {
        return Some("Confirm password is required".to_string());
    }
    if password == confirm_password {
        None
    } else {
        Some("Passwords must match".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingObserver {
        states: Mutex<Vec<FormWizardState>>,
    }

    impl RecordingObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                states: Mutex::new(Vec::new()),
            })
        }

        fn states(&self) -> Vec<FormWizardState> {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl FormWizardObserver for RecordingObserver {
        fn on_state(&self, state: FormWizardState) {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(state);
        }
    }

    #[test]
    fn starts_with_profile_step_and_disabled_next() {
        let state = FormWizardViewModel::new().get_state();

        assert_eq!(state.step, FormStep::Profile);
        assert!(!state.can_go_back);
        assert!(!state.can_go_next);
        assert_eq!(
            state.name_error.as_deref(),
            Some("Name must be at least 2 characters")
        );
        assert_eq!(state.email_error.as_deref(), Some("Email is required"));
    }

    #[test]
    fn validates_profile_fields_and_allows_next() {
        let vm = FormWizardViewModel::new();

        vm.update_name("A".to_string());
        vm.update_email("bad-email".to_string());
        assert!(!vm.get_state().can_go_next);

        vm.update_name("Ada".to_string());
        vm.update_email("ada@example.com".to_string());
        let state = vm.get_state();
        assert!(state.can_go_next);
        assert_eq!(state.name_error, None);
        assert_eq!(state.email_error, None);
    }

    #[test]
    fn blocks_profile_routing_for_empty_whitespace_and_missing_domain() {
        let vm = FormWizardViewModel::new();

        vm.update_name("   ".to_string());
        vm.update_email("ada@".to_string());
        vm.next();

        let state = vm.get_state();
        assert_eq!(state.step, FormStep::Profile);
        assert!(!state.can_go_next);
        assert_eq!(
            state.name_error.as_deref(),
            Some("Name must be at least 2 characters")
        );
        assert_eq!(state.email_error.as_deref(), Some("Email must be valid"));
    }

    #[test]
    fn rejects_email_with_empty_local_part_or_empty_domain_label() {
        let vm = FormWizardViewModel::new();

        vm.update_name("Ada".to_string());
        vm.update_email("@example.com".to_string());
        assert_eq!(
            vm.get_state().email_error.as_deref(),
            Some("Email must be valid")
        );

        vm.update_email("ada@.com".to_string());
        assert_eq!(
            vm.get_state().email_error.as_deref(),
            Some("Email must be valid")
        );

        vm.update_email("ada@exa mple.com".to_string());
        assert_eq!(
            vm.get_state().email_error.as_deref(),
            Some("Email must be valid")
        );
    }

    #[test]
    fn trims_profile_fields_when_deriving_summary() {
        let vm = FormWizardViewModel::new();

        vm.update_name("  Ada Lovelace  ".to_string());
        vm.update_email("  ada@example.com  ".to_string());
        vm.next();
        vm.update_password("password1".to_string());
        vm.update_confirm("password1".to_string());
        vm.next();

        assert_eq!(vm.get_state().summary, "Ada Lovelace <ada@example.com>");
    }

    #[test]
    fn next_and_back_route_between_steps() {
        let vm = valid_profile_vm();

        vm.next();
        assert_eq!(vm.get_state().step, FormStep::Security);
        assert!(vm.get_state().can_go_back);

        vm.back();
        assert_eq!(vm.get_state().step, FormStep::Profile);
        assert!(!vm.get_state().can_go_back);
    }

    #[test]
    fn validates_password_confirmation_before_summary() {
        let vm = valid_profile_vm();
        vm.next();

        vm.update_password("password1".to_string());
        vm.update_confirm("password2".to_string());
        assert_eq!(
            vm.get_state().confirm_password_error.as_deref(),
            Some("Passwords must match")
        );
        assert!(!vm.get_state().can_go_next);

        vm.update_confirm("password1".to_string());
        assert!(vm.get_state().can_go_next);
    }

    #[test]
    fn changing_password_invalidates_existing_confirmation() {
        let vm = valid_profile_vm();
        vm.next();

        vm.update_password("password1".to_string());
        vm.update_confirm("password1".to_string());
        assert!(vm.get_state().can_go_next);

        vm.update_password("password2".to_string());
        let state = vm.get_state();
        assert!(!state.can_go_next);
        assert_eq!(
            state.confirm_password_error.as_deref(),
            Some("Passwords must match")
        );

        vm.update_confirm("password2".to_string());
        assert!(vm.get_state().can_go_next);
    }

    #[test]
    fn back_preserves_fields_and_recomputes_navigation() {
        let vm = valid_profile_vm();
        vm.next();
        vm.update_password("password1".to_string());
        vm.update_confirm("password1".to_string());

        vm.back();
        let state = vm.get_state();
        assert_eq!(state.step, FormStep::Profile);
        assert_eq!(state.name, "Ada Lovelace");
        assert_eq!(state.email, "ada@example.com");
        assert_eq!(state.password, "password1");
        assert_eq!(state.confirm_password, "password1");
        assert!(state.can_go_next);
        assert!(!state.can_go_back);
    }

    #[test]
    fn summary_and_completion_are_derived_from_valid_state() {
        let vm = valid_completed_vm();

        let state = vm.get_state();
        assert_eq!(state.step, FormStep::Summary);
        assert_eq!(state.summary, "Ada Lovelace <ada@example.com>");
        assert!(state.can_go_next);
        assert!(!state.is_complete);

        vm.next();
        let state = vm.get_state();
        assert!(state.is_complete);
        assert!(!state.can_go_next);
    }

    #[test]
    fn changing_input_after_completion_clears_completion() {
        let vm = valid_completed_vm();
        vm.next();
        assert!(vm.get_state().is_complete);

        vm.update_email("ada@invalid".to_string());
        let state = vm.get_state();
        assert!(!state.is_complete);
        assert_eq!(state.email_error.as_deref(), Some("Email must be valid"));
        assert!(!state.can_go_next);
    }

    #[test]
    fn subscription_immediately_receives_current_state_and_unsubscribe_stops_updates() {
        let vm = valid_profile_vm();
        let observer = RecordingObserver::new();

        let subscription_id = vm.subscribe(observer.clone());
        vm.update_name("Grace".to_string());
        vm.unsubscribe(subscription_id);
        vm.update_name("Katherine".to_string());

        let states = observer.states();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].name, "Ada Lovelace");
        assert_eq!(states[1].name, "Grace");
    }

    fn valid_profile_vm() -> Arc<FormWizardViewModel> {
        let vm = FormWizardViewModel::new();
        vm.update_name("Ada Lovelace".to_string());
        vm.update_email("ada@example.com".to_string());
        vm
    }

    fn valid_completed_vm() -> Arc<FormWizardViewModel> {
        let vm = valid_profile_vm();
        vm.next();
        vm.update_password("password1".to_string());
        vm.update_confirm("password1".to_string());
        vm.next();
        vm
    }
}
