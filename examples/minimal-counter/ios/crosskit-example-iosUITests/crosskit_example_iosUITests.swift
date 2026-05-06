import XCTest

final class crosskit_example_iosUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testCounterButtonsUpdateDisplayedValue() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["Minimal Counter"].waitForExistence(timeout: 2))
        assertCounterValue("0", in: app)

        tap(app, id: "counter.increment")
        assertCounterValue("1", in: app)

        tap(app, id: "counter.decrement")
        assertCounterValue("0", in: app)

        tap(app, id: "counter.increment")
        tap(app, id: "counter.increment")
        assertCounterValue("2", in: app)

        tap(app, id: "counter.reset")
        assertCounterValue("0", in: app)
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
    private func assertCounterValue(_ value: String, in app: XCUIApplication) {
        let label = app.staticTexts["counter.value"]
        XCTAssertTrue(label.waitForExistence(timeout: 2))
        XCTAssertEqual(label.label, value)
    }
}
