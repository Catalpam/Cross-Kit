package com.example.crosskit_example_android

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class ExampleInstrumentedTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun appLaunchesAndUsesGeneratedBridges() {
        composeRule.onNodeWithText("Counter: 0").assertIsDisplayed()
        composeRule.onNodeWithText("List (len=0)").assertIsDisplayed()
        composeRule.onNodeWithText("+1").assertIsDisplayed().performClick()
        composeRule.onNodeWithText("Counter: 1").assertIsDisplayed()

        composeRule.onNodeWithText("Add").assertIsDisplayed().performClick()
        composeRule.onNodeWithText("List (len=1)").assertIsDisplayed()
    }
}
