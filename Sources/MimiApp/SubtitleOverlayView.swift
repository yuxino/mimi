import AppKit
import MimiCore
import SwiftUI

struct SubtitleOverlayView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings
    @State private var isHovering = false
    @State private var showsLanguagePicker = false
    private let accentColor = Color(red: 0.48, green: 0.66, blue: 1)

    var body: some View {
        Group {
            if model.isOverlayCollapsed {
                compactOverlay
                    .transition(.opacity.combined(with: .scale(scale: 0.96)))
            } else {
                overlayCanvas
                    .transition(.opacity.combined(with: .scale(scale: 0.98)))
            }
        }
        .animation(.easeInOut(duration: 0.18), value: model.isOverlayCollapsed)
    }

    private var compactOverlay: some View {
        HStack(spacing: 8) {
            WindowDragArea(onDoubleClick: model.toggleOverlayCollapsed)
                .frame(width: 42, height: 30)

            RecognitionActivityIndicator(phase: activityPhase)

            Text(activityPhase.accessibilityLabel)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(.white.opacity(0.76))
                .lineLimit(1)

            Spacer(minLength: 4)

            OverlayControlButton(
                systemImage: "chevron.down",
                label: "展开字幕"
            ) {
                model.setOverlayCollapsed(false)
            }
        }
        .padding(.horizontal, 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .fill(.black.opacity(0.68))
                .overlay {
                    RoundedRectangle(cornerRadius: 14, style: .continuous)
                        .fill(
                            LinearGradient(
                                colors: [.white.opacity(0.05), .clear],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                        )
                }
        }
        .overlay {
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(accentColor.opacity(isHovering ? 0.3 : 0.16), lineWidth: 0.75)
        }
        .onHover { isHovering = $0 }
        .padding(6)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("字幕已收起，\(activityPhase.accessibilityLabel)")
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
                WindowDragArea(onDoubleClick: model.toggleOverlayCollapsed)
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
                Button {
                    showsLanguagePicker.toggle()
                } label: {
                    HStack(spacing: 4) {
                        RecognitionActivityIndicator(phase: activityPhase)

                        if isWaitingForFinalTranslation {
                            Text("翻译中")
                                .foregroundStyle(activityPhase.color.opacity(0.96))
                            Text("·")
                                .foregroundStyle(.white.opacity(0.34))
                        }

                        Text(languageStatus.source)
                            .foregroundStyle(accentColor.opacity(isHovering ? 0.96 : 0.8))
                        Text(languageStatus.separator)
                            .foregroundStyle(.white.opacity(isHovering ? 0.48 : 0.32))
                        Text(languageStatus.target)
                            .foregroundStyle(.white.opacity(isHovering ? 0.74 : 0.56))

                        if settings.targetLanguage.translatesAudio {
                            Rectangle()
                                .fill(.white.opacity(0.14))
                                .frame(width: 0.5, height: 9)

                            Image(systemName: "sparkles")
                                .font(.system(size: 7, weight: .semibold))
                                .foregroundStyle(accentColor.opacity(0.74))
                            Text("高质量")
                                .foregroundStyle(.white.opacity(isHovering ? 0.72 : 0.52))
                        }
                        Image(systemName: "chevron.down")
                            .font(.system(size: 6, weight: .bold))
                            .foregroundStyle(.white.opacity(isHovering ? 0.5 : 0.3))
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
                }
                .buttonStyle(.plain)
                .fixedSize(horizontal: true, vertical: false)
                .disabled(model.state.status != .listening)
                .help(languagePickerHelp)
                .accessibilityLabel(
                    languagePickerAccessibilityLabel(languageStatus)
                )
                .popover(isPresented: $showsLanguagePicker, arrowEdge: .top) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("识别语言")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(.secondary)
                            .padding(.horizontal, 8)
                            .padding(.bottom, 2)

                        ForEach(SourceLanguage.manualCases) { language in
                            Button {
                                showsLanguagePicker = false
                                Task {
                                    await model.switchSourceLanguage(to: language, using: settings)
                                }
                            } label: {
                                HStack(spacing: 8) {
                                    Text(sourceLanguageButtonTitle(language))
                                    Spacer(minLength: 12)
                                    if settings.sourceLanguage == language {
                                        Image(systemName: "checkmark")
                                            .foregroundStyle(accentColor)
                                    }
                                }
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                            .padding(.horizontal, 8)
                            .frame(height: 26)
                            .accessibilityLabel(
                                settings.sourceLanguage == language
                                    ? "\(sourceLanguageButtonTitle(language))，当前语言"
                                    : "切换到 \(sourceLanguageButtonTitle(language))"
                            )
                        }
                    }
                    .padding(8)
                    .frame(width: 156)
                }
                .padding(.leading, 12)
                .padding(.top, 10)
            }
        }
        .overlay(alignment: .topTrailing) {
            if model.isActive, !settings.isOverlayLocked {
                HStack(spacing: 4) {
                    OverlayControlButton(
                        systemImage: "chevron.up",
                        label: "收起字幕"
                    ) {
                        model.setOverlayCollapsed(true)
                    }

                    if isHovering || model.showsOverlayControlsForUITesting {
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
        let visibleHistory = subtitles.history
        var rows = visibleHistory.flatMap { pair in
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
        return rows
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
        let sourceName = settings.sourceLanguage.statusDisplayName(
            detectedLanguage: model.state.detectedLanguage,
            targetLanguage: settings.targetLanguage
        )

        if settings.targetLanguage == .original {
            return (sourceName, "·", "原文")
        }
        return (sourceName, "→", settings.targetLanguage.displayName)
    }

    private var activityPhase: OverlayActivityPhase {
        switch model.state.status {
        case .connecting, .stopping:
            return .connecting
        case .listening:
            if isWaitingForFinalTranslation {
                return .translating
            }
            if !model.state.subtitles.source.text.isEmpty,
                !model.state.subtitles.source.isFinal {
                return .recognizing
            }
            return .listening
        case .idle, .error:
            return .listening
        }
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
        return model.state.isTranslationPending
    }

    private func sourceLanguageButtonTitle(_ language: SourceLanguage) -> String {
        language == .chinese ? "中文原文" : language.displayName
    }

    private var languagePickerHelp: String {
        settings.targetLanguage.translatesAudio
            ? "切换识别语言（保持高质量翻译）"
            : "切换识别语言"
    }

    private func languagePickerAccessibilityLabel(
        _ status: (source: String, separator: String, target: String)
    ) -> String {
        let mode = settings.targetLanguage.translatesAudio ? "高质量翻译" : "只显示原文"
        return "\(activityPhase.accessibilityLabel)，当前语言：\(status.source) \(status.separator) \(status.target)，\(mode)。打开以切换识别语言。"
    }
}

private enum OverlayActivityPhase {
    case connecting
    case listening
    case recognizing
    case translating

    var accessibilityLabel: String {
        switch self {
        case .connecting:
            "正在连接"
        case .listening:
            "正在聆听"
        case .recognizing:
            "正在识别"
        case .translating:
            "正在翻译"
        }
    }

    var color: Color {
        switch self {
        case .connecting:
            .white.opacity(0.5)
        case .listening:
            Color(red: 0.48, green: 0.66, blue: 1).opacity(0.62)
        case .recognizing:
            Color(red: 0.48, green: 0.66, blue: 1)
        case .translating:
            Color(red: 0.72, green: 0.58, blue: 1)
        }
    }

    var animationSpeed: Double {
        switch self {
        case .connecting:
            2.6
        case .listening:
            1.8
        case .recognizing:
            7.2
        case .translating:
            4.4
        }
    }

    var amplitude: Double {
        switch self {
        case .connecting:
            3
        case .listening:
            2
        case .recognizing:
            6
        case .translating:
            4
        }
    }
}

private struct RecognitionActivityIndicator: View {
    let phase: OverlayActivityPhase

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: reduceMotion ? 1 : 1 / 24)) { timeline in
            let time = reduceMotion ? 0 : timeline.date.timeIntervalSinceReferenceDate
            ZStack {
                if case .translating = phase {
                    Circle()
                        .fill(phase.color.opacity(innerGlowOpacity(time: time)))
                        .frame(width: innerGlowSize(time: time), height: innerGlowSize(time: time))

                    Circle()
                        .stroke(phase.color.opacity(outerRingOpacity(time: time)), lineWidth: 1)
                        .frame(width: outerRingSize(time: time), height: outerRingSize(time: time))
                }

                HStack(alignment: .center, spacing: 1.25) {
                    ForEach(0..<3, id: \.self) { index in
                        Capsule()
                            .fill(phase.color)
                            .frame(width: 1.75, height: barHeight(index: index, time: time))
                    }
                }
                .frame(width: 8, height: 10)
            }
            .frame(width: 18, height: 18)
        }
        .accessibilityHidden(true)
    }

    private func barHeight(index: Int, time: TimeInterval) -> Double {
        if reduceMotion {
            return [3.0, 6.0, 4.0][index]
        }
        let wave = (sin(time * phase.animationSpeed + Double(index) * 1.7) + 1) / 2
        return 2 + wave * phase.amplitude
    }

    private func pulseProgress(time: TimeInterval) -> Double {
        guard !reduceMotion else { return 0.45 }
        return (sin(time * 3.2) + 1) / 2
    }

    private func innerGlowSize(time: TimeInterval) -> Double {
        10 + pulseProgress(time: time) * 4
    }

    private func innerGlowOpacity(time: TimeInterval) -> Double {
        0.13 + pulseProgress(time: time) * 0.15
    }

    private func outerRingSize(time: TimeInterval) -> Double {
        10 + pulseProgress(time: time) * 8
    }

    private func outerRingOpacity(time: TimeInterval) -> Double {
        0.34 - pulseProgress(time: time) * 0.22
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
    let onDoubleClick: () -> Void

    func makeNSView(context: Context) -> NSView {
        let view = DraggableNSView(onDoubleClick: onDoubleClick)
        view.toolTip = "拖动字幕；双击收起或展开"
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        guard let view = nsView as? DraggableNSView else { return }
        view.onDoubleClick = onDoubleClick
    }

    private final class DraggableNSView: NSView {
        private let handle = NSView()
        var onDoubleClick: () -> Void
        private var trackingArea: NSTrackingArea?
        private var isPointerInside = false {
            didSet {
                guard oldValue != isPointerInside else { return }
                updateHandleAppearance()
                needsLayout = true
            }
        }

        init(onDoubleClick: @escaping () -> Void) {
            self.onDoubleClick = onDoubleClick
            super.init(frame: .zero)
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
            if event.clickCount == 2 {
                onDoubleClick()
                return
            }
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
