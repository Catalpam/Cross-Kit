import Foundation
import XCTest

final class crosskit_example_iosUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testTaskBoardUsesGeneratedBridgeState() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["task.title"].waitForExistence(timeout: 2))
        XCTAssertEqual(app.staticTexts["task.total"].label, "Total 0")

        app.buttons["task.sample"].tap()
        XCTAssertTrue(waitForLabel("Open 3", id: "task.open.count", in: app))
        XCTAssertTrue(waitForLabel("Plan", id: "task.position.0", in: app))

        app.buttons["task.toggle.1"].tap()
        XCTAssertTrue(waitForLabel("Done 1", id: "task.done.count", in: app))

        app.buttons["task.filter.done"].tap()
        XCTAssertTrue(waitForLabel("Plan", id: "task.position.0", in: app))
        XCTAssertFalse(app.staticTexts["Build"].exists)

        app.buttons["task.filter.all"].tap()
        XCTAssertTrue(waitForLabel("Review", id: "task.position.2", in: app))
        app.buttons["task.move"].tap()
        XCTAssertTrue(waitForLabel("Plan", id: "task.position.2", in: app))
        app.buttons["task.rename"].tap()
        XCTAssertTrue(waitForLabel("Renamed", id: "task.position.0", in: app))

        app.buttons["task.clear.done"].tap()
        XCTAssertTrue(waitForLabel("Total 2", id: "task.total", in: app))
    }

    @MainActor
    func testLaunchPerformance() throws {
        measure(metrics: [XCTApplicationLaunchMetric()]) {
            XCUIApplication().launch()
        }
    }

    @MainActor
    private func waitForLabel(_ label: String, id: String, in app: XCUIApplication) -> Bool {
        let element = app.staticTexts[id]
        let deadline = Date().addingTimeInterval(2)
        while Date() < deadline {
            if element.exists && element.label == label {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        return element.exists && element.label == label
    }
}
