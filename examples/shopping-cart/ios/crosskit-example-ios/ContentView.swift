import CrossKitShoppingCartShared
import Foundation
import SwiftUI

struct ContentView: View {
    @StateObject private var kit = CrossKitShoppingCartBridge()

    private var state: ShoppingCartState {
        kit.shoppingCart.state
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Shopping Cart")
                .font(.title.bold())
                .accessibilityIdentifier("cart.title")
            totals
            products
            cartItems
            actions
            if let error = state.lastError {
                Text(String(describing: error))
                    .font(.caption)
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("cart.error")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding()
    }

    private var totals: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 12) {
                Text("Items \(state.itemCount)")
                    .accessibilityIdentifier("cart.items.count")
                Text("Subtotal \(money(state.subtotalCents))")
                    .accessibilityIdentifier("cart.subtotal")
                Text("Discount \(money(state.discountCents))")
                    .accessibilityIdentifier("cart.discount")
            }
            HStack(spacing: 12) {
                Text("Tax \(money(state.taxCents))")
                    .accessibilityIdentifier("cart.tax")
                Text("Total \(money(state.totalCents))")
                    .accessibilityIdentifier("cart.total")
                Text(state.checkoutReady ? "Ready" : "Not ready")
                    .accessibilityIdentifier("cart.ready")
            }
        }
        .font(.subheadline)
    }

    private var products: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Products").font(.headline)
            ForEach(state.products, id: \.id) { product in
                HStack(spacing: 10) {
                    Text("\(product.name) \(money(product.priceCents)) stock \(product.stock)")
                        .accessibilityIdentifier("product.row.\(product.id)")
                    Spacer()
                    Button("Add") {
                        kit.cart.addProduct(productId: product.id, quantity: 1)
                    }
                    .accessibilityIdentifier("product.add.\(product.id)")
                }
            }
        }
    }

    private var cartItems: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Cart").font(.headline)
            ForEach(Array(kit.cart.items.enumerated()), id: \.element.productId) { index, item in
                HStack(spacing: 10) {
                    Text("\(item.name) x\(item.quantity) \(money(item.lineTotalCents))")
                        .accessibilityIdentifier("cart.position.\(index)")
                    Spacer()
                    Button("+") {
                        kit.cart.setQuantity(productId: item.productId, quantity: item.quantity + 1)
                    }
                    .accessibilityIdentifier("cart.more.\(item.productId)")
                    Button("Remove") {
                        kit.cart.removeProduct(productId: item.productId)
                    }
                    .accessibilityIdentifier("cart.remove.\(item.productId)")
                }
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("cart.row.\(item.productId)")
            }
        }
        .accessibilityIdentifier("cart.list")
    }

    private var actions: some View {
        HStack(spacing: 8) {
            Button("SAVE10") { kit.shoppingCart.applyCoupon(code: "SAVE10") }
                .accessibilityIdentifier("coupon.save10")
            Button("Bad coupon") { kit.shoppingCart.applyCoupon(code: "bogus") }
                .accessibilityIdentifier("coupon.bad")
            Button("Clear coupon") { kit.shoppingCart.clearCoupon() }
                .accessibilityIdentifier("coupon.clear")
            Button("Checkout") { kit.shoppingCart.checkout() }
                .accessibilityIdentifier("cart.checkout")
            Button("Clear cart") { kit.cart.clearCart() }
                .accessibilityIdentifier("cart.clear")
        }
        .buttonStyle(.borderedProminent)
    }

    private func money(_ cents: Int64) -> String {
        String(format: "$%lld.%02lld", cents / 100, abs(cents % 100))
    }
}

#Preview {
    ContentView()
}
