import AppKit
import MimiCore
import SwiftUI

struct SubtitleOverlayView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings
    @State private var isHovering = false
    private let accentColor = Color(red: 0.48, green: 0.66, blue: 1)

    var body: some View {
        overlayCanvas
    }

    private var overlayCanvas: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(.black.opacity(0.62))

            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [
                            .white.opacity(0.035),
                            .clear
                        ],
                        startPoint: .top,
                        endPoint: .center
                    )
                )

            VStack(spacing: 0) {
                WindowDragArea()
                    .frame(width: 120, height: 18)
                    .frame(maxWidth: .infinity)
                    .frame(height: model.isActive ? 38 : 24, alignment: .bottom)
                    .opacity(isHovering || model.showsOverlayControlsForUITesting ? 1 : 0)

                if visibleRows.isEmpty {
                    Spacer(minLength: 0)
                    Text(emptyStateText)
                        .font(.system(size: max(12, settings.fontSize * 0.68), weight: .medium))
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
                .stroke(
                    isHovering && !settings.isOverlayLocked
                        ? accentColor.opacity(0.34)
                        : .white.opacity(0.12),
                    lineWidth: isHovering && !settings.isOverlayLocked ? 1 : 0.75
                )
        }
        .overlay(alignment: .topLeading) {
            if model.isActive, let languageStatus {
                HStack(spacing: 3) {
                    Text(languageStatus.source)
                        .foregroundStyle(accentColor.opacity(isHovering ? 0.9 : 0.72))
                    Text(languageStatus.separator)
                        .foregroundStyle(.white.opacity(isHovering ? 0.48 : 0.32))
                    Text(languageStatus.target)
                        .foregroundStyle(.white.opacity(isHovering ? 0.7 : 0.5))
                }
                    .font(.system(size: 10, weight: .medium))
                    .lineLimit(1)
                    .padding(.horizontal, 8)
                    .frame(height: 20)
                    .background(
                        accentColor.opacity(isHovering ? 0.11 : 0.075),
                        in: Capsule()
                    )
                    .overlay {
                        Capsule()
                            .stroke(accentColor.opacity(isHovering ? 0.22 : 0.14), lineWidth: 0.5)
                    }
                    .padding(.leading, 12)
                    .padding(.top, 10)
                    .accessibilityLabel(
                        "Current languages: \(languageStatus.source) \(languageStatus.separator) \(languageStatus.target)"
                    )
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
        .animation(.easeOut(duration: 0.16), value: isHovering)
        .padding(6)
    }

    private var visibleRows: [SubtitleRow] {
        let subtitles = model.state.subtitles
        let visibleHistory = subtitles.history.filter {
            isSameLanguageMode || $0.source != $0.translation
        }
        var rows = visibleHistory.suffix(2).flatMap { pair in
            SubtitleTextSegmenter.segments(
                in: pair.translation,
                maximumCharacters: subtitleSegmentLength
            ).enumerated().map { index, text in
                SubtitleRow(
                    id: "history-\(pair.createdAt.timeIntervalSinceReferenceDate)-\(index)",
                    text: text,
                    createdAt: index == 0 ? pair.createdAt : nil
                )
            }
        }

        let currentLine = subtitles.translation
        let currentIsAlreadyInHistory = currentLine.isFinal
            && visibleHistory.last?.translation == currentLine.text
        if shouldShowCurrentSubtitle(currentLine), !currentIsAlreadyInHistory {
            rows.append(
                contentsOf: SubtitleTextSegmenter.segments(
                    in: currentLine.text,
                    maximumCharacters: subtitleSegmentLength
                ).enumerated().map { index, text in
                    SubtitleRow(
                        id: "current-\(index)",
                        text: text,
                        createdAt: nil
                    )
                }
            )
        }
        return Array(rows.suffix(3))
    }

    private func shouldShowCurrentSubtitle(_ line: SubtitleLine) -> Bool {
        !line.text.isEmpty
    }

    private var subtitleSegmentLength: Int {
        switch settings.targetLanguage {
        case .simplifiedChinese:
            28
        case .english:
            64
        case .japanese:
            32
        case .original:
            switch model.state.detectedLanguage?.code {
            case "en":
                64
            case "ja":
                32
            default:
                28
            }
        }
    }

    private var isSameLanguageMode: Bool {
        if settings.targetLanguage == .original {
            return true
        }

        if let detectedLanguage = model.state.detectedLanguage {
            return detectedLanguage.code == settings.targetLanguage.rawValue
        }

        return settings.sourceLanguage != .automatic
            && settings.sourceLanguage.rawValue == settings.targetLanguage.rawValue
    }

    private var hasSubtitleContent: Bool {
        let subtitles = model.state.subtitles
        return !subtitles.source.text.isEmpty
            || !subtitles.translation.text.isEmpty
            || !subtitles.history.isEmpty
    }

    private var languageStatus: (source: String, separator: String, target: String)? {
        let sourceName: String
        if let detected = model.state.detectedLanguage {
            sourceName = detected.displayName
        } else if settings.sourceLanguage != .automatic {
            sourceName = settings.sourceLanguage.displayName
        } else {
            sourceName = "识别中"
        }

        if settings.targetLanguage == .original {
            return (sourceName, "·", "原文")
        }
        return (sourceName, "→", settings.targetLanguage.displayName)
    }

    private var emptyStateText: String {
        switch model.state.status {
        case .connecting:
            "正在连接"
        case .listening:
            isWaitingForFinalTranslation
                ? "正在翻译"
                : "正在聆听，译文会保留在这里"
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

    private var isWaitingForFinalTranslation: Bool {
        guard !isSameLanguageMode else { return false }
        let subtitles = model.state.subtitles
        return subtitles.source.isFinal && !subtitles.translation.isFinal
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

private struct SubtitleRow: Identifiable, Equatable {
    let id: String
    let text: String
    let createdAt: Date?
}

private struct SubtitleTimeline: View {
    let rows: [SubtitleRow]
    let fontSize: Double

    private let bottomAnchor = "subtitle-timeline-bottom"

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView(.vertical) {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(rows.enumerated()), id: \.element.id) { index, row in
                        HStack(alignment: .firstTextBaseline, spacing: 8) {
                            if let createdAt = row.createdAt {
                                Text(createdAt.formatted(
                                    .dateTime
                                        .hour(.twoDigits(amPM: .omitted))
                                        .minute(.twoDigits)
                                ))
                                .font(.system(size: 9, weight: .medium, design: .monospaced))
                                .monospacedDigit()
                                .foregroundStyle(timestampColor(at: index))
                                .frame(width: 31, alignment: .trailing)
                            } else {
                                Color.clear
                                    .frame(width: 31, height: 1)
                            }

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
                        }
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
            .onChange(of: rows.count) {
                proxy.scrollTo(bottomAnchor, anchor: .bottom)
            }
        }
    }

    private func rowFontSize(at index: Int) -> Double {
        index == rows.count - 1 ? fontSize : max(12, fontSize * 0.82)
    }

    private func rowOpacity(at index: Int) -> Double {
        let distance = rows.count - 1 - index
        return switch distance {
        case 0: 1
        case 1: 0.58
        default: 0.34
        }
    }

    private func timestampColor(at index: Int) -> Color {
        let distance = rows.count - 1 - index
        let opacity = distance <= 1 ? 0.46 : 0.28
        return Color(red: 0.48, green: 0.66, blue: 1).opacity(opacity)
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
        private var trackingArea: NSTrackingArea?
        private var isPointerInside = false {
            didSet {
                guard oldValue != isPointerInside else { return }
                updateHandleAppearance()
                needsLayout = true
            }
        }

        override init(frame frameRect: NSRect) {
            super.init(frame: frameRect)
            wantsLayer = true
            handle.wantsLayer = true
            handle.layer?.cornerRadius = 1.5
            addSubview(handle)
            updateHandleAppearance()
        }

        required init?(coder: NSCoder) {
            nil
        }

        override func layout() {
            super.layout()
            let handleWidth: CGFloat = isPointerInside ? 40 : 32
            handle.frame = NSRect(
                x: (bounds.width - handleWidth) / 2,
                y: (bounds.height - 3) / 2,
                width: handleWidth,
                height: 3
            )
        }

        override func updateTrackingAreas() {
            if let trackingArea {
                removeTrackingArea(trackingArea)
            }

            let area = NSTrackingArea(
                rect: bounds,
                options: [.activeAlways, .mouseEnteredAndExited, .inVisibleRect],
                owner: self,
                userInfo: nil
            )
            addTrackingArea(area)
            trackingArea = area
            super.updateTrackingAreas()
        }

        override func resetCursorRects() {
            addCursorRect(bounds, cursor: .openHand)
        }

        override func mouseEntered(with event: NSEvent) {
            isPointerInside = true
        }

        override func mouseExited(with event: NSEvent) {
            isPointerInside = false
        }

        override func mouseDown(with event: NSEvent) {
            NSCursor.closedHand.push()
            defer { NSCursor.pop() }
            window?.performDrag(with: event)
        }

        private func updateHandleAppearance() {
            handle.layer?.backgroundColor = isPointerInside
                ? NSColor(calibratedRed: 0.48, green: 0.66, blue: 1, alpha: 0.78).cgColor
                : NSColor.white.withAlphaComponent(0.28).cgColor
        }
    }
}
