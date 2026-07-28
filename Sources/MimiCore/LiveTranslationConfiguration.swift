import Foundation

public enum LiveTranslationConfigurationError: Error, LocalizedError, Equatable, Sendable {
    case missingWorkspaceID
    case invalidWorkspaceID
    case missingAPIKey

    public var errorDescription: String? {
        switch self {
        case .missingWorkspaceID:
            "Add your Alibaba Cloud Model Studio Workspace ID in Settings."
        case .invalidWorkspaceID:
            "The Workspace ID is not valid. Copy it from Alibaba Cloud Model Studio."
        case .missingAPIKey:
            "Add your Alibaba Cloud Model Studio API key in Settings."
        }
    }
}

public struct LiveTranslationConfiguration: Equatable, Sendable {
    public var workspaceID: String
    public var apiKey: String
    public var sourceLanguage: SourceLanguage
    public var translationMode: TranslationMode

    public var effectiveTranslationMode: TranslationMode {
        sourceLanguage == .automatic ? .lowLatency : translationMode
    }

    public init(
        workspaceID: String,
        apiKey: String,
        sourceLanguage: SourceLanguage,
        translationMode: TranslationMode = .lowLatency
    ) {
        self.workspaceID = workspaceID
        self.apiKey = apiKey
        self.sourceLanguage = sourceLanguage
        self.translationMode = translationMode
    }

    public func validate() throws {
        _ = try validated()
    }

    public func validated() throws -> Self {
        let workspaceID = workspaceID.trimmingCharacters(in: .whitespacesAndNewlines)
        let apiKey = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !workspaceID.isEmpty else {
            throw LiveTranslationConfigurationError.missingWorkspaceID
        }
        do {
            _ = try LiveTranslateEndpoint(workspaceID: workspaceID)
        } catch {
            throw LiveTranslationConfigurationError.invalidWorkspaceID
        }
        guard !apiKey.isEmpty else {
            throw LiveTranslationConfigurationError.missingAPIKey
        }

        return Self(
            workspaceID: workspaceID,
            apiKey: apiKey,
            sourceLanguage: sourceLanguage,
            translationMode: translationMode
        )
    }
}
