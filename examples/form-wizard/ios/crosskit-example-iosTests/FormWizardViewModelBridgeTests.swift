import Combine
import CrossKitFormWizardShared
import XCTest

@MainActor
final class FormWizardViewModelBridgeTests: XCTestCase {
    func testRootContainerStartsAtProfileStep() {
        let kit = CrossKitFormWizardBridge()

        XCTAssertEqual(kit.formWizard.state.step, .profile)
        XCTAssertFalse(kit.formWizard.state.canGoNext)
        XCTAssertEqual(kit.formWizard.state.nameError, "Name must be at least 2 characters")
        XCTAssertEqual(kit.formWizard.state.emailError, "Email is required")
    }

    func testProfileSecuritySummaryAndCompletionFlow() async {
        let kit = CrossKitFormWizardBridge()

        kit.formWizard.updateName(name: "Ada Lovelace")
        kit.formWizard.updateEmail(email: "ada@example.com")
        var updated = await waitFor { kit.formWizard.state.canGoNext }
        XCTAssertTrue(updated)

        kit.formWizard.next()
        updated = await waitFor { kit.formWizard.state.step == .security }
        XCTAssertTrue(updated)

        kit.formWizard.updatePassword(password: "password1")
        kit.formWizard.updateConfirm(confirm: "password2")
        updated = await waitFor { kit.formWizard.state.confirmPasswordError == "Passwords must match" }
        XCTAssertTrue(updated)
        XCTAssertFalse(kit.formWizard.state.canGoNext)

        kit.formWizard.updateConfirm(confirm: "password1")
        updated = await waitFor { kit.formWizard.state.canGoNext }
        XCTAssertTrue(updated)

        kit.formWizard.next()
        updated = await waitFor { kit.formWizard.state.step == .summary }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.formWizard.state.summary, "Ada Lovelace <ada@example.com>")

        kit.formWizard.next()
        updated = await waitFor { kit.formWizard.state.isComplete }
        XCTAssertTrue(updated)
        XCTAssertFalse(kit.formWizard.state.canGoNext)
    }

    func testBackKeepsRustOwnedFieldState() async {
        let kit = CrossKitFormWizardBridge()

        kit.formWizard.updateName(name: "Ada Lovelace")
        kit.formWizard.updateEmail(email: "ada@example.com")
        _ = await waitFor { kit.formWizard.state.canGoNext }
        kit.formWizard.next()
        _ = await waitFor { kit.formWizard.state.step == .security }

        kit.formWizard.back()
        let updated = await waitFor { kit.formWizard.state.step == .profile }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.formWizard.state.email, "ada@example.com")
        XCTAssertTrue(kit.formWizard.state.canGoNext)
    }

    func testRootContainerForwardsFormWizardChanges() async {
        let kit = CrossKitFormWizardBridge()
        var forwardedChanges = 0
        let cancellable = kit.objectWillChange.sink {
            forwardedChanges += 1
        }

        kit.formWizard.updateName(name: "Grace Hopper")
        let forwarded = await waitFor { forwardedChanges > 0 }
        XCTAssertTrue(forwarded)

        cancellable.cancel()
    }

    private func waitFor(_ condition: @escaping () -> Bool, timeout: TimeInterval = 0.5) async -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return true }
            await Task.yield()
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        return condition()
    }
}
