package com.example.crosskit_example_android

import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextClearance
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SearchScreenInstrumentedTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun searchFlowUsesGeneratedBridgeState() {
        composeRule.onNodeWithTag("search.title").assertTextEquals("Search Refresh")
        composeRule.onNodeWithTag("search.progress").assertTextEquals("Progress 0%")
        composeRule.onNodeWithTag("search.cancel").assertIsNotEnabled()

        composeRule.onNodeWithTag("search.query").performTextInput("rust")
        composeRule.onNodeWithTag("search.submit").assertIsEnabled()
        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Loading")
        composeRule.onNodeWithTag("search.cancel").assertIsEnabled()

        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.progress").assertTextEquals("Progress 50%")

        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Idle")
        composeRule.onNodeWithTag("search.progress").assertTextEquals("Progress 100%")
        composeRule.onNodeWithTag("search.result.1.title").assertTextEquals("rust guide")

        composeRule.onNodeWithTag("search.query").performTextClearance()
        composeRule.onNodeWithTag("search.query").performTextInput("swift")
        composeRule.onAllNodesWithTag("search.result.1.title").assertCountEquals(0)
    }

    @Test
    fun errorCancelAndRetryUseGeneratedState() {
        composeRule.onNodeWithTag("search.query").performTextInput("network")
        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.error").assertTextContains("Network")

        composeRule.onNodeWithTag("search.query").performTextClearance()
        composeRule.onNodeWithTag("search.query").performTextInput("rust")
        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.cancel").performClick()
        composeRule.onNodeWithTag("search.error").assertTextContains("Cancelled")

        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.result.1.title").assertTextEquals("rust guide")
    }
}
