import Foundation
import Security

enum KeychainStoreError: LocalizedError {
    case unexpectedStatus(OSStatus)
    case invalidData

    var errorDescription: String? {
        switch self {
        case let .unexpectedStatus(status):
            if let message = SecCopyErrorMessageString(status, nil) as String? {
                "Keychain: \(message)"
            } else {
                "Keychain operation failed (\(status))."
            }
        case .invalidData:
            "The saved API key could not be read from Keychain."
        }
    }
}

struct KeychainStore {
    private let service = "app.yuxino.mimi.credentials.v2"
    private let legacyService = "app.yuxino.mimi.translation"
    private let account = "dashscope-api-key"

    func loadAPIKey() throws -> String? {
        if let value = try loadAPIKey(service: service) {
            return value
        }

        guard let legacyValue = try loadAPIKey(service: legacyService) else {
            return nil
        }
        try saveAPIKey(legacyValue)
        return legacyValue
    }

    private func loadAPIKey(service: String) throws -> String? {
        var query = baseQuery(service: service)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw KeychainStoreError.unexpectedStatus(status)
        }
        guard
            let data = item as? Data,
            let value = String(data: data, encoding: .utf8)
        else {
            throw KeychainStoreError.invalidData
        }
        return value
    }

    func saveAPIKey(_ apiKey: String) throws {
        let data = Data(apiKey.utf8)
        let attributes = [kSecValueData as String: data]
        let updateStatus = SecItemUpdate(
            baseQuery(service: service) as CFDictionary,
            attributes as CFDictionary
        )

        if updateStatus == errSecSuccess {
            return
        }
        guard updateStatus == errSecItemNotFound else {
            throw KeychainStoreError.unexpectedStatus(updateStatus)
        }

        var addQuery = baseQuery(service: service)
        addQuery[kSecValueData as String] = data
        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        guard addStatus == errSecSuccess else {
            throw KeychainStoreError.unexpectedStatus(addStatus)
        }
    }

    private func baseQuery(service: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]
    }
}
