import CrossKitSearchRefreshShared
import SwiftUI

struct ContentView: View {
    @StateObject private var kit = CrossKitSearchRefreshBridge()

    private var state: SearchState {
        kit.search.state
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Search Refresh")
                .font(.title.bold())
                .accessibilityIdentifier("search.title")
            TextField("Query", text: Binding(
                get: { state.query },
                set: { kit.search.updateQuery(query: $0) }
            ))
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("search.query")
            controls
            progress
            if let error = state.error {
                Text(String(describing: error))
                    .font(.caption)
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("search.error")
            }
            results
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding()
    }

    private var controls: some View {
        HStack(spacing: 10) {
            Button("Submit") {
                kit.search.submit()
            }
            .disabled(!state.canSubmit)
            .accessibilityIdentifier("search.submit")

            Button("Tick") {
                kit.search.tick()
            }
            .disabled(!state.canCancel)
            .accessibilityIdentifier("search.tick")

            Button("Cancel") {
                kit.search.cancel()
            }
            .disabled(!state.canCancel)
            .accessibilityIdentifier("search.cancel")
        }
        .buttonStyle(.borderedProminent)
    }

    private var progress: some View {
        HStack(spacing: 10) {
            Text(state.isLoading ? "Loading" : "Idle")
                .accessibilityIdentifier("search.loading")
            Text("Progress \(state.progress)%")
                .accessibilityIdentifier("search.progress")
        }
        .font(.subheadline)
    }

    private var results: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Results").font(.headline)
            ForEach(state.results, id: \.rank) { result in
                VStack(alignment: .leading, spacing: 2) {
                    Text(result.title)
                        .font(.body.weight(.semibold))
                        .accessibilityIdentifier("search.result.\(result.rank).title")
                    Text(result.snippet)
                        .font(.caption)
                        .accessibilityIdentifier("search.result.\(result.rank).snippet")
                }
            }
        }
    }
}

#Preview {
    ContentView()
}
