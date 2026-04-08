import Capacitor
import Foundation
import LocalAuthentication
import Security

@objc(SecureVaultPlugin)
public class SecureVaultPlugin: CAPPlugin {
    private let keyTag = "xyz.gradience.wallet.securevault"
    private let keyService = "SecureVault"

    @objc func isAvailable(_ call: CAPPluginCall) {
        let context = LAContext()
        var error: NSError?
        let available = context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error)
        call.resolve(["value": available])
    }

    @objc func storeKey(_ call: CAPPluginCall) {
        guard let key = call.getString("key") else {
            call.reject("Key is required")
            return
        }
        _ = deleteExisting()

        var error: Unmanaged<CFError>?
        guard let accessControl = SecAccessControlCreateWithFlags(
            kCFAllocatorDefault,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            .biometryAny,
            &error
        ) else {
            call.reject("Failed to create access control")
            return
        }

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: keyTag,
            kSecAttrService as String: keyService,
            kSecValueData as String: key.data(using: .utf8)!,
            kSecAttrAccessControl as String: accessControl,
        ]
        let status = SecItemAdd(query as CFDictionary, nil)
        if status == errSecSuccess {
            call.resolve()
        } else {
            call.reject("Failed to store key", String(status))
        }
    }

    @objc func retrieveKey(_ call: CAPPluginCall) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: keyTag,
            kSecAttrService as String: keyService,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
            kSecUseOperationPrompt as String: "Authenticate to unlock your wallet",
        ]
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecSuccess, let data = result as? Data, let key = String(data: data, encoding: .utf8) {
            call.resolve(["key": key])
        } else {
            call.reject("Failed to retrieve key", String(status))
        }
    }

    @objc func deleteKey(_ call: CAPPluginCall) {
        let status = deleteExisting()
        call.resolve(["deleted": status == errSecSuccess || status == errSecItemNotFound])
    }

    private func deleteExisting() -> OSStatus {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: keyTag,
            kSecAttrService as String: keyService,
        ]
        return SecItemDelete(query as CFDictionary)
    }
}
