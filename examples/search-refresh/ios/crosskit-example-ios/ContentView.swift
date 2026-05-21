import CrossKitSearchRefreshShared
import SwiftUI

struct ContentView: View {
    // Cross-Kit still exposes a synchronous action + observed state model even
    // for long-running work. The generated bridge owns subscription cleanup.
    @StateObject private var kit = CrossKitSearchRefreshBridge()

    private var state: SearchState {
        // Loading, progress, notices, and stale-result protection are Rust
        // presentation state. SwiftUI does not need an async task model here.
        kit.search.state
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Search Refresh")
                .font(.title.bold())
                .accessibilityIdentifier("search.title")
            TextField("Query", text: Binding(
                get: { state.query },
                // Text edits are intent calls into Rust; Rust decides how they
                // affect pending work, notices, and previous results.
                set: { kit.search.updateQuery(query: $0) }
            ))
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("search.query")
            controls
            progress
            notice
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

            Button("Retry") {
                kit.search.submit()
            }
            .disabled(!state.canRetry)
            .accessibilityIdentifier("search.retry")

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
            Text(statusLabel(state.status))
                .accessibilityIdentifier("search.loading")
            Text("Progress \(state.progress)%")
                .accessibilityIdentifier("search.progress")
        }
        .font(.subheadline)
    }

    @ViewBuilder
    private var notice: some View {
        if let notice = state.notice {
            Text(noticeText(notice))
                .font(.caption)
                .foregroundStyle(noticeColor(notice))
                .accessibilityIdentifier("search.notice")
        }
    }

    private var results: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Results").font(.headline)
            if state.status == .empty {
                Text("No results")
                    .font(.caption)
                    .accessibilityIdentifier("search.empty")
            }
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

    private func statusLabel(_ status: SearchStatus) -> String {
        switch status {
        case .idle:
            return "Idle"
        case .loading:
            return "Loading"
        case .results:
            return "Results"
        case .empty:
            return "Empty"
        case .failed:
            return "Failed"
        }
    }

    private func noticeText(_ notice: SearchNotice) -> String {
        switch notice {
        case let .inline(message):
            return message
        case let .toast(message):
            return message
        case let .dialog(title, message):
            return "\(title): \(message)"
        }
    }

    private func noticeColor(_ notice: SearchNotice) -> Color {
        switch notice {
        case .inline:
            return .red
        case .toast:
            return .orange
        case .dialog:
            return .blue
        }
    }
}

#Preview {
    ContentView()
}
