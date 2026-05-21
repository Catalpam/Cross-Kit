import Combine
import CrossKitShoppingCartShared
import XCTest

@MainActor
final class ShoppingCartBridgeTests: XCTestCase {
    func testRootContainerStartsWithCatalogAndEmptyCart() {
        let kit = CrossKitShoppingCartBridge()

        XCTAssertEqual(kit.shoppingCart.state.products.count, 3)
        XCTAssertEqual(kit.shoppingCart.state.products[0].name, "Coffee")
        XCTAssertEqual(kit.shoppingCart.state.itemCount, 0)
        XCTAssertEqual(kit.shoppingCart.state.totalCents, 0)
        XCTAssertTrue(kit.cart.items.isEmpty)
    }

    func testCartMergeCouponAndClearFlow() async {
        let kit = CrossKitShoppingCartBridge()

        kit.cart.addProduct(productId: 1, quantity: 1)
        var updated = await waitFor { kit.cart.items.count == 1 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.shoppingCart.state.subtotalCents, 1299)
        XCTAssertEqual(kit.shoppingCart.state.totalCents, 1406)

        kit.cart.addProduct(productId: 1, quantity: 2)
        updated = await waitFor { kit.cart.items.first?.quantity == 3 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.shoppingCart.state.itemCount, 3)

        kit.shoppingCart.applyCoupon(code: "save10")
        updated = await waitFor { kit.shoppingCart.state.discountCents == 389 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.shoppingCart.state.totalCents, 3797)

        kit.cart.setQuantity(productId: 1, quantity: 5)
        updated = await waitFor { kit.cart.items.first?.quantity == 5 }
        XCTAssertTrue(updated)

        kit.cart.clearCart()
        updated = await waitFor { kit.cart.items.isEmpty }
        XCTAssertTrue(updated)
        XCTAssertNil(kit.shoppingCart.state.couponCode)
        XCTAssertFalse(kit.shoppingCart.state.checkoutEnabled)
        XCTAssertNil(kit.shoppingCart.state.checkoutNotice)
    }

    func testPresentationNoticesStayRustOwned() async {
        let kit = CrossKitShoppingCartBridge()

        kit.cart.addProduct(productId: 3, quantity: 3)
        var updated = await waitFor { self.inlineMessage(kit.shoppingCart.state.checkoutNotice) == "Only 2 left in stock." }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.shoppingCart.state.stockWarnings.first?.message, "Requested 3, only 2 in stock.")
        XCTAssertFalse(kit.shoppingCart.state.checkoutEnabled)

        kit.cart.addProduct(productId: 1, quantity: 1)
        _ = await waitFor { kit.cart.items.count == 1 }
        kit.shoppingCart.applyCoupon(code: "bogus")
        updated = await waitFor { self.toastMessage(kit.shoppingCart.state.checkoutNotice) == "Coupon BOGUS is not valid." }
        XCTAssertTrue(updated)

        kit.cart.setQuantity(productId: 1, quantity: 0)
        updated = await waitFor { self.inlineMessage(kit.shoppingCart.state.checkoutNotice) == "Quantity must be positive." }
        XCTAssertTrue(updated)

        kit.cart.addProduct(productId: 999, quantity: 1)
        updated = await waitFor { self.inlineMessage(kit.shoppingCart.state.checkoutNotice) == "Product 999 is no longer available." }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.shoppingCart.state.itemCount, 1)
    }

    func testCouponRecomputesAndCheckoutClearsTransientError() async {
        let kit = CrossKitShoppingCartBridge()

        kit.cart.addProduct(productId: 1, quantity: 1)
        _ = await waitFor { kit.shoppingCart.state.itemCount == 1 }
        kit.shoppingCart.applyCoupon(code: "SAVE10")
        _ = await waitFor { kit.shoppingCart.state.discountCents == 129 }

        kit.cart.addProduct(productId: 1, quantity: 1)
        var updated = await waitFor { kit.shoppingCart.state.discountCents == 259 }
        XCTAssertTrue(updated)
        XCTAssertEqual(kit.shoppingCart.state.totalCents, 2532)

        kit.shoppingCart.applyCoupon(code: "bogus")
        updated = await waitFor { self.toastMessage(kit.shoppingCart.state.checkoutNotice) == "Coupon BOGUS is not valid." }
        XCTAssertTrue(updated)
        XCTAssertTrue(kit.shoppingCart.state.checkoutEnabled)

        kit.shoppingCart.checkout()
        updated = await waitFor { kit.shoppingCart.state.checkoutNotice == nil }
        XCTAssertTrue(updated)
        XCTAssertTrue(kit.shoppingCart.state.checkoutEnabled)
        XCTAssertNil(kit.shoppingCart.state.checkoutNotice)
    }

    func testRootContainerForwardsCartChanges() async {
        let kit = CrossKitShoppingCartBridge()
        var forwardedChanges = 0
        let cancellable = kit.objectWillChange.sink {
            forwardedChanges += 1
        }

        kit.cart.addProduct(productId: 1, quantity: 1)
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

    private func inlineMessage(_ notice: CartNotice?) -> String? {
        guard let notice else { return nil }
        if case let .inline(message) = notice {
            return message
        }
        return nil
    }

    private func toastMessage(_ notice: CartNotice?) -> String? {
        guard let notice else { return nil }
        if case let .toast(message) = notice {
            return message
        }
        return nil
    }
}
