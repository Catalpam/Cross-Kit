package com.example.crosskit_example_android

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ShoppingCartInstrumentedTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun shoppingCartUsesGeneratedBridgeState() {
        composeRule.onNodeWithTag("cart.title").assertTextEquals("Shopping Cart")
        composeRule.onNodeWithTag("cart.total").assertTextEquals("Total \$0.00")

        composeRule.onNodeWithTag("product.add.1").performClick()
        composeRule.onNodeWithTag("cart.items.count").assertTextEquals("Items 1")
        composeRule.onNodeWithTag("cart.position.0").assertTextEquals("Coffee x1 \$12.99")
        composeRule.onNodeWithTag("cart.total").assertTextEquals("Total \$14.06")

        composeRule.onNodeWithTag("product.add.1").performClick()
        composeRule.onNodeWithTag("cart.items.count").assertTextEquals("Items 2")
        composeRule.onNodeWithTag("cart.position.0").assertTextEquals("Coffee x2 \$25.98")

        composeRule.onNodeWithTag("coupon.save10").performClick()
        composeRule.onNodeWithTag("cart.discount").assertTextEquals("Discount \$2.59")
        composeRule.onNodeWithTag("cart.total").assertTextEquals("Total \$25.32")

        composeRule.onNodeWithTag("coupon.bad").performClick()
        composeRule.onNodeWithTag("cart.error").assertTextContains("InvalidCoupon")

        composeRule.onNodeWithTag("cart.more.1").performClick()
        composeRule.onNodeWithTag("cart.items.count").assertTextEquals("Items 3")

        composeRule.onNodeWithTag("cart.remove.1").performClick()
        composeRule.onNodeWithTag("cart.items.count").assertTextEquals("Items 0")
        composeRule.onNodeWithTag("cart.total").assertTextEquals("Total \$0.00")

        composeRule.onNodeWithTag("product.add.3").performClick()
        composeRule.onNodeWithTag("product.add.3").performClick()
        composeRule.onNodeWithTag("product.add.3").performClick()
        composeRule.onNodeWithTag("cart.error").assertTextContains("OutOfStock")
    }
}
