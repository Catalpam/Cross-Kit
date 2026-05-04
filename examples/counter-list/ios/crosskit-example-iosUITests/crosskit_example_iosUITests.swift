//
//  crosskit_example_iosUITests.swift
//  crosskit-example-iosUITests
//
//  Created by zigengm3 on 2026/2/4.
//

import XCTest

final class crosskit_example_iosUITests: XCTestCase {

    override func setUpWithError() throws {
        // Put setup code here. This method is called before the invocation of each test method in the class.

        // In UI tests it is usually best to stop immediately when a failure occurs.
        continueAfterFailure = false

        // In UI tests it’s important to set the initial state - such as interface orientation - required for your tests before they run. The setUp method is a good place to do this.
    }

    override func tearDownWithError() throws {
        // Put teardown code here. This method is called after the invocation of each test method in the class.
    }

    @MainActor
    func testExample() throws {
        // UI tests must launch the application that they test.
        let app = XCUIApplication()
        app.launch()

        // Use XCTAssert and related functions to verify your tests produce the correct results.
        tap(app, id: "list.append")
        tap(app, id: "list.insert")
        XCTAssertTrue(app.staticTexts["Summary"].waitForExistence(timeout: 2))
        let backButton = app.navigationBars.buttons.element(boundBy: 0)
        XCTAssertTrue(backButton.waitForExistence(timeout: 2))
        backButton.tap()
        tap(app, id: "list.update")
        tap(app, id: "list.move")
        tap(app, id: "list.sort")
        tap(app, id: "list.remove")

        tap(app, id: "counter.increment")
        tap(app, id: "counter.increment")
        tap(app, id: "counter.increment")

        XCTAssertTrue(app.staticTexts["Detail"].waitForExistence(timeout: 2))
    }

    @MainActor
    func testLaunchPerformance() throws {
        // This measures how long it takes to launch your application.
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
}
