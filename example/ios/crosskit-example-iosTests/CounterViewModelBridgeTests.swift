import CrossKitShared
import XCTest

@MainActor
final class CounterViewModelBridgeTests: XCTestCase {
    func testIncrementUpdatesValue() {
        let vm = CounterViewModelBridge(initial: 1)
        XCTAssertEqual(vm.state.value, 1)

        vm.increment()
        XCTAssertEqual(vm.state.value, 2)
    }
}
