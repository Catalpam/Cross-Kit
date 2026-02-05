//
//  Invoke.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/28.
//

import Foundation
import SigsongSDK

class SwiftInvokeFFI: InvokeFfi {
    weak var delegate: SwiftInvokeFFIDelegate?
//    func currentUserUpdate(userInfo: UserInfo) {
//        delegate?.setCurrentUser(userInfo)
//    }

    func getDocumentPath() throws -> String {
        FileManager.default
            .urls(for: .documentDirectory, in: .userDomainMask).first!
            .path(percentEncoded: false)
    }

    func showToast(toastType: SigsongSDK.ToastType, message: String) {
        delegate?.showToast(toastType: toastType, message: message)
    }
}

protocol SwiftInvokeFFIDelegate: AnyObject {
//    func setCurrentUser(_ new: UserInfo)
    func showToast(toastType: SigsongSDK.ToastType, message: String)
}
