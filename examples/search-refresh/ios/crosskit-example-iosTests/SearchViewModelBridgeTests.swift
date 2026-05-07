import Combine
import CrossKitSearchRefreshShared
import XCTest

@MainActor
final class SearchViewModelBridgeTests: XCTestCase {
    func testRootContainerStartsIdle() {
        let kit = CrossKitSearchRefreshBridge()

        XCTAssertEqual(kit.search.state.query, "")
        XCTAssertFalse(kit.search.state.isLoading)
        XCTAssertFalse(kit.search.state.canSubmit)
        XCTAssertTrue(kit.search.state.results.isEmpty)
    }

    func testSubmitTickAndSuccessFlow() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.updateQuery(query: "rust")
        var updated = await waitFor { kit.search.state.canSubmit }
        XCTAssertTrue(updated)

        kit.search.submit()
        updated = await waitFor { kit.search.state.isLoading }
        XCTAssertTrue(updated)
        XCTAssertTrue(kit.search.state.canCancel)

        kit.search.tick()
        updated = await waitFor { kit.search.state.progress == 50 }
        XCTAssertTrue(updated)

        kit.search.tick()
        updated = await waitFor { kit.search.state.results.count == 3 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.search.state.results[0].title, "rust guide")
        XCTAssertFalse(kit.search.state.isLoading)
    }

    func testTypedErrorsAndCancelAreStateDriven() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.submit()
        var updated = await waitFor { String(describing: kit.search.state.error).contains("emptyQuery") }
        XCTAssertTrue(updated)

        kit.search.updateQuery(query: "network")
        kit.search.submit()
        kit.search.tick()
        kit.search.tick()
        updated = await waitFor { String(describing: kit.search.state.error).contains("network") }
        XCTAssertTrue(updated)

        kit.search.updateQuery(query: "rust")
        kit.search.submit()
        _ = await waitFor { kit.search.state.isLoading }
        kit.search.cancel()
        updated = await waitFor { String(describing: kit.search.state.error).contains("cancelled") }
        XCTAssertTrue(updated)
        XCTAssertFalse(kit.search.state.isLoading)
    }

    func testIdleCancelDoesNotWriteErrorState() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.updateQuery(query: "rust")
        _ = await waitFor { kit.search.state.canSubmit }
        kit.search.cancel()

        XCTAssertNil(kit.search.state.error)
        XCTAssertFalse(kit.search.state.isLoading)
        XCTAssertTrue(kit.search.state.canSubmit)
        XCTAssertFalse(kit.search.state.canCancel)
    }

    func testEditingQueryClearsRenderedResults() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.updateQuery(query: "old")
        kit.search.submit()
        kit.search.tick()
        kit.search.tick()
        var updated = await waitFor { kit.search.state.results.count == 3 }
        XCTAssertTrue(updated)

        kit.search.updateQuery(query: "new")
        updated = await waitFor { kit.search.state.results.isEmpty }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.search.state.query, "new")
        XCTAssertNil(kit.search.state.error)
    }

    func testRootContainerForwardsSearchChanges() async {
        let kit = CrossKitSearchRefreshBridge()
        var forwardedChanges = 0
        let cancellable = kit.objectWillChange.sink {
            forwardedChanges += 1
        }

        kit.search.updateQuery(query: "rust")
        let forwarded = await waitFor { forwardedChanges > 0 }
        XCTAssertTrue(forwarded)

        cancellable.cancel()
    }

    private func waitFor(_ condition: @escaping () -> Bool, timeout: TimeInterval = 0.7) async -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if condition() { return true }
            await Task.yield()
            try? await Task.sleep(nanoseconds: 20_000_000)
        }
        return condition()
    }
}
