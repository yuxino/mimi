import MimiCore
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings
    @State private var message = ""
    @State private var isError = false

    var body: some View {
        Form {
            Section("Alibaba Cloud Model Studio") {
                TextField("Workspace ID", text: $settings.workspaceID)
                    .textFieldStyle(.roundedBorder)
                SecureField("DashScope API Key", text: $settings.apiKey)
                    .textFieldStyle(.roundedBorder)
                Text("Credentials stay on this Mac. The API key is stored in Keychain.")
                    .font(.caption)
                    .foregroundStyle(.secondary)

                if let credentialError = settings.credentialLoadError {
                    HStack(alignment: .firstTextBaseline) {
                        Text("Couldn’t read the saved API key: \(credentialError)")
                            .font(.caption)
                            .foregroundStyle(.red)
                        Button("Try Again") {
                            reloadAPIKey()
                        }
                        .buttonStyle(.link)
                    }
                }
            }

            Section("Subtitles") {
                Picker("Translation mode", selection: $settings.translationMode) {
                    ForEach(TranslationMode.allCases) { mode in
                        Text(mode.displayName).tag(mode)
                    }
                }
                .disabled(model.isActive || settings.sourceLanguage == .automatic)

                Text(translationModeHelp)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Picker("Source language", selection: $settings.sourceLanguage) {
                    ForEach(SourceLanguage.allCases) { language in
                        Text(language.displayName).tag(language)
                    }
                }
                .disabled(model.isActive)
                .onChange(of: settings.sourceLanguage) {
                    if settings.sourceLanguage == .automatic {
                        settings.translationMode = .lowLatency
                    }
                }

                HStack {
                    Text("Font size")
                    Slider(value: $settings.fontSize, in: 20...48, step: 1)
                    Text("\(Int(settings.fontSize))")
                        .monospacedDigit()
                        .frame(width: 28, alignment: .trailing)
                }

                Toggle("Lock subtitle position", isOn: $settings.isOverlayLocked)

                Text("Drag the subtitle background to move it. Drag an edge to resize it; mimi remembers both after restart.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            HStack {
                Button("Save") {
                    save()
                }
                .keyboardShortcut(.defaultAction)

                Button {
                    Task {
                        if model.isActive {
                            await model.stop()
                        } else {
                            startListening()
                        }
                    }
                } label: {
                    Label(
                        model.isActive ? "Stop Listening" : "Start Listening",
                        systemImage: model.isActive ? "stop.fill" : "play.fill"
                    )
                }

                if !message.isEmpty {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(isError ? .red : .green)
                }

                Text(sessionStatusText)
                    .font(.caption)
                    .foregroundStyle(sessionStatusColor)
                Spacer()
            }
        }
        .formStyle(.grouped)
        .frame(width: 560, height: 460)
        .onDisappear {
            settings.persistPreferences()
            model.setOverlayLocked(settings.isOverlayLocked)
        }
        .onAppear {
            if settings.apiKey.isEmpty, settings.credentialLoadError != nil {
                reloadAPIKey()
            }
        }
    }

    private func save() {
        do {
            try settings.save()
            model.setOverlayLocked(settings.isOverlayLocked)
            isError = false
            message = "Saved"
        } catch {
            isError = true
            message = error.localizedDescription
        }
    }

    private func startListening() {
        model.setOverlayLocked(settings.isOverlayLocked)
        isError = false
        message = "Starting…"
        Task {
            await model.start(using: settings)
            if case let .error(errorMessage) = model.state.status {
                isError = true
                message = errorMessage
            } else {
                message = "Saved"
            }
        }
    }

    private func reloadAPIKey() {
        do {
            if try settings.reloadAPIKey() {
                isError = false
                message = "API key restored"
            }
        } catch {
            isError = true
            message = error.localizedDescription
        }
    }

    private var sessionStatusText: String {
        switch model.state.status {
        case .idle:
            "Ready"
        case .connecting:
            "Connecting…"
        case .listening:
            "Listening and translating"
        case .stopping:
            "Stopping…"
        case let .error(message):
            message
        }
    }

    private var translationModeHelp: String {
        if settings.sourceLanguage == .automatic {
            return "自动判断每段语音的语言，并使用低延迟模式翻译成中文。"
        }

        return switch settings.translationMode {
        case .lowLatency:
            "实时识别并持续翻译，字幕出现更快。"
        case .highQuality:
            "整句翻译更稳，但通常会多等几秒。"
        }
    }

    private var sessionStatusColor: Color {
        switch model.state.status {
        case .listening:
            .green
        case .error:
            .red
        default:
            .secondary
        }
    }
}
