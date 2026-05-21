import Combine
import CrossKitSearchRefreshShared
import XCTest

@MainActor
final class SearchViewModelBridgeTests: XCTestCase {
    func testRootContainerStartsIdle() {
        let kit = CrossKitSearchRefreshBridge()

        XCTAssertEqual(kit.search.state.query, "")
        XCTAssertEqual(kit.search.state.status, .idle)
        XCTAssertFalse(kit.search.state.isLoading)
        XCTAssertFalse(kit.search.state.canSubmit)
        XCTAssertFalse(kit.search.state.canRetry)
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
        XCTAssertEqual(kit.search.state.status, .loading)
        XCTAssertTrue(kit.search.state.canCancel)
        XCTAssertFalse(kit.search.state.canRetry)

        kit.search.tick()
        updated = await waitFor { kit.search.state.progress == 50 }
        XCTAssertTrue(updated)

        kit.search.tick()
        updated = await waitFor { kit.search.state.results.count == 3 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.search.state.status, .results)
        XCTAssertEqual(kit.search.state.results[0].title, "rust guide")
        XCTAssertFalse(kit.search.state.isLoading)
        XCTAssertNil(kit.search.state.notice)
    }

    func testPresentationNoticesAndCancelAreStateDriven() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.submit()
        var updated = await waitFor { self.inlineMessage(kit.search.state.notice) == "Enter a query to search." }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.search.state.status, .failed)
        XCTAssertFalse(kit.search.state.canRetry)

        kit.search.updateQuery(query: "network")
        kit.search.submit()
        kit.search.tick()
        kit.search.tick()
        updated = await waitFor { self.toastMessage(kit.search.state.notice) == "Search is temporarily unavailable." }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.search.state.status, .failed)
        XCTAssertTrue(kit.search.state.canRetry)

        kit.search.submit()
        updated = await waitFor { kit.search.state.status == .loading && kit.search.state.notice == nil }
        XCTAssertTrue(updated)
        kit.search.tick()
        kit.search.tick()
        updated = await waitFor { kit.search.state.results.first?.title == "network guide" }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.search.state.status, .results)
        XCTAssertNil(kit.search.state.notice)
        XCTAssertFalse(kit.search.state.canRetry)

        kit.search.updateQuery(query: "rust")
        kit.search.submit()
        _ = await waitFor { kit.search.state.isLoading }
        kit.search.cancel()
        updated = await waitFor { kit.search.state.status == .idle && kit.search.state.notice == nil }
        XCTAssertTrue(updated)
        XCTAssertFalse(kit.search.state.isLoading)
        XCTAssertFalse(kit.search.state.canRetry)
    }

    func testIdleCancelDoesNotWriteErrorState() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.updateQuery(query: "rust")
        _ = await waitFor { kit.search.state.canSubmit }
        kit.search.cancel()

        XCTAssertNil(kit.search.state.notice)
        XCTAssertEqual(kit.search.state.status, .idle)
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
        XCTAssertEqual(kit.search.state.status, .idle)
        XCTAssertNil(kit.search.state.notice)
    }

    func testEmptyResultsUsePresentationState() async {
        let kit = CrossKitSearchRefreshBridge()

        kit.search.updateQuery(query: "empty")
        kit.search.submit()
        kit.search.tick()
        kit.search.tick()

        let updated = await waitFor { kit.search.state.status == .empty }
        XCTAssertTrue(updated)
        XCTAssertEqual(inlineMessage(kit.search.state.notice), "No results for \"empty\".")
        XCTAssertTrue(kit.search.state.results.isEmpty)
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

    private func inlineMessage(_ notice: SearchNotice?) -> String? {
        guard let notice else { return nil }
        if case let .inline(message) = notice {
            return message
        }
        return nil
    }

    private func toastMessage(_ notice: SearchNotice?) -> String? {
        guard let notice else { return nil }
        if case let .toast(message) = notice {
            return message
        }
        return nil
    }
}
