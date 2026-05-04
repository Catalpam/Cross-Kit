package com.example.crosskit_example_android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crosskit.shared.AppState
import com.crosskit.shared.AppViewModelBridge
import com.crosskit.shared.CounterViewModelBridge
import com.crosskit.shared.ListItem
import com.crosskit.shared.ListViewModelBridge
import com.crosskit.shared.Route
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
    val appVm = remember { AppViewModelBridge(initial = 0) }
    val counterVm = remember(appVm) { appVm.makeCounterVm() }
    val listVm = remember(appVm) { appVm.makeListVm() }

    DisposableEffect(Unit) {
        onDispose {
            listVm.close()
            counterVm.close()
            appVm.close()
        }
    }

    val state = appVm.state
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        val contentModifier = Modifier
            .fillMaxSize()
            .padding(innerPadding)
            .padding(16.dp)
        when (val route = state.route) {
            is Route.ListDetail -> ListDetailScreen(route, onBack = appVm::clearRoute, modifier = contentModifier)
            is Route.Summary -> SummaryScreen(state, onBack = appVm::clearRoute, modifier = contentModifier)
            null -> HomeScreen(counterVm, listVm, state, modifier = contentModifier)
        }
    }
}

@Composable
private fun HomeScreen(
    counterVm: CounterViewModelBridge,
    listVm: ListViewModelBridge,
    state: AppState,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(16.dp)) {
        CounterSection(counterVm)
        ListSection(listVm, state)
    }
}

@Composable
private fun CounterSection(counterVm: CounterViewModelBridge) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "Counter: ${counterVm.state.value}", style = MaterialTheme.typography.titleMedium)
        Button(onClick = { counterVm.increment() }) {
            Text(text = "+1")
        }
    }
}

@Composable
private fun ListSection(listVm: ListViewModelBridge, state: AppState) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "List (len=${state.listLen})", style = MaterialTheme.typography.titleMedium)
        LazyColumn(modifier = Modifier.fillMaxWidth().height(200.dp)) {
            items(listVm.items, key = { it.id }) { item ->
                ListRow(item)
            }
        }
        ListButtons(listVm)
    }
}

@Composable
private fun ListRow(item: ListItem) {
    Column(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
        Text(text = "id=${item.id}  ts=${item.timestampMs}")
        Text(text = item.dateCn, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
private fun ListButtons(listVm: ListViewModelBridge) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { listVm.appendNow() }) { Text(text = "Add") }
            Button(onClick = { listVm.insertNow(0) }) { Text(text = "Insert") }
            Button(onClick = { updateFirstItem(listVm) }) { Text(text = "Update") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { moveLastToFirst(listVm) }) { Text(text = "Move") }
            Button(onClick = { listVm.sortByTimestampDesc() }) { Text(text = "Sort") }
            Button(onClick = { removeLastItem(listVm) }) { Text(text = "Remove") }
        }
    }
}

private fun updateFirstItem(listVm: ListViewModelBridge) {
    if (listVm.items.isEmpty()) return
    val timestampMs = System.currentTimeMillis()
    listVm.updateWithTimestamp(0, timestampMs)
}

private fun moveLastToFirst(listVm: ListViewModelBridge) {
    val count = listVm.items.size
    if (count <= 1) return
    listVm.moveItem((count - 1).toLong(), 0)
}

private fun removeLastItem(listVm: ListViewModelBridge) {
    val count = listVm.items.size
    if (count == 0) return
    listVm.removeAt((count - 1).toLong())
}

@Composable
private fun SummaryScreen(state: AppState, onBack: () -> Unit, modifier: Modifier = Modifier) {
    BackHandler { onBack() }
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "Summary", style = MaterialTheme.typography.titleLarge)
        Text(text = "Counter: ${state.counter.value}")
        Text(text = "List length: ${state.listLen}")
        val lastItem = state.lastItem
        Text(text = "Last item: ${lastItem?.dateCn ?: "None"}")
        Spacer(modifier = Modifier.height(8.dp))
        Button(onClick = onBack) { Text(text = "Back") }
    }
}

@Composable
private fun ListDetailScreen(route: Route.ListDetail, onBack: () -> Unit, modifier: Modifier = Modifier) {
    BackHandler { onBack() }
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(text = "Detail", style = MaterialTheme.typography.titleLarge)
        Text(text = "id=${route.id}")
        Text(text = route.dateCn)
        Spacer(modifier = Modifier.height(8.dp))
        Button(onClick = onBack) { Text(text = "Back") }
    }
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    CrosskitexampleandroidTheme {
        CrossKitApp()
    }
}
