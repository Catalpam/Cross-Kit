import Combine
import Foundation
import CrossKitShared

@MainActor
final class CounterViewModelBridge: ObservableObject, CounterObserver {
    @Published private(set) var value: Int32 = 0

    private let vm: CrossKitShared.CounterViewModelProtocol

    init(initial: Int32 = 0) {
        let vm = CrossKitShared.CounterViewModel(initial: initial)
        self.vm = vm
        vm.subscribe(observer: self)
    }

    func increment() {
        _ = vm.increment()
    }

    func onState(state: CounterState) {
        if Thread.isMainThread {
            value = state.value
        } else {
            DispatchQueue.main.async { [weak self] in
                self?.value = state.value
            }
        }
    }
}
