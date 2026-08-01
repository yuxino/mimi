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

            Picker("识别语言", selection: sourceLanguageBinding) {
                ForEach(SourceLanguage.manualCases) { language in
                    Text(sourceLanguageTitle(language)).tag(language)
                }
            }
            .disabled(isChangingSession)

            Label(
                settings.targetLanguage.translatesAudio ? "高质量翻译" : "只显示中文原文",
                systemImage: settings.targetLanguage.translatesAudio ? "sparkles" : "text.quote"
            )
            .font(.caption)
            .foregroundStyle(.secondary)

            Toggle("Lock Subtitle Position", isOn: $settings.isOverlayLocked)
                .onChange(of: settings.isOverlayLocked) {
                    settings.persistPreferences()
                    model.setOverlayLocked(settings.isOverlayLocked)
                }

            Button("Show Subtitle Window") {
                model.showOverlay()
            }
            .disabled(model.state.status != .listening)

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
        .onAppear {
            prepareLanguagePreferences()
        }
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
                        prepareLanguagePreferences()
                        await model.start(using: settings)
                    } else {
                        await model.stop()
                    }
                }
            }
        )
    }

    private var sourceLanguageBinding: Binding<SourceLanguage> {
        Binding(
            get: {
                settings.sourceLanguage == .automatic ? .japanese : settings.sourceLanguage
            },
            set: { language in
                Task {
                    await model.switchSourceLanguage(to: language, using: settings)
                }
            }
        )
    }

    private func sourceLanguageTitle(_ language: SourceLanguage) -> String {
        language == .chinese ? "中文原文" : language.displayName
    }

    private func prepareLanguagePreferences() {
        if settings.sourceLanguage == .automatic {
            settings.sourceLanguage = .japanese
        }
        if settings.sourceLanguage == .chinese {
            settings.targetLanguage = .original
        }
        settings.translationMode = .highQuality
        settings.persistPreferences()
    }

    private var isChangingSession: Bool {
        model.state.status == .connecting || model.state.status == .stopping
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
