//
//  ContentView.swift
//  crosskit-example-ios
//
//  Created by zigengm3 on 2026/2/4.
//

import SwiftUI

struct ContentView: View {
    @StateObject private var vm = CounterViewModelBridge()

    var body: some View {
        VStack {
            Text("Counter")
                .font(.title.bold())
            Text("\(vm.value)")
                .font(.system(size: 48, weight: .semibold))
                .monospacedDigit()
                .padding(.bottom, 12)
            Button(action: vm.increment) {
                Text("+1")
                    .padding(.horizontal, 24)
                    .padding(.vertical, 10)
            }
            .buttonStyle(.borderedProminent)
        }
        .padding()
    }
}

#Preview {
    ContentView()
}
