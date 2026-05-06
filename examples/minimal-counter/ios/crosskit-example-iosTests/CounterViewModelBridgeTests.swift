import Combine
import CrossKitMinimalCounterShared
import XCTest

@MainActor
final class CounterViewModelBridgeTests: XCTestCase {
    func testRootContainerStartsFromInitialValue() {
        let kit = CrossKitMinimalCounterBridge(initial: 3)

        XCTAssertEqual(kit.counter.state.value, 3)
    }

    func testIncrementDecrementAndResetUpdateValue() async {
        let kit = CrossKitMinimalCounterBridge(initial: 1)

        _ = kit.counter.increment()
        var updated = await waitFor { kit.counter.state.value == 2 }
        XCTAssertTrue(updated)

        _ = kit.counter.decrement()
        updated = await waitFor { kit.counter.state.value == 1 }
        XCTAssertTrue(updated)

        _ = kit.counter.reset()
        updated = await waitFor { kit.counter.state.value == 0 }
        XCTAssertTrue(updated)
    }

    func testRootContainerForwardsCounterChanges() async {
        let kit = CrossKitMinimalCounterBridge(initial: 0)
        var forwardedChanges = 0
        let cancellable = kit.objectWillChange.sink {
            forwardedChanges += 1
        }

        _ = kit.counter.increment()
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
