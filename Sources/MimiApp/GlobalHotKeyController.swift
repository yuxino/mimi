import Carbon.HIToolbox
import Foundation
import MimiCore

final class GlobalHotKeyController: @unchecked Sendable {
    private static let signature: OSType = 0x6D696D69
    private static let identifier: UInt32 = 1

    private let action: @MainActor @Sendable () -> Void
    private var eventHandler: EventHandlerRef?
    private var hotKey: EventHotKeyRef?
    private var isPressed = false
    private var lastTriggerUptime = -Double.infinity

    init(action: @escaping @MainActor @Sendable () -> Void) {
        self.action = action
        install()
    }

    deinit {
        if let hotKey {
            UnregisterEventHotKey(hotKey)
        }
        if let eventHandler {
            RemoveEventHandler(eventHandler)
        }
    }

    private func install() {
        var eventTypes = [
            EventTypeSpec(
                eventClass: OSType(kEventClassKeyboard),
                eventKind: UInt32(kEventHotKeyPressed)
            ),
            EventTypeSpec(
                eventClass: OSType(kEventClassKeyboard),
                eventKind: UInt32(kEventHotKeyReleased)
            )
        ]

        let handlerStatus = eventTypes.withUnsafeMutableBufferPointer { eventTypes in
            InstallEventHandler(
                GetApplicationEventTarget(),
                { _, event, userData in
                    guard let event, let userData else { return noErr }

                    var hotKeyID = EventHotKeyID()
                    let status = GetEventParameter(
                        event,
                        EventParamName(kEventParamDirectObject),
                        EventParamType(typeEventHotKeyID),
                        nil,
                        MemoryLayout<EventHotKeyID>.size,
                        nil,
                        &hotKeyID
                    )
                    guard status == noErr,
                        hotKeyID.signature == GlobalHotKeyController.signature,
                        hotKeyID.id == GlobalHotKeyController.identifier
                    else {
                        return noErr
                    }

                    let controller = Unmanaged<GlobalHotKeyController>
                        .fromOpaque(userData)
                        .takeUnretainedValue()
                    if GetEventKind(event) == UInt32(kEventHotKeyReleased) {
                        controller.isPressed = false
                        return noErr
                    }
                    guard !controller.isPressed else { return noErr }
                    controller.isPressed = true
                    let now = ProcessInfo.processInfo.systemUptime
                    guard now - controller.lastTriggerUptime >= 2 else { return noErr }
                    controller.lastTriggerUptime = now

                    PipelineDiagnostics.log("global hotkey triggered")
                    Task { @MainActor in
                        controller.action()
                    }
                    return noErr
                },
                eventTypes.count,
                eventTypes.baseAddress,
                Unmanaged.passUnretained(self).toOpaque(),
                &eventHandler
            )
        }
        guard handlerStatus == noErr else {
            PipelineDiagnostics.log("global hotkey handler failed status=\(handlerStatus)")
            return
        }

        let hotKeyID = EventHotKeyID(
            signature: Self.signature,
            id: Self.identifier
        )
        let registrationStatus = RegisterEventHotKey(
            UInt32(kVK_Space),
            UInt32(cmdKey | shiftKey),
            hotKeyID,
            GetApplicationEventTarget(),
            0,
            &hotKey
        )

        if registrationStatus != noErr, let eventHandler {
            PipelineDiagnostics.log("global hotkey registration failed status=\(registrationStatus)")
            RemoveEventHandler(eventHandler)
            self.eventHandler = nil
        } else {
            PipelineDiagnostics.log("global hotkey registered")
        }
    }
}
