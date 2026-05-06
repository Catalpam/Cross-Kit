import Combine
import CrossKitTaskBoardShared
import XCTest

@MainActor
final class TaskBoardBridgeTests: XCTestCase {
    func testRootContainerStartsEmpty() {
        let kit = CrossKitTaskBoardBridge()

        XCTAssertEqual(kit.taskBoard.state.filter, .all)
        XCTAssertEqual(kit.taskBoard.state.totalCount, 0)
        XCTAssertTrue(kit.taskList.items.isEmpty)
    }

    func testBatchToggleFilterMoveAndClearFlow() async {
        let kit = CrossKitTaskBoardBridge()

        kit.taskList.addSampleBatch()
        var updated = await waitFor { kit.taskList.items.count == 3 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.taskBoard.state.openCount, 3)

        let firstId = kit.taskList.items[0].id
        kit.taskList.toggleDone(id: firstId)
        updated = await waitFor { kit.taskBoard.state.doneCount == 1 }
        XCTAssertTrue(updated)

        kit.taskBoard.setFilter(filter: .done)
        updated = await waitFor { kit.taskList.items.count == 1 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.taskList.items[0].id, firstId)

        kit.taskBoard.setFilter(filter: .all)
        _ = await waitFor { kit.taskList.items.count == 3 }
        kit.taskList.moveVisible(from: 0, to: 2)
        updated = await waitFor { kit.taskList.items.last?.id == firstId }
        XCTAssertTrue(updated)

        let lastId = kit.taskList.items[2].id
        kit.taskList.moveVisible(from: 2, to: 0)
        updated = await waitFor { kit.taskList.items.first?.id == lastId }
        XCTAssertTrue(updated)

        kit.taskList.renameTask(id: lastId, title: "Renamed")
        updated = await waitFor { kit.taskList.items.first?.title == "Renamed" }
        XCTAssertTrue(updated)

        kit.taskList.clearDone()
        updated = await waitFor { kit.taskBoard.state.doneCount == 0 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.taskBoard.state.totalCount, 2)
    }

    func testInvalidAddAndMoveSurfaceRustOwnedErrors() async {
        let kit = CrossKitTaskBoardBridge()

        kit.taskList.addTask(title: " ")
        var updated = await waitFor { kit.taskBoard.state.lastError == "Task title is required" }
        XCTAssertTrue(updated)

        kit.taskList.addTask(title: "One")
        _ = await waitFor { kit.taskList.items.count == 1 }
        kit.taskList.moveVisible(from: 9, to: 0)
        updated = await waitFor { kit.taskBoard.state.lastError == "Move index is out of range" }
        XCTAssertTrue(updated)
    }

    func testRootContainerForwardsTaskListChanges() async {
        let kit = CrossKitTaskBoardBridge()
        var forwardedChanges = 0
        let cancellable = kit.objectWillChange.sink {
            forwardedChanges += 1
        }

        kit.taskList.addTask(title: "One")
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
