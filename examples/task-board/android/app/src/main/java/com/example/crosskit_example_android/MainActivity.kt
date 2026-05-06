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
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crosskit.taskboard.shared.TaskBoardState
import com.crosskit.taskboard.shared.TaskFilter
import com.crosskit.taskboard.shared.TaskItem
import com.crosskit.taskboard.shared.TaskListViewModelBridge
import com.crosskit.taskboard.shared.rememberCrossKitTaskBoardBridge
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
    val kit = rememberCrossKitTaskBoardBridge()
    Scaffold(modifier = modifier.fillMaxSize()) { innerPadding ->
        TaskBoardScreen(
            state = kit.taskBoard.state,
            taskList = kit.taskList,
            onFilter = kit.taskBoard::setFilter,
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(16.dp)
        )
    }
}

@Composable
private fun TaskBoardScreen(
    state: TaskBoardState,
    taskList: TaskListViewModelBridge,
    onFilter: (TaskFilter) -> Unit,
    modifier: Modifier = Modifier
) {
    var draftTitle by remember { mutableStateOf("") }
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(
            text = "Task Board",
            modifier = Modifier.testTag("task.title"),
            style = MaterialTheme.typography.titleLarge
        )
        CounterRow(state)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(
                value = draftTitle,
                onValueChange = { draftTitle = it },
                modifier = Modifier
                    .weight(1f)
                    .testTag("task.input"),
                label = { Text(text = "Task title") }
            )
            Button(
                onClick = {
                    taskList.addTask(draftTitle)
                    draftTitle = ""
                },
                modifier = Modifier.testTag("task.add")
            ) {
                Text(text = "Add")
            }
        }
        FilterButtons(state.filter, onFilter)
        LazyColumn(
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f)
                .testTag("task.list"),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            itemsIndexed(taskList.items, key = { _, task -> task.id }) { index, task ->
                TaskRow(index, task, taskList)
            }
        }
        ActionButtons(state, taskList)
        state.lastError?.let { error ->
            Text(
                text = error,
                modifier = Modifier.testTag("task.error"),
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall
            )
        }
    }
}

@Composable
private fun CounterRow(state: TaskBoardState) {
    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(text = "Total ${state.totalCount}", modifier = Modifier.testTag("task.total"))
        Text(text = "Open ${state.openCount}", modifier = Modifier.testTag("task.open.count"))
        Text(text = "Done ${state.doneCount}", modifier = Modifier.testTag("task.done.count"))
    }
}

@Composable
private fun FilterButtons(active: TaskFilter, onFilter: (TaskFilter) -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(
            onClick = { onFilter(TaskFilter.ALL) },
            enabled = active != TaskFilter.ALL,
            modifier = Modifier.testTag("task.filter.all")
        ) {
            Text(text = "All")
        }
        Button(
            onClick = { onFilter(TaskFilter.OPEN) },
            enabled = active != TaskFilter.OPEN,
            modifier = Modifier.testTag("task.filter.open")
        ) {
            Text(text = "Open")
        }
        Button(
            onClick = { onFilter(TaskFilter.DONE) },
            enabled = active != TaskFilter.DONE,
            modifier = Modifier.testTag("task.filter.done")
        ) {
            Text(text = "Done")
        }
    }
}

@Composable
private fun TaskRow(index: Int, task: TaskItem, taskList: TaskListViewModelBridge) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .testTag("task.row.${task.id}"),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Text(text = if (task.done) "Done" else "Open")
        Text(
            text = task.title,
            modifier = Modifier
                .weight(1f)
                .testTag("task.position.$index")
        )
        Button(
            onClick = { taskList.toggleDone(task.id) },
            modifier = Modifier.testTag("task.toggle.${task.id}")
        ) {
            Text(text = if (task.done) "Open" else "Done")
        }
        Button(
            onClick = { taskList.deleteTask(task.id) },
            modifier = Modifier.testTag("task.delete.${task.id}")
        ) {
            Text(text = "Delete")
        }
    }
}

@Composable
private fun ActionButtons(state: TaskBoardState, taskList: TaskListViewModelBridge) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = taskList::addSampleBatch, modifier = Modifier.testTag("task.sample")) {
            Text(text = "Sample")
        }
        Button(
            onClick = { taskList.moveVisible(0, (taskList.items.size - 1).toLong()) },
            enabled = taskList.items.size >= 2,
            modifier = Modifier.testTag("task.move")
        ) {
            Text(text = "Move")
        }
        Button(
            onClick = {
                taskList.items.firstOrNull()?.let { task ->
                    taskList.renameTask(task.id, "Renamed")
                }
            },
            enabled = taskList.items.isNotEmpty(),
            modifier = Modifier.testTag("task.rename")
        ) {
            Text(text = "Rename")
        }
        Button(
            onClick = taskList::clearDone,
            enabled = state.canClearDone,
            modifier = Modifier.testTag("task.clear.done")
        ) {
            Text(text = "Clear done")
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
