import CrossKitShared
import XCTest

@MainActor
final class ListViewModelBridgeTests: XCTestCase {
    func testListDiffsApplyToItems() async {
        let app = AppViewModelBridge(initial: 0)
        let vm = ListViewModelBridge(app: app)
        XCTAssertEqual(vm.items.count, 0)

        vm.insertWithTimestamp(index: 0, timestampMs: 1_000)
        vm.insertWithTimestamp(index: 1, timestampMs: 2_000)
        let inserted = await waitFor { vm.items.count == 2 }
        XCTAssertTrue(inserted)
        XCTAssertEqual(vm.items.count, 2)
        XCTAssertEqual(vm.items[0].timestampMs, 1_000)
        XCTAssertEqual(vm.items[1].timestampMs, 2_000)

        vm.updateWithTimestamp(index: 0, timestampMs: 3_000)
        let updated = await waitFor { vm.items.first?.timestampMs == 3_000 }
        XCTAssertTrue(updated)
        XCTAssertEqual(vm.items[0].timestampMs, 3_000)

        vm.moveItem(from: 1, to: 0)
        let moved = await waitFor { vm.items.first?.timestampMs == 2_000 }
        XCTAssertTrue(moved)
        XCTAssertEqual(vm.items[0].timestampMs, 2_000)

        XCTAssertTrue(vm.sortByTimestampDesc())
        let sorted = await waitFor {
            vm.items.count == 2 && vm.items[0].timestampMs >= vm.items[1].timestampMs
        }
        XCTAssertTrue(sorted)
        XCTAssertGreaterThanOrEqual(vm.items[0].timestampMs, vm.items[1].timestampMs)

        vm.removeAt(index: 1)
        let removed = await waitFor { vm.items.count == 1 }
        XCTAssertTrue(removed)
        XCTAssertEqual(vm.items.count, 1)
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
