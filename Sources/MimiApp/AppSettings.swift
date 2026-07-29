import Foundation
import MimiCore

@MainActor
final class AppSettings: ObservableObject {
    static let fontSizeRange = 14.0 ... 20.0
    static let defaultFontSize = 18.0

    @Published var workspaceID: String
    @Published var apiKey: String
    @Published var sourceLanguage: SourceLanguage
    @Published var targetLanguage: TargetLanguage
    @Published var translationMode: TranslationMode
    @Published var fontSize: Double
    @Published var isOverlayLocked: Bool
    @Published private(set) var credentialLoadError: String?

    private enum Keys {
        static let workspaceID = "workspaceID"
        static let sourceLanguage = "sourceLanguage"
        static let targetLanguage = "targetLanguage"
        static let translationMode = "translationMode"
        static let fontSize = "fontSize"
        static let overlayLocked = "overlayLocked"
    }

    private let defaults: UserDefaults
    private let keychain: KeychainStore

    init(defaults: UserDefaults = .standard, keychain: KeychainStore = KeychainStore()) {
        let isUITestMode = ProcessInfo.processInfo.environment["MIMI_UI_TEST"] == "1"
        self.defaults = defaults
        self.keychain = keychain
        self.workspaceID = isUITestMode
            ? "your-workspace-id"
            : defaults.string(forKey: Keys.workspaceID) ?? ""
        let storedSourceLanguage = SourceLanguage(
            rawValue: defaults.string(forKey: Keys.sourceLanguage) ?? "auto"
        ) ?? .automatic
        self.sourceLanguage = storedSourceLanguage
        self.targetLanguage = TargetLanguage(
            rawValue: defaults.string(forKey: Keys.targetLanguage) ?? "zh"
        ) ?? .simplifiedChinese
        let storedTranslationMode = TranslationMode(
            rawValue: defaults.string(forKey: Keys.translationMode) ?? "highQuality"
        ) ?? .highQuality
        self.translationMode = storedSourceLanguage == .automatic
            ? .lowLatency
            : storedTranslationMode

        let storedFontSize = defaults.double(forKey: Keys.fontSize)
        let preferredFontSize = storedFontSize == 0
            ? Self.defaultFontSize
            : storedFontSize
        self.fontSize = min(
            Self.fontSizeRange.upperBound,
            max(Self.fontSizeRange.lowerBound, preferredFontSize)
        )
        self.isOverlayLocked = defaults.object(forKey: Keys.overlayLocked) as? Bool ?? false
        if isUITestMode {
            self.apiKey = "sk-demo-not-a-real-key"
            self.credentialLoadError = nil
        } else {
            do {
                self.apiKey = try keychain.loadAPIKey() ?? ""
                self.credentialLoadError = nil
            } catch {
                self.apiKey = ""
                self.credentialLoadError = error.localizedDescription
            }
        }
    }

    func configuration() throws -> LiveTranslationConfiguration {
        try LiveTranslationConfiguration(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            targetLanguage: targetLanguage,
            translationMode: translationMode
        ).validated()
    }

    func save() throws {
        let configuration = try configuration()
        workspaceID = configuration.workspaceID
        apiKey = configuration.apiKey
        targetLanguage = configuration.targetLanguage
        translationMode = configuration.effectiveTranslationMode
        try keychain.saveAPIKey(configuration.apiKey)
        credentialLoadError = nil
        persistPreferences()
    }

    @discardableResult
    func reloadAPIKey() throws -> Bool {
        do {
            guard let storedKey = try keychain.loadAPIKey(), !storedKey.isEmpty else {
                credentialLoadError = nil
                return false
            }
            apiKey = storedKey
            credentialLoadError = nil
            return true
        } catch {
            credentialLoadError = error.localizedDescription
            throw error
        }
    }

    func persistPreferences() {
        defaults.set(workspaceID, forKey: Keys.workspaceID)
        defaults.set(sourceLanguage.rawValue, forKey: Keys.sourceLanguage)
        defaults.set(targetLanguage.rawValue, forKey: Keys.targetLanguage)
        defaults.set(translationMode.rawValue, forKey: Keys.translationMode)
        defaults.set(fontSize, forKey: Keys.fontSize)
        defaults.set(isOverlayLocked, forKey: Keys.overlayLocked)
    }
}
