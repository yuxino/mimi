//! Windows virtual-desktop following for the subtitle surfaces.
//!
//! Tauri/Tao's `visible_on_all_workspaces` is a no-op on Windows. Windows has
//! no supported public API for pinning an arbitrary desktop window, but the
//! supported `IVirtualDesktopManager` can move Mimi-owned windows. A small
//! background follower therefore moves both subtitle surfaces to the desktop
//! of the current foreground window after a desktop switch.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use ::windows::core::IUnknown;
use ::windows::Win32::Foundation::HWND;
use ::windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use ::windows::Win32::UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager};
use ::windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindowVisible};
use tauri::Manager;

const WORKSPACE_POLL_INTERVAL: Duration = Duration::from_millis(150);

pub struct WindowsWorkspaceFollower {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Drop for WindowsWorkspaceFollower {
    fn drop(&mut self) {
        let (stopped, wake) = &*self.stop;
        *stopped.lock().unwrap() = true;
        wake.notify_all();
        if let Ok(thread) = self.thread.get_mut() {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}

pub fn install(app: &tauri::AppHandle) -> WindowsWorkspaceFollower {
    let stop = Arc::new((Mutex::new(false), Condvar::new()));
    let handles = ["overlay", "overlay-control"]
        .into_iter()
        .filter_map(|label| app.get_webview_window(label))
        .filter_map(|window| window.hwnd().ok())
        .map(|handle| handle.0 as isize)
        .collect::<Vec<_>>();

    let thread = if handles.len() == 2 {
        let stop_for_thread = Arc::clone(&stop);
        match std::thread::Builder::new()
            .name("mimi-virtual-desktop".to_string())
            .spawn(move || follow_current_workspace(handles, stop_for_thread))
        {
            Ok(thread) => Some(thread),
            Err(_) => {
                tracing::warn!("virtual desktop follower unavailable label=thread_start_failed");
                None
            }
        }
    } else {
        tracing::warn!("virtual desktop follower unavailable label=missing_overlay_window");
        None
    };

    WindowsWorkspaceFollower {
        stop,
        thread: Mutex::new(thread),
    }
}

fn follow_current_workspace(handles: Vec<isize>, stop: Arc<(Mutex<bool>, Condvar)>) {
    // The worker owns one multithreaded COM apartment and every COM interface
    // it creates. No interface or HWND wrapper crosses a thread boundary.
    let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if initialized.is_err() {
        tracing::warn!("virtual desktop follower unavailable label=com_initialization_failed");
        return;
    }

    let manager = unsafe {
        CoCreateInstance::<_, IVirtualDesktopManager>(
            &VirtualDesktopManager,
            None::<&IUnknown>,
            CLSCTX_ALL,
        )
    };
    let Ok(manager) = manager else {
        unsafe { CoUninitialize() };
        tracing::warn!("virtual desktop follower unavailable label=manager_creation_failed");
        return;
    };

    loop {
        if *stop.0.lock().unwrap() {
            break;
        }
        unsafe { follow_once(&manager, &handles) };
        let stopped = stop.0.lock().unwrap();
        let (stopped, _) = stop
            .1
            .wait_timeout_while(stopped, WORKSPACE_POLL_INTERVAL, |stopped| !*stopped)
            .unwrap();
        if *stopped {
            break;
        }
    }

    drop(manager);
    unsafe { CoUninitialize() };
}

unsafe fn follow_once(manager: &IVirtualDesktopManager, handles: &[isize]) {
    let windows = handles
        .iter()
        .copied()
        .map(|raw| HWND(raw as *mut _))
        .collect::<Vec<_>>();
    if !windows
        .iter()
        .copied()
        .any(|window| unsafe { IsWindowVisible(window).as_bool() })
    {
        return;
    }

    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0.is_null() {
        return;
    }
    if windows.contains(&foreground) {
        // Mimi can remain the foreground owner for a moment during a desktop
        // transition. It cannot tell us which destination desktop the user
        // selected, so wait for an unrelated foreground window instead.
        return;
    }
    let Ok(desktop_id) = (unsafe { manager.GetWindowDesktopId(foreground) }) else {
        return;
    };

    for window in windows {
        let on_current = unsafe { manager.IsWindowOnCurrentVirtualDesktop(window) }
            .ok()
            .map(|value| value.as_bool());
        if should_follow_workspace(true, false, true, on_current) {
            // MoveWindowToDesktop is the supported public API and is valid for
            // windows owned by this process. Moving the hidden control surface
            // alongside its visible owner keeps the next expansion local too.
            let _ = unsafe { manager.MoveWindowToDesktop(window, &desktop_id) };
        }
    }
}

fn should_follow_workspace(
    any_surface_visible: bool,
    foreground_is_mimi: bool,
    target_desktop_known: bool,
    window_on_current: Option<bool>,
) -> bool {
    any_surface_visible
        && !foreground_is_mimi
        && target_desktop_known
        && window_on_current == Some(false)
}

#[cfg(test)]
mod tests {
    use super::should_follow_workspace;

    #[test]
    fn visible_surface_follows_only_when_target_and_current_desktops_are_proven() {
        assert!(should_follow_workspace(true, false, true, Some(false)));
        assert!(!should_follow_workspace(true, false, true, Some(true)));
        assert!(!should_follow_workspace(false, false, true, Some(false)));
        assert!(!should_follow_workspace(true, true, true, Some(false)));
        assert!(!should_follow_workspace(true, false, false, Some(false)));
        assert!(!should_follow_workspace(true, false, true, None));
    }
}
