import Foundation
import MimiCore

@MainActor
final class AppSettings: ObservableObject {
    @Published var workspaceID: String
    @Published var apiKey: String
    @Published var sourceLanguage: SourceLanguage
    @Published var translationMode: TranslationMode
    @Published var fontSize: Double
    @Published var isOverlayLocked: Bool

    private enum Keys {
        static let workspaceID = "workspaceID"
        static let sourceLanguage = "sourceLanguage"
        static let translationMode = "translationMode"
        static let fontSize = "fontSize"
        static let overlayLocked = "overlayLocked"
    }

    private let defaults: UserDefaults
    private let keychain: KeychainStore

    init(defaults: UserDefaults = .standard, keychain: KeychainStore = KeychainStore()) {
        self.defaults = defaults
        self.keychain = keychain
        self.workspaceID = defaults.string(forKey: Keys.workspaceID) ?? ""
        self.sourceLanguage = SourceLanguage(
            rawValue: defaults.string(forKey: Keys.sourceLanguage) ?? "en"
        ) ?? .english
        self.translationMode = TranslationMode(
            rawValue: defaults.string(forKey: Keys.translationMode) ?? "lowLatency"
        ) ?? .lowLatency

        let storedFontSize = defaults.double(forKey: Keys.fontSize)
        self.fontSize = storedFontSize > 0 ? storedFontSize : 30
        self.isOverlayLocked = defaults.object(forKey: Keys.overlayLocked) as? Bool ?? false
        self.apiKey = (try? keychain.loadAPIKey()) ?? ""
    }

    func configuration() throws -> LiveTranslationConfiguration {
        try LiveTranslationConfiguration(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            translationMode: translationMode
        ).validated()
    }

    func save() throws {
        let configuration = try configuration()
        workspaceID = configuration.workspaceID
        apiKey = configuration.apiKey
        try keychain.saveAPIKey(configuration.apiKey)
        persistPreferences()
    }

    func persistPreferences() {
        defaults.set(workspaceID, forKey: Keys.workspaceID)
        defaults.set(sourceLanguage.rawValue, forKey: Keys.sourceLanguage)
        defaults.set(translationMode.rawValue, forKey: Keys.translationMode)
        defaults.set(fontSize, forKey: Keys.fontSize)
        defaults.set(isOverlayLocked, forKey: Keys.overlayLocked)
    }
}
