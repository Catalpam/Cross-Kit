package com.example.crosskit_example_android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crosskit.minimalcounter.shared.CounterViewModelBridge
import com.crosskit.minimalcounter.shared.rememberCrossKitMinimalCounterBridge
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
    val kit = rememberCrossKitMinimalCounterBridge(initial = 0)
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        CounterScreen(
            counter = kit.counter,
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(16.dp)
        )
    }
}

@Composable
private fun CounterScreen(counter: CounterViewModelBridge, modifier: Modifier = Modifier) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(text = "Minimal Counter", style = MaterialTheme.typography.titleLarge)
        Text(
            text = counter.state.value.toString(),
            modifier = Modifier.testTag("counter.value"),
            style = MaterialTheme.typography.displayMedium
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = { counter.decrement() },
                modifier = Modifier.testTag("counter.decrement")
            ) {
                Text(text = "-1")
            }
            Button(
                onClick = { counter.reset() },
                modifier = Modifier.testTag("counter.reset")
            ) {
                Text(text = "Reset")
            }
            Button(
                onClick = { counter.increment() },
                modifier = Modifier.testTag("counter.increment")
            ) {
                Text(text = "+1")
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    CrosskitexampleandroidTheme {
        CrossKitApp()
    }
}
