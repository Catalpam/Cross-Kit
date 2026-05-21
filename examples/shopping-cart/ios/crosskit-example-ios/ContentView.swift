import CrossKitShoppingCartShared
import Foundation
import SwiftUI

struct ContentView: View {
    // One generated container exposes both the summary state VM and the cart
    // diff-list VM. The app does not manually wire parent/child bridges.
    @StateObject private var kit = CrossKitShoppingCartBridge()

    private var state: ShoppingCartState {
        // Totals, stock warnings, coupon state, and checkout affordances are
        // derived in Rust so iOS and Android render the same business rules.
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
            notice
            stockWarnings
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
                Text(state.checkoutEnabled ? "Ready" : "Not ready")
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
                        // Intent-style call: Rust validates stock, merges
                        // existing rows, emits cart diffs, and recomputes totals.
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
                .disabled(!state.checkoutEnabled)
                .accessibilityIdentifier("cart.checkout")
            Button("Clear cart") { kit.cart.clearCart() }
                .accessibilityIdentifier("cart.clear")
        }
        .buttonStyle(.borderedProminent)
    }

    @ViewBuilder
    private var notice: some View {
        if let notice = state.checkoutNotice {
            Text(noticeText(notice))
                .font(.caption)
                .foregroundStyle(noticeColor(notice))
                .accessibilityIdentifier("cart.notice")
        }
    }

    private var stockWarnings: some View {
        VStack(alignment: .leading, spacing: 4) {
            ForEach(state.stockWarnings, id: \.productId) { warning in
                Text(warning.message)
                    .font(.caption)
                    .foregroundStyle(.orange)
                    .accessibilityIdentifier("cart.stock.warning.\(warning.productId)")
            }
        }
    }

    private func money(_ cents: Int64) -> String {
        String(format: "$%lld.%02lld", cents / 100, abs(cents % 100))
    }

    private func noticeText(_ notice: CartNotice) -> String {
        switch notice {
        case let .inline(message):
            return message
        case let .toast(message):
            return message
        case let .dialog(title, message):
            return "\(title): \(message)"
        }
    }

    private func noticeColor(_ notice: CartNotice) -> Color {
        switch notice {
        case .inline:
            return .red
        case .toast:
            return .orange
        case .dialog:
            return .blue
        }
    }
}

#Preview {
    ContentView()
}
