import CrossKitShared
import XCTest

@MainActor
final class ListViewModelBridgeTests: XCTestCase {
    func testListDiffsApplyToItems() {
        let vm = ListViewModelBridge()
        XCTAssertEqual(vm.items.count, 0)

        vm.insertWithTimestamp(index: 0, timestampMs: 1_000)
        vm.insertWithTimestamp(index: 1, timestampMs: 2_000)
        XCTAssertEqual(vm.items.count, 2)
        XCTAssertEqual(vm.items[0].timestampMs, 1_000)
        XCTAssertEqual(vm.items[1].timestampMs, 2_000)

        vm.updateWithTimestamp(index: 0, timestampMs: 3_000)
        XCTAssertEqual(vm.items[0].timestampMs, 3_000)

        vm.moveItem(from: 1, to: 0)
        XCTAssertEqual(vm.items[0].timestampMs, 2_000)

        XCTAssertTrue(vm.sortByTimestampDesc())
        XCTAssertGreaterThanOrEqual(vm.items[0].timestampMs, vm.items[1].timestampMs)

        vm.removeAt(index: 1)
        XCTAssertEqual(vm.items.count, 1)
    }
}
