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
