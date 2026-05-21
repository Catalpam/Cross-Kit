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
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crosskit.searchrefresh.shared.SearchNotice
import com.crosskit.searchrefresh.shared.SearchState
import com.crosskit.searchrefresh.shared.SearchStatus
import com.crosskit.searchrefresh.shared.SearchViewModelBridge
import com.crosskit.searchrefresh.shared.rememberCrossKitSearchRefreshBridge
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
    // Even this long-operation example stays state-driven: the generated bridge
    // exposes synchronous actions and observable state, not Kotlin suspend APIs.
    val kit = rememberCrossKitSearchRefreshBridge()
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        SearchScreen(
            search = kit.search,
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(16.dp)
        )
    }
}

@Composable
private fun SearchScreen(search: SearchViewModelBridge, modifier: Modifier = Modifier) {
    // Loading, progress, notices, and stale-result protection come from Rust.
    // The UI just renders fields and sends user intents back.
    val state = search.state
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            text = "Search Refresh",
            modifier = Modifier.testTag("search.title"),
            style = MaterialTheme.typography.titleLarge
        )
        OutlinedTextField(
            value = state.query,
            onValueChange = search::updateQuery,
            modifier = Modifier
                .fillMaxWidth()
                .testTag("search.query"),
            label = { Text(text = "Query") }
        )
        Controls(search, state)
        Progress(state)
        Notice(state.notice)
        Results(state)
    }
}

@Composable
private fun Controls(search: SearchViewModelBridge, state: SearchState) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                enabled = state.canSubmit,
                onClick = search::submit,
                modifier = Modifier.testTag("search.submit")
            ) {
                Text(text = "Submit")
            }
            Button(
                enabled = state.canRetry,
                onClick = search::submit,
                modifier = Modifier.testTag("search.retry")
            ) {
                Text(text = "Retry")
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                enabled = state.canCancel,
                onClick = search::tick,
                modifier = Modifier.testTag("search.tick")
            ) {
                Text(text = "Tick")
            }
            Button(
                enabled = state.canCancel,
                onClick = search::cancel,
                modifier = Modifier.testTag("search.cancel")
            ) {
                Text(text = "Cancel")
            }
        }
    }
}

@Composable
private fun Progress(state: SearchState) {
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            text = statusLabel(state.status),
            modifier = Modifier.testTag("search.loading")
        )
        Text(
            text = "Progress ${state.progress}%",
            modifier = Modifier.testTag("search.progress")
        )
    }
}

@Composable
private fun Notice(notice: SearchNotice?) {
    notice ?: return
    Text(
        text = noticeText(notice),
        modifier = Modifier.testTag("search.notice"),
        color = when (notice) {
            is SearchNotice.Inline -> MaterialTheme.colorScheme.error
            is SearchNotice.Toast -> MaterialTheme.colorScheme.tertiary
            is SearchNotice.Dialog -> MaterialTheme.colorScheme.primary
        },
        style = MaterialTheme.typography.bodySmall
    )
}

@Composable
private fun Results(state: SearchState) {
    LazyColumn(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("search.results"),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        if (state.status == SearchStatus.EMPTY) {
            item {
                Text(
                    text = "No results",
                    modifier = Modifier.testTag("search.empty"),
                    style = MaterialTheme.typography.bodySmall
                )
            }
        }
        items(state.results, key = { result -> result.rank }) { result ->
            Column {
                Text(
                    text = result.title,
                    modifier = Modifier.testTag("search.result.${result.rank}.title"),
                    style = MaterialTheme.typography.bodyLarge
                )
                Text(
                    text = result.snippet,
                    modifier = Modifier.testTag("search.result.${result.rank}.snippet"),
                    style = MaterialTheme.typography.bodySmall
                )
            }
        }
    }
}

private fun statusLabel(status: SearchStatus): String =
    when (status) {
        SearchStatus.IDLE -> "Idle"
        SearchStatus.LOADING -> "Loading"
        SearchStatus.RESULTS -> "Results"
        SearchStatus.EMPTY -> "Empty"
        SearchStatus.FAILED -> "Failed"
    }

private fun noticeText(notice: SearchNotice): String =
    when (notice) {
        is SearchNotice.Inline -> notice.message
        is SearchNotice.Toast -> notice.message
        is SearchNotice.Dialog -> "${notice.title}: ${notice.message}"
    }

@Preview(showBackground = true)
@Composable
fun SearchRefreshPreview() {
    CrosskitexampleandroidTheme {
        CrossKitApp()
    }
}
