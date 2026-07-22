import SwiftUI

@main
struct MimiApplication: App {
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
                }
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environmentObject(model)
                .environmentObject(settings)
        }
        .windowResizability(.contentSize)
    }

    private var menuBarIcon: String {
        switch model.state.status {
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
