package com.example.crosskit_example_android

import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextClearance
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class FormWizardInstrumentedTest {
    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun formCompletesThroughGeneratedBridge() {
        composeRule.onNodeWithTag("form.title").assertTextEquals("Form Wizard")
        composeRule.onNodeWithTag("form.step").assertTextEquals("Profile")
        composeRule.onNodeWithTag("form.name.error")
            .assertTextEquals("Name must be at least 2 characters")
        composeRule.onNodeWithTag("form.email.error").assertTextEquals("Email is required")
        composeRule.onNodeWithTag("form.next").assertIsNotEnabled()

        composeRule.onNodeWithTag("form.name").performTextInput("Ada Lovelace")
        composeRule.onNodeWithTag("form.email").performTextInput("ada@example.com")
        composeRule.onNodeWithTag("form.next").assertIsEnabled().performClick()

        composeRule.onNodeWithTag("form.step").assertTextEquals("Security")
        composeRule.onNodeWithTag("form.password").performTextInput("password1")
        composeRule.onNodeWithTag("form.confirm").performTextInput("password2")
        composeRule.onNodeWithTag("form.confirm.error").assertTextContains("Passwords must match")
        composeRule.onNodeWithTag("form.next").assertIsNotEnabled()

        composeRule.onNodeWithTag("form.confirm").performTextClearance()
        composeRule.onNodeWithTag("form.confirm").performTextInput("password1")
        composeRule.onNodeWithTag("form.next").assertIsEnabled().performClick()

        composeRule.onNodeWithTag("form.summary")
            .assertTextEquals("Ada Lovelace <ada@example.com>")
        composeRule.onNodeWithTag("form.submit").assertIsEnabled().performClick()
        composeRule.onNodeWithTag("form.complete").assertTextEquals("Complete")
    }
}
