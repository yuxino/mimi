import SwiftUI

@main
struct MimiApplication: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var model = AppModel()
    @StateObject private var settings = AppSettings()

    var body: some Scene {
        MenuBarExtra {
            MenuBarView()
                .environmentObject(model)
                .environmentObject(settings)
        } label: {
            Label("mimi", systemImage: menuBarIcon)
                .task {
                    model.attachOverlay(settings: settings)
                    appDelegate.installShowSettingsHandler { [weak model] in
                        model?.showSettings()
                    }
                    appDelegate.installGlobalHotKeyHandler { [weak model, weak settings] in
                        guard let model, let settings else { return }
                        guard model.state.status != .connecting,
                            model.state.status != .stopping
                        else { return }

                        Task {
                            if model.isActive {
                                await model.stop()
                            } else {
                                settings.prepareForListening()
                                await model.start(using: settings)
                            }
                        }
                    }
                }
        }
        .menuBarExtraStyle(.window)

        .commands {
            CommandGroup(replacing: .appSettings) {
                Button("Settings…") {
                    model.showSettings()
                }
                .keyboardShortcut(",", modifiers: .command)
            }
        }
    }

    private var menuBarIcon: String {
        if model.isPaused {
            return "pause.circle.fill"
        }

        return switch model.state.status {
        case .listening:
            "ear.badge.waveform"
        case .connecting, .stopping:
            "waveform"
        case .error:
            "exclamationmark.triangle"
        case .idle:
            "ear"
        }
    }
}
