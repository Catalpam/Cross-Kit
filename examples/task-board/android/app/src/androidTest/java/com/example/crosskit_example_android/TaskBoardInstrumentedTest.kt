package com.example.crosskit_example_android

import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import androidx.test.ext.junit.runners.AndroidJUnit4

@RunWith(AndroidJUnit4::class)
class TaskBoardInstrumentedTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun taskBoardUsesGeneratedBridgeState() {
        composeRule.onNodeWithTag("task.title").assertTextEquals("Task Board")
        composeRule.onNodeWithTag("task.total").assertTextEquals("Total 0")

        composeRule.onNodeWithTag("task.sample").performClick()
        composeRule.onNodeWithTag("task.position.0").assertTextEquals("Plan")
        composeRule.onNodeWithTag("task.open.count").assertTextEquals("Open 3")

        composeRule.onNodeWithTag("task.toggle.1").performClick()
        composeRule.onNodeWithTag("task.done.count").assertTextEquals("Done 1")

        composeRule.onNodeWithTag("task.filter.done").performClick()
        composeRule.onNodeWithTag("task.position.0").assertTextEquals("Plan")
        composeRule.onAllNodesWithTag("task.row.2").assertCountEquals(0)

        composeRule.onNodeWithTag("task.filter.all").performClick()
        composeRule.onNodeWithTag("task.position.2").assertTextEquals("Review")
        composeRule.onNodeWithTag("task.move").performClick()
        composeRule.onNodeWithTag("task.position.2").assertTextEquals("Plan")
        composeRule.onNodeWithTag("task.rename").performClick()
        composeRule.onNodeWithTag("task.position.0").assertTextEquals("Renamed")

        composeRule.onNodeWithTag("task.clear.done").performClick()
        composeRule.onNodeWithTag("task.total").assertTextEquals("Total 2")
    }
}
