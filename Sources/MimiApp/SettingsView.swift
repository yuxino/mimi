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
            }

            Section("Subtitles") {
                Picker("Source language", selection: $settings.sourceLanguage) {
                    ForEach(SourceLanguage.allCases) { language in
                        Text(language.displayName).tag(language)
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
                            await model.start(using: settings)
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
        .frame(width: 540, height: 420)
        .onDisappear {
            settings.persistPreferences()
            model.setOverlayLocked(settings.isOverlayLocked)
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
