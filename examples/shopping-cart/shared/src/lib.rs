use std::sync::{Arc, Mutex};

pub use cross_kit::CkVmMetadata;
use cross_kit::{ObserverSet, SubscriptionId, vm_bridge};

uniffi::setup_scaffolding!();

// Shopping Cart is a business-rule example. Rust owns catalog stock checks,
// coupon validation, totals, tax, and cart diffs; platform code should mostly
// render state and invoke intent-like methods.
const TAX_BASIS_POINTS: i64 = 825;
const SAVE10: &str = "SAVE10";

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
    pub stock: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct CartItem {
    pub product_id: i64,
    pub name: String,
    pub unit_price_cents: i64,
    pub quantity: i64,
    pub line_total_cents: i64,
    pub position: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CartDiff {
    Insert { index: i64, item: CartItem },
    Update { index: i64, item: CartItem },
    Remove { index: i64, product_id: i64 },
    Move { from: i64, to: i64 },
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum CartError {
    QuantityMustBePositive,
    ProductNotFound {
        product_id: i64,
    },
    OutOfStock {
        product_id: i64,
        requested: i64,
        available: i64,
    },
    InvalidCoupon {
        code: String,
    },
    CartEmpty,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ShoppingCartState {
    pub products: Vec<Product>,
    pub coupon_code: Option<String>,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
    pub item_count: i64,
    pub checkout_ready: bool,
    pub last_error: Option<CartError>,
}

#[uniffi::export(with_foreign)]
pub trait ShoppingCartObserver: Send + Sync {
    fn on_state(&self, state: ShoppingCartState);
}

#[uniffi::export(with_foreign)]
pub trait CartObserver: Send + Sync {
    fn on_diffs(&self, diffs: Vec<CartDiff>);
}

#[derive(Clone)]
struct Store {
    // The cart list and summary state are derived from the same locked store,
    // but notifications are emitted through separate observer sets so generated
    // bridges can update `kit.cart.items` and `kit.shoppingCart.state`.
    inner: Arc<Mutex<StoreInner>>,
    cart_observers: ObserverSet<dyn CartObserver>,
    state_observers: ObserverSet<dyn ShoppingCartObserver>,
}

struct StoreInner {
    products: Vec<Product>,
    cart: Vec<CartItem>,
    coupon_code: Option<String>,
    last_error: Option<CartError>,
}

#[derive(uniffi::Object)]
pub struct ShoppingCartViewModel {
    store: Store,
}

#[derive(uniffi::Object)]
pub struct CartViewModel {
    store: Store,
}

// Root state VM: owns summary-level actions and creates the child cart VM used
// by the diff-list bridge.
#[vm_bridge(mode = "state")]
#[uniffi::export]
impl ShoppingCartViewModel {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Store::new(),
        })
    }

    pub fn make_cart_vm(self: Arc<Self>) -> Arc<CartViewModel> {
        Arc::new(CartViewModel {
            store: self.store.clone(),
        })
    }

    pub fn apply_coupon(&self, code: String) {
        self.store.mutate(|inner| {
            let code = code.trim().to_ascii_uppercase();
            if code == SAVE10 {
                inner.coupon_code = Some(code);
                inner.last_error = None;
            } else {
                inner.last_error = Some(CartError::InvalidCoupon { code });
            }
            Vec::new()
        });
    }

    pub fn clear_coupon(&self) {
        self.store.mutate(|inner| {
            inner.coupon_code = None;
            inner.last_error = None;
            Vec::new()
        });
    }

    pub fn checkout(&self) {
        self.store.mutate(|inner| {
            inner.last_error = if inner.cart.is_empty() {
                Some(CartError::CartEmpty)
            } else {
                None
            };
            Vec::new()
        });
    }

    pub fn get_state(&self) -> ShoppingCartState {
        self.store.state()
    }

    pub fn subscribe(&self, observer: Arc<dyn ShoppingCartObserver>) -> SubscriptionId {
        let state = self.get_state();
        let id = self.store.state_observers.subscribe(observer.clone());
        // Replay keeps totals and product catalog available on first render.
        observer.on_state(state);
        id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.store.state_observers.unsubscribe(id);
    }
}

// Child diff-list VM: generated iOS/Android bridges maintain the visible cart
// collection by applying these diffs, rather than requiring platform code to
// recalculate list positions.
#[vm_bridge(
    mode = "diff_list",
    diff = CartDiff,
    item = CartItem,
    factory = ShoppingCartViewModel::make_cart_vm
)]
#[uniffi::export]
impl CartViewModel {
    pub fn add_product(&self, product_id: i64, quantity: i64) {
        self.store.mutate(|inner| {
            let Some(product) = product(inner, product_id) else {
                inner.last_error = Some(CartError::ProductNotFound { product_id });
                return Vec::new();
            };
            if quantity <= 0 {
                inner.last_error = Some(CartError::QuantityMustBePositive);
                return Vec::new();
            }
            let existing_index = inner
                .cart
                .iter()
                .position(|item| item.product_id == product_id);
            let requested = existing_index
                .map(|index| inner.cart[index].quantity + quantity)
                .unwrap_or(quantity);
            if requested > product.stock {
                inner.last_error = Some(CartError::OutOfStock {
                    product_id,
                    requested,
                    available: product.stock,
                });
                return Vec::new();
            }
            inner.last_error = None;
            match existing_index {
                Some(index) => {
                    inner.cart[index] = cart_item(&product, requested, index);
                    vec![CartDiff::Update {
                        index: index as i64,
                        item: inner.cart[index].clone(),
                    }]
                }
                None => {
                    let index = inner.cart.len();
                    let item = cart_item(&product, quantity, index);
                    inner.cart.push(item.clone());
                    vec![CartDiff::Insert {
                        index: index as i64,
                        item,
                    }]
                }
            }
        });
    }

    pub fn set_quantity(&self, product_id: i64, quantity: i64) {
        self.store.mutate(|inner| {
            if quantity <= 0 {
                inner.last_error = Some(CartError::QuantityMustBePositive);
                return Vec::new();
            }
            let Some(product) = product(inner, product_id) else {
                inner.last_error = Some(CartError::ProductNotFound { product_id });
                return Vec::new();
            };
            if quantity > product.stock {
                inner.last_error = Some(CartError::OutOfStock {
                    product_id,
                    requested: quantity,
                    available: product.stock,
                });
                return Vec::new();
            }
            let Some(index) = inner
                .cart
                .iter()
                .position(|item| item.product_id == product_id)
            else {
                inner.last_error = Some(CartError::ProductNotFound { product_id });
                return Vec::new();
            };
            inner.cart[index] = cart_item(&product, quantity, index);
            inner.last_error = None;
            vec![CartDiff::Update {
                index: index as i64,
                item: inner.cart[index].clone(),
            }]
        });
    }

    pub fn remove_product(&self, product_id: i64) {
        self.store.mutate(|inner| {
            let Some(index) = inner
                .cart
                .iter()
                .position(|item| item.product_id == product_id)
            else {
                inner.last_error = Some(CartError::ProductNotFound { product_id });
                return Vec::new();
            };
            inner.cart.remove(index);
            normalize_positions(&mut inner.cart);
            inner.last_error = None;
            vec![CartDiff::Remove {
                index: index as i64,
                product_id,
            }]
        });
    }

    pub fn clear_cart(&self) {
        self.store.mutate(|inner| {
            let old = inner.cart.clone();
            inner.cart.clear();
            inner.coupon_code = None;
            inner.last_error = None;
            replace_cart_diffs(&old, &inner.cart)
        });
    }

    pub fn subscribe(&self, observer: Arc<dyn CartObserver>) -> SubscriptionId {
        let cart = self.store.cart_items();
        let id = self.store.cart_observers.subscribe(observer.clone());
        if !cart.is_empty() {
            // Initial cart contents are sent as inserts so a newly created list
            // bridge has the same state as Rust before live updates arrive.
            observer.on_diffs(
                cart.into_iter()
                    .enumerate()
                    .map(|(index, item)| CartDiff::Insert {
                        index: index as i64,
                        item,
                    })
                    .collect(),
            );
        }
        id
    }

    pub fn unsubscribe(&self, id: SubscriptionId) {
        self.store.cart_observers.unsubscribe(id);
    }
}

impl Store {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StoreInner {
                products: default_products(),
                cart: Vec::new(),
                coupon_code: None,
                last_error: None,
            })),
            cart_observers: ObserverSet::new(),
            state_observers: ObserverSet::new(),
        }
    }

    fn state(&self) -> ShoppingCartState {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        shopping_cart_state(&inner)
    }

    fn cart_items(&self) -> Vec<CartItem> {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.cart.clone()
    }

    fn mutate(&self, mutate: impl FnOnce(&mut StoreInner) -> Vec<CartDiff>) {
        let (state, diffs) = {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let diffs = mutate(&mut inner);
            (shopping_cart_state(&inner), diffs)
        };
        let state_observers = self.state_observers.snapshot();
        ObserverSet::notify_snapshot(&state_observers, |observer| {
            observer.on_state(state.clone())
        });
        if !diffs.is_empty() {
            let cart_observers = self.cart_observers.snapshot();
            ObserverSet::notify_snapshot(&cart_observers, |observer| {
                observer.on_diffs(diffs.clone())
            });
        }
    }
}

fn default_products() -> Vec<Product> {
    vec![
        Product {
            id: 1,
            name: "Coffee".to_string(),
            price_cents: 1299,
            stock: 5,
        },
        Product {
            id: 2,
            name: "Tea".to_string(),
            price_cents: 499,
            stock: 8,
        },
        Product {
            id: 3,
            name: "Mug".to_string(),
            price_cents: 1599,
            stock: 2,
        },
    ]
}

fn shopping_cart_state(inner: &StoreInner) -> ShoppingCartState {
    let subtotal_cents = inner
        .cart
        .iter()
        .map(|item| item.line_total_cents)
        .sum::<i64>();
    let discount_cents = if inner.coupon_code.as_deref() == Some(SAVE10) {
        subtotal_cents / 10
    } else {
        0
    };
    let taxable_cents = subtotal_cents - discount_cents;
    let tax_cents = round_div(taxable_cents * TAX_BASIS_POINTS, 10_000);
    ShoppingCartState {
        products: inner.products.clone(),
        coupon_code: inner.coupon_code.clone(),
        subtotal_cents,
        discount_cents,
        tax_cents,
        total_cents: taxable_cents + tax_cents,
        item_count: inner.cart.iter().map(|item| item.quantity).sum(),
        checkout_ready: !inner.cart.is_empty() && inner.last_error.is_none(),
        last_error: inner.last_error.clone(),
    }
}

fn product(inner: &StoreInner, product_id: i64) -> Option<Product> {
    inner
        .products
        .iter()
        .find(|product| product.id == product_id)
        .cloned()
}

fn cart_item(product: &Product, quantity: i64, position: usize) -> CartItem {
    CartItem {
        product_id: product.id,
        name: product.name.clone(),
        unit_price_cents: product.price_cents,
        quantity,
        line_total_cents: product.price_cents * quantity,
        position: position as i64,
    }
}

fn replace_cart_diffs(old: &[CartItem], new: &[CartItem]) -> Vec<CartDiff> {
    let mut diffs = Vec::new();
    for (index, item) in old.iter().enumerate().rev() {
        diffs.push(CartDiff::Remove {
            index: index as i64,
            product_id: item.product_id,
        });
    }
    for (index, item) in new.iter().cloned().enumerate() {
        diffs.push(CartDiff::Insert {
            index: index as i64,
            item,
        });
    }
    diffs
}

fn normalize_positions(cart: &mut [CartItem]) {
    for (index, item) in cart.iter_mut().enumerate() {
        item.position = index as i64;
    }
}

fn round_div(numerator: i64, denominator: i64) -> i64 {
    (numerator + denominator / 2) / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RecordingStateObserver {
        states: Mutex<Vec<ShoppingCartState>>,
    }

    struct RecordingCartObserver {
        diffs: Mutex<Vec<Vec<CartDiff>>>,
    }

    impl RecordingStateObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                states: Mutex::new(Vec::new()),
            })
        }

        fn states(&self) -> Vec<ShoppingCartState> {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl RecordingCartObserver {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                diffs: Mutex::new(Vec::new()),
            })
        }

        fn diffs(&self) -> Vec<Vec<CartDiff>> {
            self.diffs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        }
    }

    impl ShoppingCartObserver for RecordingStateObserver {
        fn on_state(&self, state: ShoppingCartState) {
            self.states
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(state);
        }
    }

    impl CartObserver for RecordingCartObserver {
        fn on_diffs(&self, diffs: Vec<CartDiff>) {
            self.diffs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(diffs);
        }
    }

    #[test]
    fn starts_with_catalog_and_empty_cart() {
        let cart = ShoppingCartViewModel::new();
        let state = cart.get_state();

        assert_eq!(state.products.len(), 3);
        assert_eq!(state.products[0].name, "Coffee");
        assert_eq!(state.item_count, 0);
        assert_eq!(state.subtotal_cents, 0);
        assert_eq!(state.tax_cents, 0);
        assert!(!state.checkout_ready);
    }

    #[test]
    fn add_product_inserts_cart_item_and_computes_totals() {
        let (cart, items, observer) = subscribed_cart();

        items.add_product(1, 1);

        let state = cart.get_state();
        assert_eq!(state.subtotal_cents, 1299);
        assert_eq!(state.tax_cents, 107);
        assert_eq!(state.total_cents, 1406);
        assert_eq!(state.item_count, 1);
        assert!(state.checkout_ready);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![CartDiff::Insert {
                index: 0,
                item: CartItem {
                    product_id: 1,
                    name: "Coffee".to_string(),
                    unit_price_cents: 1299,
                    quantity: 1,
                    line_total_cents: 1299,
                    position: 0,
                },
            }]
        );
    }

    #[test]
    fn add_existing_product_merges_quantity_and_emits_update() {
        let (cart, items, observer) = subscribed_cart();
        items.add_product(1, 1);

        items.add_product(1, 2);

        assert_eq!(cart.get_state().item_count, 3);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![CartDiff::Update {
                index: 0,
                item: CartItem {
                    product_id: 1,
                    name: "Coffee".to_string(),
                    unit_price_cents: 1299,
                    quantity: 3,
                    line_total_cents: 3897,
                    position: 0,
                },
            }]
        );
    }

    #[test]
    fn add_rejects_zero_quantity_without_diff() {
        let (cart, items, observer) = subscribed_cart();

        items.add_product(1, 0);

        assert_eq!(
            cart.get_state().last_error,
            Some(CartError::QuantityMustBePositive)
        );
        assert!(observer.diffs().is_empty());
    }

    #[test]
    fn add_rejects_stock_overflow_without_changing_cart() {
        let (cart, items, observer) = subscribed_cart();

        items.add_product(3, 3);

        assert_eq!(
            cart.get_state().last_error,
            Some(CartError::OutOfStock {
                product_id: 3,
                requested: 3,
                available: 2,
            })
        );
        assert_eq!(cart.get_state().item_count, 0);
        assert!(observer.diffs().is_empty());
    }

    #[test]
    fn unknown_product_operations_do_not_mutate_cart() {
        let (cart, items, observer) = subscribed_cart();
        items.add_product(1, 1);
        let diffs_before = observer.diffs().len();

        items.add_product(999, 1);
        assert_eq!(
            cart.get_state().last_error,
            Some(CartError::ProductNotFound { product_id: 999 })
        );
        assert_eq!(cart.get_state().item_count, 1);
        assert_eq!(observer.diffs().len(), diffs_before);

        items.set_quantity(999, 2);
        assert_eq!(
            cart.get_state().last_error,
            Some(CartError::ProductNotFound { product_id: 999 })
        );
        assert_eq!(cart.get_state().item_count, 1);
        assert_eq!(observer.diffs().len(), diffs_before);

        items.remove_product(999);
        assert_eq!(
            cart.get_state().last_error,
            Some(CartError::ProductNotFound { product_id: 999 })
        );
        assert_eq!(cart.get_state().item_count, 1);
        assert_eq!(observer.diffs().len(), diffs_before);
    }

    #[test]
    fn set_quantity_updates_line_total_and_rounds_tax() {
        let (cart, items, observer) = subscribed_cart();
        items.add_product(2, 1);

        items.set_quantity(2, 3);

        let state = cart.get_state();
        assert_eq!(state.subtotal_cents, 1497);
        assert_eq!(state.tax_cents, 124);
        assert_eq!(state.total_cents, 1621);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![CartDiff::Update {
                index: 0,
                item: CartItem {
                    product_id: 2,
                    name: "Tea".to_string(),
                    unit_price_cents: 499,
                    quantity: 3,
                    line_total_cents: 1497,
                    position: 0,
                },
            }]
        );
    }

    #[test]
    fn set_quantity_zero_is_invalid_because_remove_is_explicit() {
        let (cart, items, observer) = subscribed_cart();
        items.add_product(1, 1);
        let before = observer.diffs().len();

        items.set_quantity(1, 0);

        assert_eq!(
            cart.get_state().last_error,
            Some(CartError::QuantityMustBePositive)
        );
        assert_eq!(cart.get_state().item_count, 1);
        assert_eq!(observer.diffs().len(), before);
    }

    #[test]
    fn remove_product_emits_remove_and_recomputes_positions() {
        let (cart, items, observer) = subscribed_cart();
        items.add_product(1, 1);
        items.add_product(2, 1);

        items.remove_product(1);

        assert_eq!(cart.get_state().item_count, 1);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![CartDiff::Remove {
                index: 0,
                product_id: 1,
            }]
        );
    }

    #[test]
    fn save10_coupon_updates_discount_tax_and_total() {
        let (cart, items, _observer) = subscribed_cart();
        items.add_product(1, 1);

        cart.apply_coupon(" save10 ".to_string());

        let state = cart.get_state();
        assert_eq!(state.coupon_code.as_deref(), Some(SAVE10));
        assert_eq!(state.discount_cents, 129);
        assert_eq!(state.tax_cents, 97);
        assert_eq!(state.total_cents, 1267);
    }

    #[test]
    fn invalid_coupon_keeps_existing_coupon_and_reports_typed_error() {
        let (cart, items, _observer) = subscribed_cart();
        items.add_product(1, 1);
        cart.apply_coupon(SAVE10.to_string());

        cart.apply_coupon("bogus".to_string());

        let state = cart.get_state();
        assert_eq!(state.coupon_code.as_deref(), Some(SAVE10));
        assert_eq!(
            state.last_error,
            Some(CartError::InvalidCoupon {
                code: "BOGUS".to_string(),
            })
        );
        assert_eq!(state.total_cents, 1267);
        assert!(!state.checkout_ready);
    }

    #[test]
    fn coupon_recomputes_after_cart_quantity_changes_and_clear_coupon() {
        let (cart, items, _observer) = subscribed_cart();
        items.add_product(1, 1);
        cart.apply_coupon(SAVE10.to_string());

        items.add_product(1, 1);

        let state = cart.get_state();
        assert_eq!(state.subtotal_cents, 2598);
        assert_eq!(state.discount_cents, 259);
        assert_eq!(state.tax_cents, 193);
        assert_eq!(state.total_cents, 2532);

        cart.clear_coupon();

        let state = cart.get_state();
        assert_eq!(state.coupon_code, None);
        assert_eq!(state.discount_cents, 0);
        assert_eq!(state.tax_cents, 214);
        assert_eq!(state.total_cents, 2812);
        assert!(state.checkout_ready);
    }

    #[test]
    fn clear_cart_removes_everything_and_clears_coupon() {
        let (cart, items, observer) = subscribed_cart();
        items.add_product(1, 1);
        items.add_product(2, 1);
        cart.apply_coupon(SAVE10.to_string());

        items.clear_cart();

        let state = cart.get_state();
        assert_eq!(state.item_count, 0);
        assert_eq!(state.coupon_code, None);
        assert!(!state.checkout_ready);
        assert_eq!(
            observer.diffs().last().unwrap(),
            &vec![
                CartDiff::Remove {
                    index: 1,
                    product_id: 2,
                },
                CartDiff::Remove {
                    index: 0,
                    product_id: 1,
                },
            ]
        );
    }

    #[test]
    fn checkout_empty_cart_sets_cart_empty_error() {
        let cart = ShoppingCartViewModel::new();

        cart.checkout();

        assert_eq!(cart.get_state().last_error, Some(CartError::CartEmpty));
    }

    #[test]
    fn checkout_with_valid_cart_clears_previous_error() {
        let cart = ShoppingCartViewModel::new();
        let items = cart.clone().make_cart_vm();
        items.add_product(1, 1);
        cart.apply_coupon("bogus".to_string());
        assert!(!cart.get_state().checkout_ready);

        cart.checkout();

        let state = cart.get_state();
        assert_eq!(state.last_error, None);
        assert!(state.checkout_ready);
    }

    #[test]
    fn cart_subscribe_replays_current_items_and_unsubscribe_stops_updates() {
        let cart = ShoppingCartViewModel::new();
        let items = cart.clone().make_cart_vm();
        items.add_product(1, 1);
        let observer = RecordingCartObserver::new();

        let id = items.subscribe(observer.clone());
        items.unsubscribe(id);
        items.add_product(2, 1);

        let diffs = observer.diffs();
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0][0], CartDiff::Insert { index: 0, .. }));
    }

    #[test]
    fn state_subscribe_pushes_current_state_and_unsubscribe_stops_updates() {
        let cart = ShoppingCartViewModel::new();
        let items = cart.clone().make_cart_vm();
        let observer = RecordingStateObserver::new();

        let id = cart.subscribe(observer.clone());
        items.add_product(1, 1);
        cart.unsubscribe(id);
        items.add_product(2, 1);

        let states = observer.states();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].item_count, 0);
        assert_eq!(states[1].item_count, 1);
    }

    fn subscribed_cart() -> (
        Arc<ShoppingCartViewModel>,
        Arc<CartViewModel>,
        Arc<RecordingCartObserver>,
    ) {
        let cart = ShoppingCartViewModel::new();
        let items = cart.clone().make_cart_vm();
        let observer = RecordingCartObserver::new();
        items.subscribe(observer.clone());
        (cart, items, observer)
    }
}
