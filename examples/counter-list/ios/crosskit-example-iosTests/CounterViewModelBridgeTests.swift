import CrossKitShared
import XCTest

@MainActor
final class CounterViewModelBridgeTests: XCTestCase {
    func testIncrementUpdatesValue() async {
        let app = AppViewModelBridge(initial: 1)
        let vm = CounterViewModelBridge(app: app)
        XCTAssertEqual(vm.state.value, 1)

        _ = vm.increment()
        let updated = await waitFor { vm.state.value == 2 }
        XCTAssertTrue(updated)
        XCTAssertEqual(vm.state.value, 2)
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
