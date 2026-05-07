import Foundation
import XCTest

final class crosskit_example_iosUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testSearchFlowUsesGeneratedState() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["search.title"].waitForExistence(timeout: 2))
        app.textFields["search.query"].tap()
        app.textFields["search.query"].typeText("rust")

        tap(app, id: "search.submit")
        XCTAssertTrue(waitForLabel("Loading", id: "search.loading", in: app))
        XCTAssertTrue(waitForLabel("Progress 0%", id: "search.progress", in: app))

        tap(app, id: "search.tick")
        XCTAssertTrue(waitForLabel("Progress 50%", id: "search.progress", in: app))

        tap(app, id: "search.tick")
        XCTAssertTrue(waitForLabel("Idle", id: "search.loading", in: app))
        XCTAssertTrue(waitForLabel("Progress 100%", id: "search.progress", in: app))
        XCTAssertTrue(waitForLabel("rust guide", id: "search.result.1.title", in: app))
    }

    @MainActor
    func testErrorRendersFromState() throws {
        let app = XCUIApplication()
        app.launch()

        app.textFields["search.query"].tap()
        app.textFields["search.query"].typeText("network")
        tap(app, id: "search.submit")
        tap(app, id: "search.tick")
        tap(app, id: "search.tick")
        XCTAssertTrue(waitForLabelContaining("network", id: "search.error", in: app))
    }

    @MainActor
    func testCancelRendersFromState() throws {
        let app = XCUIApplication()
        app.launch()

        app.textFields["search.query"].tap()
        app.textFields["search.query"].typeText("rust")
        tap(app, id: "search.submit")
        tap(app, id: "search.cancel")
        XCTAssertTrue(waitForLabelContaining("cancelled", id: "search.error", in: app))
    }

    @MainActor
    func testLaunchPerformance() throws {
        measure(metrics: [XCTApplicationLaunchMetric()]) {
            XCUIApplication().launch()
        }
    }

    @MainActor
    private func tap(_ app: XCUIApplication, id: String) {
        let button = app.buttons[id]
        XCTAssertTrue(button.waitForExistence(timeout: 2))
        button.tap()
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

    @MainActor
    private func waitForLabelContaining(_ fragment: String, id: String, in app: XCUIApplication) -> Bool {
        let element = app.staticTexts[id]
        let deadline = Date().addingTimeInterval(2)
        while Date() < deadline {
            if element.exists && element.label.contains(fragment) {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.05))
        }
        return element.exists && element.label.contains(fragment)
    }
}
