import CrossKitTaskBoardShared
import SwiftUI

struct ContentView: View {
    // The generated container keeps `taskBoard` state and `taskList` diffs in
    // sync with the same Rust store.
    @StateObject private var kit = CrossKitTaskBoardBridge()
    @State private var draftTitle = ""

    private var state: TaskBoardState {
        // Counters, filter state, and validation errors are derived in Rust; the
        // Swift view only renders them and sends user intents back.
        kit.taskBoard.state
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Task Board")
                .font(.title.bold())
                .accessibilityIdentifier("task.title")
            counters
            composer
            filters
            taskList
            actions
            if let error = state.lastError {
                Text(error)
                    .font(.caption)
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("task.error")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding()
    }

    private var counters: some View {
        HStack(spacing: 12) {
            Text("Total \(state.totalCount)")
                .accessibilityIdentifier("task.total")
            Text("Open \(state.openCount)")
                .accessibilityIdentifier("task.open.count")
            Text("Done \(state.doneCount)")
                .accessibilityIdentifier("task.done.count")
        }
        .font(.subheadline)
    }

    private var composer: some View {
        HStack(spacing: 8) {
            TextField("Task title", text: $draftTitle)
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("task.input")
            Button("Add") {
                // Rust trims and validates the title, assigns stable ids, and
                // emits the diff needed by `kit.taskList.items`.
                kit.taskList.addTask(title: draftTitle)
                draftTitle = ""
            }
            .accessibilityIdentifier("task.add")
        }
    }

    private var filters: some View {
        HStack(spacing: 8) {
            filterButton("All", filter: .all, id: "task.filter.all")
            filterButton("Open", filter: .open, id: "task.filter.open")
            filterButton("Done", filter: .done, id: "task.filter.done")
        }
        .buttonStyle(.bordered)
    }

    private var taskList: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(kit.taskList.items.enumerated()), id: \.element.id) { index, task in
                HStack(spacing: 12) {
                    Text(task.done ? "Done" : "Open")
                        .font(.caption)
                        .frame(width: 44, alignment: .leading)
                    Text(task.title)
                        .accessibilityIdentifier("task.position.\(index)")
                    Spacer()
                    Button(task.done ? "Open" : "Done") {
                        kit.taskList.toggleDone(id: task.id)
                    }
                    .accessibilityIdentifier("task.toggle.\(task.id)")
                    Button("Delete") {
                        kit.taskList.deleteTask(id: task.id)
                    }
                    .accessibilityIdentifier("task.delete.\(task.id)")
                }
                .padding(.vertical, 6)
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier("task.row.\(task.id)")
            }
        }
        .accessibilityIdentifier("task.list")
    }

    private var actions: some View {
        HStack(spacing: 8) {
            Button("Sample") { kit.taskList.addSampleBatch() }
                .accessibilityIdentifier("task.sample")
            Button("Move") { kit.taskList.moveVisible(from: 0, to: max(Int64(kit.taskList.items.count - 1), 0)) }
                .disabled(kit.taskList.items.count < 2)
                .accessibilityIdentifier("task.move")
            Button("Rename") {
                if let first = kit.taskList.items.first {
                    kit.taskList.renameTask(id: first.id, title: "Renamed")
                }
            }
            .disabled(kit.taskList.items.isEmpty)
            .accessibilityIdentifier("task.rename")
            Button("Clear done") { kit.taskList.clearDone() }
                .disabled(!state.canClearDone)
                .accessibilityIdentifier("task.clear.done")
        }
        .buttonStyle(.borderedProminent)
    }

    private func filterButton(_ title: String, filter: TaskFilter, id: String) -> some View {
        Button(title) { kit.taskBoard.setFilter(filter: filter) }
            .disabled(state.filter == filter)
            .accessibilityIdentifier(id)
    }
}

#Preview {
    ContentView()
}
