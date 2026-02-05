//
//  LoginPage.swift
//  SigSong
//
//  Created by ZigengM1 on 2024/01/28.
//

import Foundation
import SwiftUI

struct LoginPage: View {
    @State private var username: String
    @State private var password: String = ""
    @State private var isSelected = false

    @State private var accountErrorMsg: String?
    @State private var passwordErrorMsg: String?

    @EnvironmentObject var userStore: UserStore
    @EnvironmentObject var ntToast: NTToast
    @Environment(\.dismiss) private var dismiss

    init(account: String? = nil) {
        username = account ?? ""
    }
    var body: some View {
        VStack {
            TextField("", text: $username)
                .placeholder(when: username.isEmpty) {
                    if let text = accountErrorMsg {
                        Text(text).foregroundColor(.red)
                    } else {
                        Text("用户名").foregroundColor(.gray)
                    }
                }
                .padding()
                .background(Color.gray.opacity(0.3))
                .cornerRadius(8)
                .padding(.vertical, 10)
                .padding(.horizontal, 20)

            SecureField("", text: $password)
                .placeholder(when: password.isEmpty) {
                    if let text = passwordErrorMsg {
                        Text(text).foregroundColor(.red)
                    } else {
                        Text("密码").foregroundColor(.gray)
                    }
                }
                .padding()
                .background(Color.gray.opacity(0.3))
                .cornerRadius(8)
                .padding(.vertical, 10)
                .padding(.horizontal, 20)

            HStack {
                Toggle("选择此项代表您已阅读并同意了用户协议", isOn: $isSelected)
                    .padding(.vertical, 10)
                    .padding(.horizontal, 20)
                    .toggleStyle(CheckBoxToggleStyle(shape: .circle))
                Spacer()
            }.alert(isPresented: $isSelected) {
                Alert(
                    title: Text("错误"),
                    message: Text("ErrorMessage"),
                    dismissButton: .default(Text("确定")) {
                        ntToast.showToast(.info, message: "toast测试测试")
                        isSelected = false
                    }
                )
            }
            Button(action: login) {
                Text("Log In")
                    .padding()
                    .foregroundColor(.white)
                    .frame(maxWidth: .infinity)
                    .background(Color.blue)
                    .cornerRadius(5.0)
            }
            .cornerRadius(100)
            .padding(.vertical, 5)
            .frame(maxWidth: 200)
        }
        .onAppear {

        }
        .padding()
        .navigationBarHidden(true)
    }

    func login() {
        accountErrorMsg = username.isEmpty ? "用户名还没有填呢" : nil
        passwordErrorMsg = password.isEmpty ? "密码还没有填呢" : nil

        guard accountErrorMsg == nil, passwordErrorMsg == nil else { return }
        guard isSelected else {
            ntToast.showToast(.info, message: "请先勾选用户协议")
            return
        }

        Task { @MainActor in
            do {
                try await userStore.login(account: username, password: password)
                ntToast.showToast(.success, message: "登录成功")
                dismiss()
            } catch {
                ntToast.showToast(.error, message: error.localizedDescription)
            }
        }
    }
}

struct CheckBoxToggleStyle: ToggleStyle {
    enum CheckBoxShape: String {
        case circle
        case square
    }
    let shape: CheckBoxShape
    init(shape: CheckBoxShape = .circle) {
        self.shape = shape
    }
    // configuration中包含isOn和Label
    func makeBody(configuration: Configuration) -> some View {
        let systemName: String = configuration.isOn ? "checkmark.\(shape.rawValue).fill" : shape.rawValue
        HStack {
            Image(systemName: systemName)
                .resizable()
                .frame(width: 20, height: 20)
            configuration.label
                .font(.system(size: 15))
                .foregroundStyle(.gray)
        }.onTapGesture {
            configuration.isOn.toggle()
        }
    }
}

extension View {
    func placeholder<Content: View>(
        when shouldShow: Bool,
        alignment: Alignment = .leading,
        @ViewBuilder placeholder: () -> Content) -> some View {

            ZStack(alignment: alignment) {
                placeholder().opacity(shouldShow ? 1 : 0)
                self
            }
        }
}

#Preview {
    LoginPage()
}
