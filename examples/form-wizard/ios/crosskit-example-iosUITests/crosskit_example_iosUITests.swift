import XCTest

final class crosskit_example_iosUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testFormWizardCompletesThroughGeneratedBridge() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["form.title"].waitForExistence(timeout: 2))
        XCTAssertEqual(app.staticTexts["form.step"].label, "Profile")
        XCTAssertEqual(app.staticTexts["form.name.error"].label, "Name must be at least 2 characters")
        XCTAssertEqual(app.staticTexts["form.email.error"].label, "Email is required")
        XCTAssertFalse(app.buttons["form.next"].isEnabled)

        app.textFields["form.name"].tap()
        app.textFields["form.name"].typeText("Ada Lovelace")
        app.textFields["form.email"].tap()
        app.textFields["form.email"].typeText("ada@example.com")
        XCTAssertTrue(app.buttons["form.next"].isEnabled)
        app.buttons["form.next"].tap()

        XCTAssertTrue(app.staticTexts["form.step"].waitForExistence(timeout: 2))
        XCTAssertEqual(app.staticTexts["form.step"].label, "Security")
        app.secureTextFields["form.password"].tap()
        app.secureTextFields["form.password"].typeText("password1")
        app.secureTextFields["form.confirm"].tap()
        app.secureTextFields["form.confirm"].typeText("password2")
        XCTAssertTrue(app.staticTexts["form.confirm.error"].waitForExistence(timeout: 2))
        XCTAssertFalse(app.buttons["form.next"].isEnabled)

        app.secureTextFields["form.confirm"].tap()
        app.secureTextFields["form.confirm"].typeText(String(repeating: XCUIKeyboardKey.delete.rawValue, count: 9))
        app.secureTextFields["form.confirm"].typeText("password1")
        XCTAssertTrue(app.buttons["form.next"].isEnabled)
        app.buttons["form.next"].tap()

        XCTAssertTrue(app.staticTexts["form.summary"].waitForExistence(timeout: 2))
        XCTAssertEqual(app.staticTexts["form.summary"].label, "Ada Lovelace <ada@example.com>")
        app.buttons["form.submit"].tap()
        XCTAssertTrue(app.staticTexts["form.complete"].waitForExistence(timeout: 2))
    }

    @MainActor
    func testLaunchPerformance() throws {
        measure(metrics: [XCTApplicationLaunchMetric()]) {
            XCUIApplication().launch()
        }
    }
}
