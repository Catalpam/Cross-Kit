import Foundation
import SigsongSDK
import Combine

public enum Event {
//    case /*updateCurrentUser*/(new: UserInfo)
    case showToast(toastType: ToastType, message: String)
}

private let SDKBridge = SDKBridgeCenter()
// 现在需要注入的依赖比较少, 先通过全局变量的形式去管理, 如果比较多的话再用DIC
public let API = SDKBridge.invokeManager
public let EventPublisher = SDKBridge.eventPublisher

//public typealias API = InvokeManagerProtocol
public class SDKBridgeCenter {
    public let eventPublisher = PassthroughSubject<Event, Never>()
    let invokeManager: InvokeManager
    public init() {
        let startTime = Date()
        let invokeFFI = SwiftInvokeFFI()
        try! invokeManager = InvokeManager(invoke: invokeFFI)
        invokeFFI.delegate = self
        print("初始化SDK操作耗时：\(Date().timeIntervalSince(startTime)) 秒")
    }
}

extension SDKBridgeCenter: SwiftInvokeFFIDelegate {
//    func setCurrentUser(_ new: UserInfo) {
//        eventPublisher.send(.updateCurrentUser(new: new))
//    }
    func showToast(toastType: SigsongSDK.ToastType, message: String) {
        eventPublisher.send(.showToast(toastType: toastType, message: message))
    }
}
