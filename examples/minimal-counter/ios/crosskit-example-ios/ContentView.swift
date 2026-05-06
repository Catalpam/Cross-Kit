import CrossKitMinimalCounterShared
import SwiftUI

struct ContentView: View {
    @StateObject private var kit = CrossKitMinimalCounterBridge(initial: 0)

    var body: some View {
        VStack(spacing: 12) {
            Text("Minimal Counter")
                .font(.title.bold())
            Text("\(kit.counter.state.value)")
                .font(.system(size: 48, weight: .semibold))
                .monospacedDigit()
                .accessibilityIdentifier("counter.value")
            HStack(spacing: 12) {
                Button("-1") { _ = kit.counter.decrement() }
                    .accessibilityIdentifier("counter.decrement")
                Button("Reset") { _ = kit.counter.reset() }
                    .accessibilityIdentifier("counter.reset")
                Button("+1") { _ = kit.counter.increment() }
                    .accessibilityIdentifier("counter.increment")
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
    }
}

#Preview {
    ContentView()
}
