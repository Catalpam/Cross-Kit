# Shopping Cart

This example shows a Rust-owned shopping cart rendered through generated iOS and
Android libraries:

- Rust owns `ShoppingCartViewModel`, `CartViewModel`, the product catalog,
  cart merge rules, quantity validation, coupon validation, tax rounding,
  totals, list diffs, observer subscription, and metadata.
- iOS uses `CrossKitShoppingCartBridge()` and renders `kit.shoppingCart.state` plus
  `kit.cart.items`.
- Android uses `rememberCrossKitShoppingCartBridge()` and renders
  `kit.shoppingCart.state` plus `kit.cart.items`.
- Platform code only displays state/items and invokes actions such as
  `addProduct`, `setQuantity`, `removeProduct`, `applyCoupon`, and
  `checkout`; it does not recalculate totals, discounts, taxes, stock rules, or
  list diffs.

Run the shared tests and metadata binary:

```bash
cargo test -p shopping-cart-shared --lib --tests
cargo run -p shopping-cart-shared --bin ck_shopping_cart_metadata
```

Package and build iOS:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/shopping-cart/cross-kit.toml
xcodebuild -project examples/shopping-cart/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Package and build Android:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/shopping-cart/cross-kit.toml

cd examples/shopping-cart/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug testDebugUnitTest assembleDebugAndroidTest
```
