import MimiCore
import SwiftUI

struct SettingsView: View {
    @EnvironmentObject private var model: AppModel
    @EnvironmentObject private var settings: AppSettings
    @State private var showsServiceSettings = false
    @State private var credentialMessage: String?
    @State private var credentialMessageIsError = false

    private let accentColor = Color(red: 0.20, green: 0.46, blue: 0.94)

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                sessionCard
                subtitleCard
                serviceCard
            }
            .padding(20)
        }
        .frame(width: 560, height: 570)
        .background(Color(nsColor: .windowBackgroundColor))
        .onAppear {
            prepareListeningPreferences()
            showsServiceSettings = settings.workspaceID.isEmpty
                || settings.apiKey.isEmpty
                || settings.credentialLoadError != nil
        }
        .onDisappear {
            settings.persistPreferences()
            model.setOverlayLocked(settings.isOverlayLocked)
        }
    }

    private var sessionCard: some View {
        HStack(spacing: 14) {
            ZStack {
                Circle()
                    .fill(accentColor.opacity(0.12))
                    .frame(width: 42, height: 42)

                Image(systemName: "captions.bubble.fill")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(accentColor)
            }

            VStack(alignment: .leading, spacing: 4) {
                Text("实时字幕")
                    .font(.system(size: 17, weight: .semibold))

                HStack(spacing: 7) {
                    SettingsStatusIndicator(
                        color: sessionStatusColor,
                        isActive: model.state.status == .listening && !model.isPaused
                    )
                    Text(sessionStatusText)
                        .font(.system(size: 12.5, weight: .medium))
                        .foregroundStyle(sessionStatusColor)
                        .lineLimit(2)
                        .help(sessionStatusHelp)
                }
            }

            Spacer(minLength: 12)

            Button {
                Task {
                    if model.isActive {
                        await model.stop()
                    } else {
                        startListening()
                    }
                }
            } label: {
                Label(
                    model.isActive ? "停止" : "开始",
                    systemImage: model.isActive ? "stop.fill" : "play.fill"
                )
                .frame(minWidth: 62)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .tint(model.isActive ? .red : accentColor)
            .disabled(model.state.status == .stopping)
        }
        .padding(16)
        .background(cardBackground)
    }

    private var subtitleCard: some View {
        VStack(alignment: .leading, spacing: 15) {
            HStack {
                Text("字幕")
                    .font(.headline)

                Spacer()

                Label(translationBadgeText, systemImage: translationBadgeIcon)
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(accentColor)
                    .padding(.horizontal, 9)
                    .frame(height: 24)
                    .background(accentColor.opacity(0.10), in: Capsule())
            }

            VStack(alignment: .leading, spacing: 9) {
                Text("识别语言")
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.secondary)

                HStack(spacing: 8) {
                    ForEach(SourceLanguage.manualCases) { language in
                        sourceLanguageButton(language)
                    }
                }

                Text(sourceLanguageHelp)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Divider()

            settingsRow("翻译成") {
                Picker("翻译成", selection: $settings.targetLanguage) {
                    ForEach(TargetLanguage.allCases) { language in
                        Text(language.displayName).tag(language)
                    }
                }
                .labelsHidden()
                .frame(width: 168)
                .disabled(model.isActive || settings.sourceLanguage == .chinese)
                .onChange(of: settings.targetLanguage) {
                    settings.persistPreferences()
                }
            }

            Divider()

            settingsRow("翻译模式") {
                Picker("翻译模式", selection: $settings.translationMode) {
                    ForEach(TranslationMode.allCases) { mode in
                        Text(mode.displayName).tag(mode)
                    }
                }
                .labelsHidden()
                .frame(width: 168)
                .disabled(model.isActive)
                .onChange(of: settings.translationMode) {
                    settings.persistPreferences()
                }
            }

            Text(translationModeHelp)
                .font(.caption)
                .foregroundStyle(.secondary)

            Divider()

            settingsRow("字幕字号") {
                HStack(spacing: 10) {
                    Text("A")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Slider(value: $settings.fontSize, in: AppSettings.fontSizeRange, step: 1)
                        .frame(width: 178)
                    Text("\(Int(settings.fontSize))")
                        .monospacedDigit()
                        .frame(width: 24, alignment: .trailing)
                }
                .onChange(of: settings.fontSize) {
                    settings.persistPreferences()
                }
            }

            Divider()

            settingsRow("锁定字幕位置") {
                Toggle("锁定字幕位置", isOn: $settings.isOverlayLocked)
                    .labelsHidden()
                    .onChange(of: settings.isOverlayLocked) {
                        settings.persistPreferences()
                        model.setOverlayLocked(settings.isOverlayLocked)
                    }
            }

            Text("关闭锁定后，可拖动字幕顶部来移动位置，也可从边缘或四角调整大小。")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(16)
        .background(cardBackground)
    }

    private var serviceCard: some View {
        DisclosureGroup(isExpanded: $showsServiceSettings) {
            VStack(alignment: .leading, spacing: 12) {
                Divider()
                    .padding(.top, 4)

                VStack(alignment: .leading, spacing: 6) {
                    Text("工作空间 ID")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    TextField("输入 Workspace ID", text: $settings.workspaceID)
                        .textFieldStyle(.roundedBorder)
                        .onChange(of: settings.workspaceID) {
                            credentialMessage = nil
                        }
                }

                VStack(alignment: .leading, spacing: 6) {
                    Text("DashScope API Key")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    SecureField("输入 API Key", text: $settings.apiKey)
                        .textFieldStyle(.roundedBorder)
                        .onChange(of: settings.apiKey) {
                            credentialMessage = nil
                        }
                }

                HStack(alignment: .center, spacing: 10) {
                    Text("凭证仅保存在这台 Mac 上，API Key 会存入钥匙串。")
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    Spacer()

                    Button("保存凭证") {
                        saveCredentials()
                    }
                }

                if let credentialError = settings.credentialLoadError {
                    credentialFeedback(
                        "无法读取已保存的 API Key：\(credentialError)",
                        isError: true
                    )
                }

                if let credentialMessage {
                    credentialFeedback(
                        credentialMessage,
                        isError: credentialMessageIsError
                    )
                }
            }
            .padding(.top, 8)
        } label: {
            HStack(spacing: 10) {
                Image(systemName: "key.fill")
                    .foregroundStyle(.secondary)
                Text("服务设置")
                    .font(.headline)
                Spacer()
                if credentialsAreConfigured {
                    Text("已配置")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .padding(16)
        .background(cardBackground)
    }

    private var cardBackground: some View {
        RoundedRectangle(cornerRadius: 14, style: .continuous)
            .fill(Color.secondary.opacity(0.075))
            .overlay {
                RoundedRectangle(cornerRadius: 14, style: .continuous)
                    .stroke(Color.primary.opacity(0.045), lineWidth: 0.5)
            }
    }

    private func sourceLanguageButton(_ language: SourceLanguage) -> some View {
        let isSelected = settings.sourceLanguage == language

        return Button {
            Task {
                await model.switchSourceLanguage(to: language, using: settings)
            }
        } label: {
            HStack(spacing: 6) {
                Text(sourceLanguageButtonTitle(language))
                    .font(.system(size: 13, weight: isSelected ? .semibold : .medium))
                if isSelected {
                    Image(systemName: "checkmark.circle.fill")
                        .font(.system(size: 11, weight: .semibold))
                }
            }
            .foregroundStyle(isSelected ? accentColor : Color.primary.opacity(0.78))
            .frame(maxWidth: .infinity)
            .frame(height: 34)
            .background(
                isSelected ? accentColor.opacity(0.12) : Color.primary.opacity(0.035),
                in: RoundedRectangle(cornerRadius: 9, style: .continuous)
            )
            .overlay {
                RoundedRectangle(cornerRadius: 9, style: .continuous)
                    .stroke(
                        isSelected ? accentColor.opacity(0.34) : Color.primary.opacity(0.07),
                        lineWidth: 0.75
                    )
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(isChangingSession)
        .help(sourceLanguageButtonHelp(language))
        .accessibilityLabel(
            isSelected
                ? "\(sourceLanguageButtonTitle(language))，当前识别语言"
                : "切换到 \(sourceLanguageButtonTitle(language))"
        )
    }

    private func settingsRow<Content: View>(
        _ title: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        HStack(spacing: 16) {
            Text(title)
                .font(.system(size: 13.5, weight: .medium))
            Spacer()
            content()
        }
        .frame(minHeight: 30)
    }

    private func credentialFeedback(_ message: String, isError: Bool) -> some View {
        Label(message, systemImage: isError ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
            .font(.caption)
            .foregroundStyle(isError ? Color.red : Color.green)
    }

    private func prepareListeningPreferences() {
        if settings.sourceLanguage == .automatic {
            settings.sourceLanguage = .japanese
        }
        if settings.sourceLanguage == .chinese {
            settings.targetLanguage = .original
        }
        settings.persistPreferences()
    }

    private func startListening() {
        prepareListeningPreferences()
        model.setOverlayLocked(settings.isOverlayLocked)
        Task {
            await model.start(using: settings)
        }
    }

    private func saveCredentials() {
        prepareListeningPreferences()
        do {
            try settings.save()
            credentialMessage = "凭证已安全保存。"
            credentialMessageIsError = false
        } catch {
            credentialMessage = error.localizedDescription
            credentialMessageIsError = true
        }
    }

    private var credentialsAreConfigured: Bool {
        !settings.workspaceID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !settings.apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var isChangingSession: Bool {
        model.state.status == .connecting || model.state.status == .stopping
    }

    private var sourceLanguageHelp: String {
        if settings.sourceLanguage == .chinese {
            return model.state.status == .listening
                ? "正在识别中文，只显示原文，不发送翻译请求。"
                : "只识别并显示中文原文，不发送翻译请求。"
        }
        if model.state.status == .listening {
            return "切换后会自动重新连接，继续使用当前翻译模式。"
        } else {
            return "选择主要语种，整句翻译更准确。"
        }
    }

    private func sourceLanguageButtonTitle(_ language: SourceLanguage) -> String {
        language == .chinese ? "中文原文" : language.displayName
    }

    private func sourceLanguageButtonHelp(_ language: SourceLanguage) -> String {
        language == .chinese
            ? "切换到中文识别，只显示原文"
            : "切换到 \(language.displayName) 识别，保持当前翻译模式"
    }

    private var translationBadgeText: String {
        settings.sourceLanguage == .chinese && settings.targetLanguage == .original
            ? "仅显示原文"
            : "\(settings.translationMode.displayName)翻译"
    }

    private var translationBadgeIcon: String {
        settings.sourceLanguage == .chinese && settings.targetLanguage == .original
            ? "text.quote"
            : "sparkles"
    }

    private var sessionStatusText: String {
        if model.isPaused {
            return "已暂停"
        }

        return switch model.state.status {
        case .idle:
            "准备就绪"
        case .connecting:
            "正在连接\(settings.translationMode.displayName)翻译…"
        case .listening:
            "正在识别并翻译"
        case .stopping:
            "正在停止…"
        case .error:
            "翻译暂时不可用，请重试"
        }
    }

    private var sessionStatusHelp: String {
        if case let .error(message) = model.state.status {
            return message
        }
        return sessionStatusText
    }

    private var translationModeHelp: String {
        switch settings.translationMode {
        case .turbo:
            "极速：边识别边用快模型翻译，速度优先，一句话说完即定稿。"
        case .highQuality:
            "高质量：整句翻译完成后再显示，最准确，稍有延迟。"
        case .lowLatency:
            "低延迟：快速预览 + 高质量定稿，速度和准确度兼顾。"
        }
    }

    private var sessionStatusColor: Color {
        if model.isPaused {
            return .orange
        }

        return switch model.state.status {
        case .listening:
            .green
        case .connecting:
            accentColor
        case .error:
            .red
        default:
            .secondary
        }
    }
}

private struct SettingsStatusIndicator: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    let color: Color
    let isActive: Bool

    var body: some View {
        TimelineView(.animation(minimumInterval: 0.8, paused: !isActive || reduceMotion)) { context in
            let isExpanded = isActive
                && Int(context.date.timeIntervalSinceReferenceDate * 1.25).isMultiple(of: 2)

            ZStack {
                Circle()
                    .fill(color.opacity(isExpanded ? 0.16 : 0))
                    .frame(width: isExpanded ? 16 : 8, height: isExpanded ? 16 : 8)

                Circle()
                    .fill(color)
                    .frame(width: 7, height: 7)
            }
            .frame(width: 16, height: 16)
            .animation(
                reduceMotion ? nil : .easeInOut(duration: 0.55),
                value: isExpanded
            )
        }
    }
}
