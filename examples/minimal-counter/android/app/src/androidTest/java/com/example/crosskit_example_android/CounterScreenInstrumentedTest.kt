package com.example.crosskit_example_android

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import androidx.test.ext.junit.runners.AndroidJUnit4

@RunWith(AndroidJUnit4::class)
class CounterScreenInstrumentedTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun buttonsUpdateCounterThroughGeneratedBridge() {
        composeRule.onNodeWithTag("counter.value").assertTextEquals("0")

        composeRule.onNodeWithTag("counter.increment").performClick()
        composeRule.onNodeWithTag("counter.value").assertTextEquals("1")

        composeRule.onNodeWithTag("counter.decrement").performClick()
        composeRule.onNodeWithTag("counter.value").assertTextEquals("0")

        composeRule.onNodeWithTag("counter.increment").performClick()
        composeRule.onNodeWithTag("counter.increment").performClick()
        composeRule.onNodeWithTag("counter.reset").performClick()
        composeRule.onNodeWithTag("counter.value").assertTextEquals("0")
    }
}
