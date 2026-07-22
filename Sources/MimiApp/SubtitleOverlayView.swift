import MimiCore
import SwiftUI

struct SubtitleOverlayView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings

    var body: some View {
        VStack(spacing: 8) {
            if !model.state.subtitles.source.text.isEmpty {
                Text(model.state.subtitles.source.text)
                    .font(.system(size: max(14, settings.fontSize * 0.6), weight: .medium))
                    .foregroundStyle(
                        model.state.subtitles.source.isFinal
                            ? Color.white.opacity(0.82)
                            : Color.white.opacity(0.48)
                    )
                    .lineLimit(2)
                    .multilineTextAlignment(.center)
            }

            Text(translationText)
                .font(.system(size: settings.fontSize, weight: .semibold))
                .foregroundStyle(translationColor)
                .lineLimit(2)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 28)
        .padding(.vertical, 18)
        .background(.black.opacity(0.76), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 18, style: .continuous)
                .stroke(.white.opacity(0.1), lineWidth: 1)
        }
        .overlay(alignment: .topTrailing) {
            if !settings.isOverlayLocked {
                Button {
                    model.showSettings()
                } label: {
                    Image(systemName: "gearshape.fill")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(.white.opacity(0.62))
                        .padding(7)
                        .background(.black.opacity(0.34), in: Circle())
                }
                .buttonStyle(.plain)
                .help("Open mimi Settings")
                .accessibilityLabel("Open mimi Settings")
                .padding(14)
            }
        }
        .overlay(alignment: .bottomTrailing) {
            if !settings.isOverlayLocked {
                Image(systemName: "arrow.up.left.and.arrow.down.right")
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(.white.opacity(0.35))
                    .padding(14)
                    .allowsHitTesting(false)
                    .accessibilityHidden(true)
            }
        }
        .padding(8)
    }

    private var translationText: String {
        if !model.state.subtitles.translation.text.isEmpty {
            return model.state.subtitles.translation.text
        }
        switch model.state.status {
        case .connecting:
            return "正在连接阿里云"
        case .listening:
            return "正在聆听"
        case .stopping:
            return "正在结束"
        case let .error(message):
            return message
        case .idle:
            return "mimi"
        }
    }

    private var translationColor: Color {
        if case .error = model.state.status {
            return .red.opacity(0.9)
        }
        return model.state.subtitles.translation.isFinal
            ? .white
            : .white.opacity(0.62)
    }
}
