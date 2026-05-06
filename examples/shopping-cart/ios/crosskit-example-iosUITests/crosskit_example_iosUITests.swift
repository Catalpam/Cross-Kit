import Foundation
import XCTest

final class crosskit_example_iosUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor
    func testShoppingCartUsesGeneratedBridgeState() throws {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.staticTexts["cart.title"].waitForExistence(timeout: 2))
        XCTAssertTrue(waitForLabel("Total $0.00", id: "cart.total", in: app))

        app.buttons["product.add.1"].tap()
        XCTAssertTrue(waitForLabel("Items 1", id: "cart.items.count", in: app))
        XCTAssertTrue(waitForLabel("Coffee x1 $12.99", id: "cart.position.0", in: app))
        XCTAssertTrue(waitForLabel("Total $14.06", id: "cart.total", in: app))

        app.buttons["product.add.1"].tap()
        XCTAssertTrue(waitForLabel("Items 2", id: "cart.items.count", in: app))
        XCTAssertTrue(waitForLabel("Coffee x2 $25.98", id: "cart.position.0", in: app))

        app.buttons["coupon.save10"].tap()
        XCTAssertTrue(waitForLabel("Discount $2.59", id: "cart.discount", in: app))
        XCTAssertTrue(waitForLabel("Total $25.32", id: "cart.total", in: app))

        app.buttons["coupon.bad"].tap()
        XCTAssertTrue(waitForLabelContaining("invalidCoupon", id: "cart.error", in: app))

        app.buttons["cart.more.1"].tap()
        XCTAssertTrue(waitForLabel("Items 3", id: "cart.items.count", in: app))

        app.buttons["cart.remove.1"].tap()
        XCTAssertTrue(waitForLabel("Items 0", id: "cart.items.count", in: app))
        XCTAssertTrue(waitForLabel("Total $0.00", id: "cart.total", in: app))

        app.buttons["product.add.3"].tap()
        app.buttons["product.add.3"].tap()
        app.buttons["product.add.3"].tap()
        XCTAssertTrue(waitForLabelContaining("outOfStock", id: "cart.error", in: app))
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
