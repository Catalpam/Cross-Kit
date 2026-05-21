import CrossKitMinimalCounterShared
import SwiftUI

struct ContentView: View {
    // This is the only Cross-Kit object the app creates. The generated root
    // container owns the Rust VM, subscribes to it, and exposes `counter.state`.
    @StateObject private var kit = CrossKitMinimalCounterBridge(initial: 0)

    var body: some View {
        VStack(spacing: 12) {
            Text("Minimal Counter")
                .font(.title.bold())
            // SwiftUI reads generated observable state; there is no direct FFI
            // call here and no local copy of the counter value.
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
