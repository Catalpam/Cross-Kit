package com.example.crosskit_example_android

import androidx.compose.ui.test.assertTextEquals
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
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Results")
        composeRule.onNodeWithTag("search.progress").assertTextEquals("Progress 100%")
        composeRule.onNodeWithTag("search.result.1.title").assertTextEquals("rust guide")

        composeRule.onNodeWithTag("search.query").performTextClearance()
        composeRule.onNodeWithTag("search.query").performTextInput("swift")
        composeRule.onAllNodesWithTag("search.result.1.title").assertCountEquals(0)
    }

    @Test
    fun noticeCancelAndRetryUseGeneratedState() {
        composeRule.onNodeWithTag("search.query").performTextInput("network")
        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Failed")
        composeRule.onNodeWithTag("search.notice").assertTextEquals("Search is temporarily unavailable.")
        composeRule.onNodeWithTag("search.retry").assertIsEnabled()

        composeRule.onNodeWithTag("search.retry").performClick()
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Loading")
        composeRule.onAllNodesWithTag("search.notice").assertCountEquals(0)
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Results")
        composeRule.onNodeWithTag("search.result.1.title").assertTextEquals("network guide")

        composeRule.onNodeWithTag("search.query").performTextClearance()
        composeRule.onNodeWithTag("search.query").performTextInput("rust")
        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.cancel").performClick()
        composeRule.onNodeWithTag("search.loading").assertTextEquals("Idle")
        composeRule.onAllNodesWithTag("search.notice").assertCountEquals(0)

        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.result.1.title").assertTextEquals("rust guide")
    }

    @Test
    fun emptyResultsUseGeneratedPresentationState() {
        composeRule.onNodeWithTag("search.query").performTextInput("empty")
        composeRule.onNodeWithTag("search.submit").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()
        composeRule.onNodeWithTag("search.tick").performClick()

        composeRule.onNodeWithTag("search.loading").assertTextEquals("Empty")
        composeRule.onNodeWithTag("search.empty").assertTextEquals("No results")
        composeRule.onNodeWithTag("search.notice").assertTextEquals("No results for \"empty\".")
    }
}
