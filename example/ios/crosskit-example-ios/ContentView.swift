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
    @StateObject private var appVm: AppViewModelBridge
    @StateObject private var counterVm: CounterViewModelBridge
    @StateObject private var listVm: ListViewModelBridge
    @State private var path: [AppRoute] = []

    init() {
        let app = AppViewModelBridge(initial: 0)
        _appVm = StateObject(wrappedValue: app)
        _counterVm = StateObject(wrappedValue: CounterViewModelBridge(app: app))
        _listVm = StateObject(wrappedValue: ListViewModelBridge(app: app))
    }

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
                    SummaryView(state: appVm.state)
                }
            }
            .onChange(of: appVm.state.route) { route in
                guard let route else { return }
                if let appRoute = AppRoute(route: route) {
                    path.append(appRoute)
                }
                appVm.clearRoute()
            }
        }
    }

    private var counterSection: some View {
        VStack(spacing: 12) {
            Text("Counter")
                .font(.title.bold())
            Text("\(counterVm.state.value)")
                .font(.system(size: 48, weight: .semibold))
                .monospacedDigit()
                .padding(.bottom, 4)
            Button(action: { _ = counterVm.increment() }) {
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
                ForEach(listVm.items, id: \.id) { item in
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
                Button("Add") { _ = listVm.appendNow() }
                    .accessibilityIdentifier("list.append")
                Button("Insert") { _ = listVm.insertNow(index: 0) }
                    .accessibilityIdentifier("list.insert")
                Button("Update") { updateFirstItem() }
                    .accessibilityIdentifier("list.update")
            }
            HStack(spacing: 12) {
                Button("Move") { moveLastToFirst() }
                    .accessibilityIdentifier("list.move")
                Button("Sort") { _ = listVm.sortByTimestampDesc() }
                    .accessibilityIdentifier("list.sort")
                Button("Remove") { removeLastItem() }
                    .accessibilityIdentifier("list.remove")
            }
        }
        .buttonStyle(.bordered)
    }

    private func updateFirstItem() {
        guard !listVm.items.isEmpty else { return }
        let timestampMs = Int64(Date().timeIntervalSince1970 * 1000)
        _ = listVm.updateWithTimestamp(index: 0, timestampMs: timestampMs)
    }

    private func moveLastToFirst() {
        let count = listVm.items.count
        guard count > 1 else { return }
        _ = listVm.moveItem(from: Int64(count - 1), to: 0)
    }

    private func removeLastItem() {
        let count = listVm.items.count
        guard count > 0 else { return }
        _ = listVm.removeAt(index: Int64(count - 1))
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
