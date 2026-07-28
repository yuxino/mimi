import AppKit
import MimiCore
import SwiftUI

struct SubtitleOverlayView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings
    @State private var isHovering = false

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(.black.opacity(0.62))

            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [.white.opacity(0.035), .clear],
                        startPoint: .top,
                        endPoint: .center
                    )
                )

            VStack(spacing: 0) {
                WindowDragArea()
                    .frame(height: 14)
                    .opacity(isHovering || model.showsOverlayControlsForUITesting ? 1 : 0)

                if visibleRows.isEmpty {
                    Spacer(minLength: 0)
                    Text(emptyStateText)
                        .font(.system(size: max(15, settings.fontSize * 0.68), weight: .medium))
                        .foregroundStyle(emptyStateColor)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal, 24)
                    Spacer(minLength: 0)
                } else {
                    SubtitleTimeline(
                        rows: visibleRows,
                        fontSize: settings.fontSize
                    )
                }
            }
            .padding(5)
        }
        .overlay {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .stroke(.white.opacity(0.12), lineWidth: 0.75)
        }
        .overlay(alignment: .topLeading) {
            if model.isActive, let languageStatusText {
                Text(languageStatusText)
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(.white.opacity(isHovering ? 0.66 : 0.46))
                    .lineLimit(1)
                    .padding(.horizontal, 8)
                    .frame(height: 20)
                    .background(
                        .black.opacity(0.24),
                        in: Capsule()
                    )
                    .overlay {
                        Capsule()
                            .stroke(.white.opacity(0.08), lineWidth: 0.5)
                    }
                    .padding(.leading, 12)
                    .padding(.top, 10)
                    .accessibilityLabel("Current languages: \(languageStatusText)")
            }
        }
        .overlay(alignment: .topTrailing) {
            if !settings.isOverlayLocked
                && (isHovering || model.showsOverlayControlsForUITesting) {
                HStack(spacing: 4) {
                    if hasSubtitleContent {
                        OverlayControlButton(
                            systemImage: "eraser.fill",
                            label: "Clear subtitles"
                        ) {
                            model.clearSubtitles()
                        }
                    }

                    OverlayControlButton(
                        systemImage: "gearshape.fill",
                        label: "Open mimi Settings"
                    ) {
                        model.showSettings()
                    }
                }
                .padding(10)
            }
        }
        .onHover { isHovering = $0 }
        .padding(6)
    }

    private var visibleRows: [SubtitleRow] {
        let subtitles = model.state.subtitles
        var rows = subtitles.history.suffix(2).map {
            SubtitleRow(text: $0.translation)
        }
        let current = subtitles.translation.text

        if !current.isEmpty, rows.last?.text != current {
            rows.append(SubtitleRow(text: current))
        }
        return Array(rows.suffix(3))
    }

    private var hasSubtitleContent: Bool {
        let subtitles = model.state.subtitles
        return !subtitles.source.text.isEmpty
            || !subtitles.translation.text.isEmpty
            || !subtitles.history.isEmpty
    }

    private var languageStatusText: String? {
        let sourceName: String
        if let detected = model.state.detectedLanguage {
            sourceName = detected.displayName
        } else if settings.sourceLanguage != .automatic {
            sourceName = settings.sourceLanguage.displayName
        } else {
            sourceName = "识别中"
        }

        if settings.targetLanguage == .original {
            return "\(sourceName) · 原文"
        }
        return "\(sourceName) → \(settings.targetLanguage.displayName)"
    }

    private var emptyStateText: String {
        switch model.state.status {
        case .connecting:
            "正在连接"
        case .listening:
            "正在聆听，译文会保留在这里"
        case .stopping:
            "正在结束"
        case let .error(message):
            message
        case .idle:
            "mimi"
        }
    }

    private var emptyStateColor: Color {
        if case .error = model.state.status { return .red.opacity(0.9) }
        return .white.opacity(0.5)
    }
}

private struct OverlayControlButton: View {
    let systemImage: String
    let label: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 10, weight: .medium))
                .foregroundStyle(.white.opacity(0.68))
                .frame(width: 24, height: 24)
                .background(.black.opacity(0.28), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        }
        .buttonStyle(.plain)
        .help(label)
        .accessibilityLabel(label)
    }
}

private struct SubtitleRow: Equatable {
    let text: String
}

private struct SubtitleTimeline: View {
    let rows: [SubtitleRow]
    let fontSize: Double

    private let bottomAnchor = "subtitle-timeline-bottom"

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical) {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                        Text(row.text)
                            .font(.system(
                                size: rowFontSize(at: index),
                                weight: index == rows.count - 1 ? .medium : .regular
                            ))
                            .foregroundStyle(.white.opacity(rowOpacity(at: index)))
                            .multilineTextAlignment(.leading)
                            .fixedSize(horizontal: false, vertical: true)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .lineSpacing(2)
                            .padding(.horizontal, 18)
                            .padding(.vertical, index == rows.count - 1 ? 7 : 5)
                    }

                    Color.clear
                        .frame(height: 1)
                        .id(bottomAnchor)
                }
            }
            .scrollIndicators(.automatic)
            .onAppear {
                proxy.scrollTo(bottomAnchor, anchor: .bottom)
            }
            .onChange(of: rows) {
                proxy.scrollTo(bottomAnchor, anchor: .bottom)
            }
        }
    }

    private func rowFontSize(at index: Int) -> Double {
        index == rows.count - 1 ? fontSize : max(15, fontSize * 0.82)
    }

    private func rowOpacity(at index: Int) -> Double {
        let distance = rows.count - 1 - index
        return switch distance {
        case 0: 1
        case 1: 0.58
        default: 0.34
        }
    }
}

private struct WindowDragArea: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = DraggableNSView()
        view.toolTip = "Drag to move subtitles"
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {}

    private final class DraggableNSView: NSView {
        private let handle = NSView()

        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            wantsLayer = true
            handle.wantsLayer = true
            handle.layer?.backgroundColor = NSColor.white.withAlphaComponent(0.28).cgColor
            handle.layer?.cornerRadius = 1.5
            addSubview(handle)
        }

        required init?(coder: NSCoder) {
            nil
        }

        override func layout() {
            super.layout()
            handle.frame = NSRect(
                x: (bounds.width - 32) / 2,
                y: (bounds.height - 3) / 2,
                width: 32,
                height: 3
            )
        }

        override func mouseDown(with event: NSEvent) {
            window?.performDrag(with: event)
        }
    }
}
