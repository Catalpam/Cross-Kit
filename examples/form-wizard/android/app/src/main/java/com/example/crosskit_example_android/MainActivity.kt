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
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crosskit.formwizard.shared.FormStep
import com.crosskit.formwizard.shared.FormWizardState
import com.crosskit.formwizard.shared.FormWizardViewModelBridge
import com.crosskit.formwizard.shared.rememberCrossKitFormWizardBridge
import com.example.crosskit_example_android.ui.theme.CrosskitexampleandroidTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            CrosskitexampleandroidTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background,
                    contentColor = MaterialTheme.colorScheme.onBackground
                ) {
                    CrossKitApp()
                }
            }
        }
    }
}

@Composable
fun CrossKitApp(modifier: Modifier = Modifier) {
    // The generated root container hides UniFFI setup and observer lifetimes.
    // App code keeps one remembered object and renders Rust-derived state.
    val kit = rememberCrossKitFormWizardBridge()
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        FormWizardScreen(
            wizard = kit.formWizard,
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(16.dp)
        )
    }
}

@Composable
private fun FormWizardScreen(wizard: FormWizardViewModelBridge, modifier: Modifier = Modifier) {
    // Validation, step routing, and button enablement are all part of Rust
    // state. Compose only binds text changes and button clicks to actions.
    val state = wizard.state
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text(
            text = "Form Wizard",
            modifier = Modifier.testTag("form.title"),
            style = MaterialTheme.typography.titleLarge
        )
        when (state.step) {
            FormStep.PROFILE -> ProfileStep(state, wizard)
            FormStep.SECURITY -> SecurityStep(state, wizard)
            FormStep.SUMMARY -> SummaryStep(state, wizard)
        }
    }
}

@Composable
private fun ProfileStep(state: FormWizardState, wizard: FormWizardViewModelBridge) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            text = "Profile",
            modifier = Modifier.testTag("form.step"),
            style = MaterialTheme.typography.titleMedium
        )
        OutlinedTextField(
            value = state.name,
            onValueChange = wizard::updateName,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("form.name"),
            label = { Text(text = "Name") },
            isError = state.nameError != null
        )
        FieldError(state.nameError, "form.name.error")
        OutlinedTextField(
            value = state.email,
            onValueChange = wizard::updateEmail,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("form.email"),
            label = { Text(text = "Email") },
            isError = state.emailError != null
        )
        FieldError(state.emailError, "form.email.error")
        Button(
            onClick = wizard::next,
            enabled = state.canGoNext,
            modifier = Modifier.testTag("form.next")
        ) {
            Text(text = "Next")
        }
    }
}

@Composable
private fun SecurityStep(state: FormWizardState, wizard: FormWizardViewModelBridge) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            text = "Security",
            modifier = Modifier.testTag("form.step"),
            style = MaterialTheme.typography.titleMedium
        )
        OutlinedTextField(
            value = state.password,
            onValueChange = wizard::updatePassword,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("form.password"),
            label = { Text(text = "Password") },
            isError = state.passwordError != null,
            visualTransformation = PasswordVisualTransformation()
        )
        FieldError(state.passwordError, "form.password.error")
        OutlinedTextField(
            value = state.confirmPassword,
            onValueChange = wizard::updateConfirm,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("form.confirm"),
            label = { Text(text = "Confirm password") },
            isError = state.confirmPasswordError != null,
            visualTransformation = PasswordVisualTransformation()
        )
        FieldError(state.confirmPasswordError, "form.confirm.error")
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = wizard::back, modifier = Modifier.testTag("form.back")) {
                Text(text = "Back")
            }
            Button(
                onClick = wizard::next,
                enabled = state.canGoNext,
                modifier = Modifier.testTag("form.next")
            ) {
                Text(text = "Next")
            }
        }
    }
}

@Composable
private fun SummaryStep(state: FormWizardState, wizard: FormWizardViewModelBridge) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            text = "Summary",
            modifier = Modifier.testTag("form.step"),
            style = MaterialTheme.typography.titleMedium
        )
        Text(
            text = state.summary,
            modifier = Modifier.testTag("form.summary"),
            style = MaterialTheme.typography.titleMedium
        )
        if (state.isComplete) {
            Text(
                text = "Complete",
                modifier = Modifier.testTag("form.complete"),
                style = MaterialTheme.typography.titleMedium
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = wizard::back, modifier = Modifier.testTag("form.back")) {
                Text(text = "Back")
            }
            Button(
                onClick = wizard::next,
                enabled = state.canGoNext,
                modifier = Modifier.testTag("form.submit")
            ) {
                Text(text = "Create account")
            }
        }
    }
}

@Composable
private fun FieldError(text: String?, tag: String) {
    if (text != null) {
        Text(
            text = text,
            modifier = Modifier.testTag(tag),
            color = MaterialTheme.colorScheme.error,
            style = MaterialTheme.typography.bodySmall
        )
    }
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    CrosskitexampleandroidTheme {
        CrossKitApp()
    }
}
