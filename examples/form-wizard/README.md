# Form Wizard

This example shows a Rust-owned form flow that iOS and Android render through
generated platform libraries:

- Rust owns `FormWizardViewModel`, `FormWizardState`, validation, navigation,
  completion state, observer subscription, and metadata.
- iOS uses `CrossKitFormWizardBridge()` and renders `kit.formWizard.state`.
- Android uses `rememberCrossKitFormWizardBridge()` and renders
  `kit.formWizard.state`.
- Platform code only displays state/errors and invokes actions such as
  `updateName`, `next`, and `back`; it does not duplicate validation rules.

Run the shared tests and metadata binary:

```bash
cargo test -p form-wizard-shared --lib --tests
cargo run -p form-wizard-shared --bin ck_form_wizard_metadata
```

Package and build iOS:

```bash
cargo run -p cross-kit-cli -- ios package --config examples/form-wizard/cross-kit.toml
xcodebuild -project examples/form-wizard/ios/crosskit-example-ios.xcodeproj \
  -scheme crosskit-example-ios \
  -configuration Debug \
  -destination 'generic/platform=iOS Simulator' build
```

Package and build Android:

```bash
JAVA_HOME=/opt/homebrew/opt/openjdk@21 \
cargo run -p cross-kit-cli -- android package --config examples/form-wizard/cross-kit.toml

cd examples/form-wizard/android
JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew clean assembleDebug testDebugUnitTest assembleDebugAndroidTest
```
