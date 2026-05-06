package com.example.crosskit_example_android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crosskit.shoppingcart.shared.CartItem
import com.crosskit.shoppingcart.shared.CartViewModelBridge
import com.crosskit.shoppingcart.shared.Product
import com.crosskit.shoppingcart.shared.ShoppingCartState
import com.crosskit.shoppingcart.shared.rememberCrossKitShoppingCartBridge
import com.example.crosskit_example_android.ui.theme.CrosskitexampleandroidTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            CrosskitexampleandroidTheme {
                CrossKitApp()
            }
        }
    }
}

@Composable
fun CrossKitApp(modifier: Modifier = Modifier) {
    val kit = rememberCrossKitShoppingCartBridge()
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        ShoppingCartScreen(
            state = kit.shoppingCart.state,
            cart = kit.cart,
            onApplyCoupon = kit.shoppingCart::applyCoupon,
            onClearCoupon = kit.shoppingCart::clearCoupon,
            onCheckout = kit.shoppingCart::checkout,
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(16.dp)
        )
    }
}

@Composable
private fun ShoppingCartScreen(
    state: ShoppingCartState,
    cart: CartViewModelBridge,
    onApplyCoupon: (String) -> Unit,
    onClearCoupon: () -> Unit,
    onCheckout: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            text = "Shopping Cart",
            modifier = Modifier.testTag("cart.title"),
            style = MaterialTheme.typography.titleLarge
        )
        Totals(state)
        ProductList(state.products, cart)
        CartList(cart)
        Actions(cart, onApplyCoupon, onClearCoupon, onCheckout)
        state.lastError?.let { error ->
            Text(
                text = error.toString(),
                modifier = Modifier.testTag("cart.error"),
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall
            )
        }
    }
}

@Composable
private fun Totals(state: ShoppingCartState) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(text = "Items ${state.itemCount}", modifier = Modifier.testTag("cart.items.count"))
            Text(text = "Subtotal ${money(state.subtotalCents)}", modifier = Modifier.testTag("cart.subtotal"))
            Text(text = "Discount ${money(state.discountCents)}", modifier = Modifier.testTag("cart.discount"))
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(text = "Tax ${money(state.taxCents)}", modifier = Modifier.testTag("cart.tax"))
            Text(text = "Total ${money(state.totalCents)}", modifier = Modifier.testTag("cart.total"))
            Text(
                text = if (state.checkoutReady) "Ready" else "Not ready",
                modifier = Modifier.testTag("cart.ready")
            )
        }
    }
}

@Composable
private fun ProductList(products: List<Product>, cart: CartViewModelBridge) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "Products", style = MaterialTheme.typography.titleMedium)
        products.forEach { product ->
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(
                    text = "${product.name} ${money(product.priceCents)} stock ${product.stock}",
                    modifier = Modifier
                        .weight(1f)
                        .testTag("product.row.${product.id}")
                )
                Button(
                    onClick = { cart.addProduct(product.id, 1) },
                    modifier = Modifier.testTag("product.add.${product.id}")
                ) {
                    Text(text = "Add")
                }
            }
        }
    }
}

@Composable
private fun CartList(cart: CartViewModelBridge) {
    LazyColumn(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("cart.list"),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        itemsIndexed(cart.items, key = { _, item -> item.productId }) { index, item ->
            CartRow(index, item, cart)
        }
    }
}

@Composable
private fun CartRow(index: Int, item: CartItem, cart: CartViewModelBridge) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("cart.row.${item.productId}"),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Text(
            text = "${item.name} x${item.quantity} ${money(item.lineTotalCents)}",
            modifier = Modifier
                .weight(1f)
                .testTag("cart.position.$index")
        )
        Button(
            onClick = { cart.setQuantity(item.productId, item.quantity + 1) },
            modifier = Modifier.testTag("cart.more.${item.productId}")
        ) {
            Text(text = "+")
        }
        Button(
            onClick = { cart.removeProduct(item.productId) },
            modifier = Modifier.testTag("cart.remove.${item.productId}")
        ) {
            Text(text = "Remove")
        }
    }
}

@Composable
private fun Actions(
    cart: CartViewModelBridge,
    onApplyCoupon: (String) -> Unit,
    onClearCoupon: () -> Unit,
    onCheckout: () -> Unit
) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = { onApplyCoupon("SAVE10") }, modifier = Modifier.testTag("coupon.save10")) {
            Text(text = "SAVE10")
        }
        Button(onClick = { onApplyCoupon("bogus") }, modifier = Modifier.testTag("coupon.bad")) {
            Text(text = "Bad coupon")
        }
        Button(onClick = onClearCoupon, modifier = Modifier.testTag("coupon.clear")) {
            Text(text = "Clear coupon")
        }
        Button(onClick = onCheckout, modifier = Modifier.testTag("cart.checkout")) {
            Text(text = "Checkout")
        }
        Button(onClick = cart::clearCart, modifier = Modifier.testTag("cart.clear")) {
            Text(text = "Clear cart")
        }
    }
}

private fun money(cents: Long): String {
    val sign = if (cents < 0) "-" else ""
    val abs = kotlin.math.abs(cents)
    return "$sign\$${abs / 100}.${(abs % 100).toString().padStart(2, '0')}"
}

@Preview(showBackground = true)
@Composable
fun ShoppingCartPreview() {
    CrosskitexampleandroidTheme {
        CrossKitApp()
    }
}
