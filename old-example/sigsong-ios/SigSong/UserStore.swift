import Combine
import Foundation
import InvokeKit
import SigsongSDK

@MainActor
final class UserStore: ObservableObject {
    @Published private(set) var current: ClUserInfo?
    @Published private(set) var isLoading = true

    init() {
        Task { await refreshCurrentUser() }
    }

    func refreshCurrentUser() async {
        defer { isLoading = false }
        do {
            current = try await API.currentUser()
        } catch {
            print("[UserStore] failed to load cached user: \(error)")
            current = nil
        }
    }

    @discardableResult
    func login(account: String, password: String) async throws -> ClUserInfo {
        let user = try await API.login(username: account, password: password)
        current = user
        return user
    }

    func logout() async {
        do {
            try await API.logout()
        } catch {
            print("[UserStore] logout failed: \(error)")
        }
        current = nil
    }
}
