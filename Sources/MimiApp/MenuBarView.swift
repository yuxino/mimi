import AppKit
import MimiCore
import SwiftUI

struct MenuBarView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 10) {
                Image(systemName: "ear.badge.waveform")
                    .font(.title2)
                VStack(alignment: .leading, spacing: 2) {
                    Text("mimi")
                        .font(.headline)
                    Text(statusText)
                        .font(.caption)
                        .foregroundStyle(statusColor)
                        .lineLimit(2)
                }
            }

            Divider()

            Toggle(isOn: listeningBinding) {
                Label(
                    "Live Subtitles",
                    systemImage: model.isActive ? "waveform" : "waveform.slash"
                )
            }
            .keyboardShortcut(.space, modifiers: [.command, .shift])

            Picker("Source Language", selection: $settings.sourceLanguage) {
                ForEach(SourceLanguage.allCases) { language in
                    Text(language.displayName).tag(language)
                }
            }
            .disabled(model.isActive)
            .onChange(of: settings.sourceLanguage) {
                if settings.sourceLanguage == .automatic {
                    settings.translationMode = .lowLatency
                }
                settings.persistPreferences()
            }

            Picker("Translation Mode", selection: $settings.translationMode) {
                ForEach(TranslationMode.allCases) { mode in
                    Text(mode.displayName).tag(mode)
                }
            }
            .disabled(model.isActive || settings.sourceLanguage == .automatic)
            .onChange(of: settings.translationMode) {
                settings.persistPreferences()
            }

            Toggle("Lock Subtitle Position", isOn: $settings.isOverlayLocked)
                .onChange(of: settings.isOverlayLocked) {
                    settings.persistPreferences()
                    model.setOverlayLocked(settings.isOverlayLocked)
                }

            Button("Show Subtitle Window") {
                model.showOverlay()
            }

            Button("Clear Subtitles") {
                model.clearSubtitles()
            }

            Divider()

            Button {
                model.showSettings()
            } label: {
                Label("Settings…", systemImage: "gearshape")
            }

            Button("Quit mimi") {
                NSApplication.shared.terminate(nil)
            }
            .keyboardShortcut("q")
        }
        .padding(14)
        .frame(width: 290)
    }

    private var statusText: String {
        switch model.state.status {
        case .idle:
            settings.workspaceID.isEmpty || settings.apiKey.isEmpty ? "Setup required" : "Ready"
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

    private var listeningBinding: Binding<Bool> {
        Binding(
            get: { model.isActive },
            set: { shouldListen in
                Task {
                    if shouldListen {
                        await model.start(using: settings)
                    } else {
                        await model.stop()
                    }
                }
            }
        )
    }

    private var statusColor: Color {
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
