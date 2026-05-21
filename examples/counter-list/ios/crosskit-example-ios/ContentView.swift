//
//  ContentView.swift
//  crosskit-example-ios
//
//  Created by zigengm3 on 2026/2/4.
//

import CrossKitShared
import Foundation
import SwiftUI

struct ContentView: View {
    // CrossKitSharedBridge is generated from `cross-kit.toml` bindings and the
    // Rust metadata. It owns the root app bridge plus child counter/list bridges.
    @StateObject private var kit = CrossKitSharedBridge(initial: 0)
    @State private var path: [AppRoute] = []

    var body: some View {
        NavigationStack(path: $path) {
            ScrollView {
                VStack(spacing: 24) {
                    counterSection
                    listSection
                }
                .padding()
            }
            .navigationTitle("Cross-Kit Demo")
            .navigationDestination(for: AppRoute.self) { route in
                switch route {
                case let .listDetail(id, dateCn):
                    ListDetailView(id: id, dateCn: dateCn)
                case .summary:
                    SummaryView(state: kit.app.state)
                }
            }
            .onChange(of: kit.app.state.route) { route in
                // Navigation is modeled as Rust state. The platform consumes the
                // route, performs native navigation, then clears it through an
                // action so the same route is not replayed.
                guard let route else { return }
                if let appRoute = AppRoute(route: route) {
                    path.append(appRoute)
                }
                kit.app.clearRoute()
            }
        }
    }

    private var counterSection: some View {
        VStack(spacing: 12) {
            Text("Counter")
                .font(.title.bold())
            Text("\(kit.counter.state.value)")
                .font(.system(size: 48, weight: .semibold))
                .monospacedDigit()
                .padding(.bottom, 4)
            Button(action: { _ = kit.counter.increment() }) {
                Text("+1")
                    .padding(.horizontal, 24)
                    .padding(.vertical, 10)
            }
            .accessibilityIdentifier("counter.increment")
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity)
    }

    private var listSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("List")
                .font(.title2.bold())
            listButtons
            VStack(spacing: 8) {
                ForEach(kit.list.items, id: \.id) { item in
                    HStack {
                        Text("#\(item.id)")
                            .font(.system(.body, design: .monospaced))
                        Spacer()
                        Text(item.dateCn)
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    .padding(.vertical, 6)
                    .padding(.horizontal, 10)
                    .background(Color(.secondarySystemBackground))
                    .clipShape(RoundedRectangle(cornerRadius: 10))
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var listButtons: some View {
        VStack(spacing: 8) {
            HStack(spacing: 12) {
                Button("Add") { _ = kit.list.appendNow() }
                    .accessibilityIdentifier("list.append")
                Button("Insert") { _ = kit.list.insertNow(index: 0) }
                    .accessibilityIdentifier("list.insert")
                Button("Update") { updateFirstItem() }
                    .accessibilityIdentifier("list.update")
            }
            HStack(spacing: 12) {
                Button("Move") { moveLastToFirst() }
                    .accessibilityIdentifier("list.move")
                Button("Sort") { _ = kit.list.sortByTimestampDesc() }
                    .accessibilityIdentifier("list.sort")
                Button("Remove") { removeLastItem() }
                    .accessibilityIdentifier("list.remove")
            }
        }
        .buttonStyle(.bordered)
    }

    private func updateFirstItem() {
        guard !kit.list.items.isEmpty else { return }
        let timestampMs = Int64(Date().timeIntervalSince1970 * 1000)
        _ = kit.list.updateWithTimestamp(index: 0, timestampMs: timestampMs)
    }

    private func moveLastToFirst() {
        let count = kit.list.items.count
        guard count > 1 else { return }
        _ = kit.list.moveItem(from: Int64(count - 1), to: 0)
    }

    private func removeLastItem() {
        let count = kit.list.items.count
        guard count > 0 else { return }
        _ = kit.list.removeAt(index: Int64(count - 1))
    }
}

enum AppRoute: Hashable {
    case listDetail(id: Int64, dateCn: String)
    case summary

    init?(route: Route) {
        switch route {
        case let .listDetail(id, dateCn):
            self = .listDetail(id: id, dateCn: dateCn)
        case .summary:
            self = .summary
        }
    }
}

struct ListDetailView: View {
    let id: Int64
    let dateCn: String

    var body: some View {
        VStack(spacing: 16) {
            Text("Detail")
                .font(.title.bold())
            Text("Item #\(id)")
                .font(.title2)
            Text(dateCn)
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .padding()
    }
}

struct SummaryView: View {
    let state: AppState

    var body: some View {
        VStack(spacing: 16) {
            Text("Summary")
                .font(.title.bold())
            Text("Counter: \(state.counter.value)")
                .font(.title3)
            Text("List count: \(state.listLen)")
                .font(.subheadline)
            if let last = state.lastItem {
                Text("Last: #\(last.id) \(last.dateCn)")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
        }
        .padding()
    }
}

#Preview {
    ContentView()
}
