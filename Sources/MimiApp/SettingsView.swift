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

                if !message.isEmpty {
                    Text(message)
                        .font(.caption)
                        .foregroundStyle(isError ? .red : .green)
                }
                Spacer()
            }
        }
        .formStyle(.grouped)
        .frame(width: 520, height: 360)
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
}
