use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    mem::{size_of, size_of_val},
    sync::mpsc,
};
use windows::{
    core::{s, w, Interface},
    System::{DispatcherQueue, DispatcherQueueController},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
            DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_HOSTBACKDROPBRUSH,
        },
        Graphics::Gdi::ScreenToClient,
        System::{
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            WinRT::{
                Composition::ICompositorDesktopInterop, CreateDispatcherQueueController,
                DispatcherQueueOptions, RoInitialize, RoUninitialize, DQTAT_COM_NONE,
                DQTYPE_THREAD_CURRENT, RO_INIT_SINGLETHREADED,
            },
        },
        UI::{
            HiDpi::GetDpiForWindow,
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                GetClientRect, GWL_EXSTYLE, GWL_STYLE, HTTRANSPARENT, STYLESTRUCT, WM_NCACTIVATE,
                WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST, WM_NCPAINT, WM_STYLECHANGING,
                WS_CAPTION, WS_EX_APPWINDOW, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW,
                WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
            },
        },
    },
    UI::Composition::{
        CompositionRoundedRectangleGeometry, Compositor, Desktop::DesktopWindowTarget, SpriteVisual,
    },
};
use windows_numerics::Vector2;

const WIDGET_FRAME_SUBCLASS: usize = 0x56454c41;
const WCA_ACCENT_POLICY: i32 = 19;
const ACCENT_DISABLED: i32 = 0;

struct ApartmentGuard(bool);

impl Drop for ApartmentGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { RoUninitialize() };
        }
    }
}

struct CompositionHost {
    compositor: Compositor,
    _queue: Option<DispatcherQueueController>,
    _apartment: ApartmentGuard,
}

struct BackdropSurface {
    target: DesktopWindowTarget,
    visual: SpriteVisual,
    geometry: CompositionRoundedRectangleGeometry,
}

#[derive(Clone, Copy)]
struct RoundedBounds {
    width: f64,
    height: f64,
    radius: f64,
}

impl RoundedBounds {
    fn contains(self, x: f64, y: f64) -> bool {
        if x < 0.0 || y < 0.0 || x >= self.width || y >= self.height {
            return false;
        }
        let nearest_x = x.clamp(self.radius, self.width - self.radius);
        let nearest_y = y.clamp(self.radius, self.height - self.radius);
        (x - nearest_x).powi(2) + (y - nearest_y).powi(2) <= self.radius.powi(2)
    }
}

impl Drop for BackdropSurface {
    fn drop(&mut self) {
        let _ = self.target.Close();
    }
}

// All Composition objects remain on the HWND-owning UI thread. One compositor
// serves the widgets; each HWND gets a clipped brush underneath its WebView.
thread_local! {
    static HOST: RefCell<Option<CompositionHost>> = const { RefCell::new(None) };
    static SURFACES: RefCell<HashMap<usize, BackdropSurface>> = RefCell::new(HashMap::new());
    static CORNERS: RefCell<HashMap<usize, RoundedBounds>> = RefCell::new(HashMap::new());
}

#[repr(C)]
struct AccentPolicy {
    state: i32,
    flags: i32,
    gradient_color: u32,
    animation_id: i32,
}

#[repr(C)]
struct CompositionAttributeData {
    attribute: i32,
    data: *mut c_void,
    size: usize,
}

type SetCompositionAttribute =
    unsafe extern "system" fn(HWND, *mut CompositionAttributeData) -> windows::core::BOOL;

// Tao may restore WS_CAPTION when showing a window or changing window flags.
// Control non-client drawing continuously, instead of clearing styles just once.
unsafe extern "system" fn widget_frame_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_id: usize,
    _data: usize,
) -> LRESULT {
    match message {
        WM_NCHITTEST => unsafe {
            // Keep hit testing independent of visual clipping: invisible corners
            // should not act as resize handles or activate the widget.
            let mut point = POINT {
                x: (lparam.0 as u16 as i16) as i32,
                y: ((lparam.0 as usize >> 16) as u16 as i16) as i32,
            };
            if ScreenToClient(hwnd, &mut point).as_bool()
                && CORNERS.with(|corners| {
                    corners
                        .borrow()
                        .get(&(hwnd.0 as usize))
                        .is_some_and(|bounds| {
                            !bounds.contains(f64::from(point.x), f64::from(point.y))
                        })
                })
            {
                return LRESULT(HTTRANSPARENT as isize);
            }
            DefSubclassProc(hwnd, message, wparam, lparam)
        },
        WM_STYLECHANGING => unsafe {
            let styles = &mut *(lparam.0 as *mut STYLESTRUCT);
            if wparam.0 as i32 == GWL_STYLE.0 {
                styles.styleNew &=
                    !(WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0);
            } else if wparam.0 as i32 == GWL_EXSTYLE.0 {
                styles.styleNew =
                    (styles.styleNew | WS_EX_TOOLWINDOW.0 | WS_EX_NOREDIRECTIONBITMAP.0)
                        & !(WS_EX_APPWINDOW.0 | WS_EX_WINDOWEDGE.0);
            }
            DefSubclassProc(hwnd, message, wparam, lparam)
        },
        WM_NCCALCSIZE | WM_NCPAINT => LRESULT(0),
        // Forward activation to Tao so focus events still work. -1 tells the
        // default window procedure to skip non-client activation painting.
        WM_NCACTIVATE => unsafe { DefSubclassProc(hwnd, message, wparam, LPARAM(-1)) },
        WM_NCDESTROY => unsafe {
            let _ = CORNERS.try_with(|corners| {
                corners.borrow_mut().remove(&(hwnd.0 as usize));
            });
            let _ = SURFACES.try_with(|surfaces| {
                surfaces.borrow_mut().remove(&(hwnd.0 as usize));
            });
            let _ = RemoveWindowSubclass(hwnd, Some(widget_frame_proc), subclass_id);
            DefSubclassProc(hwnd, message, wparam, lparam)
        },
        _ => unsafe { DefSubclassProc(hwnd, message, wparam, lparam) },
    }
}

// SetWindowSubclass must run on the thread that owns the HWND. Wait for the
// actual native result so callers do not report success after only scheduling it.
pub fn apply(window: &tauri::WebviewWindow, transparency: u8) -> Result<(), String> {
    on_window_thread(window, move |hwnd| unsafe {
        apply_to_hwnd(hwnd, transparency)
    })
}

pub fn update_backdrop_geometry(window: &tauri::WebviewWindow, radius: u8) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|error| error.to_string())?;
    on_window_thread(window, move |hwnd| update_geometry(hwnd, radius, scale))
}

fn on_window_thread(
    window: &tauri::WebviewWindow,
    operation: impl FnOnce(HWND) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let owned_window = window.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    window
        .run_on_main_thread(move || {
            let result = owned_window
                .hwnd()
                .map_err(|error| error.to_string())
                .and_then(operation);
            let _ = sender.send(result);
        })
        .map_err(|error| error.to_string())?;
    receiver
        .recv()
        .map_err(|error| format!("无法接收组件材质设置结果：{error}"))?
}

unsafe fn apply_to_hwnd(hwnd: HWND, transparency: u8) -> Result<(), String> {
    if !unsafe { SetWindowSubclass(hwnd, Some(widget_frame_proc), WIDGET_FRAME_SUBCLASS, 0) }
        .as_bool()
    {
        return Err("无法安装组件无边框窗口处理。".to_string());
    }

    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &DWMNCRP_DISABLED as *const _ as *const c_void,
            size_of_val(&DWMNCRP_DISABLED) as u32,
        )
        .map_err(|error| format!("无法关闭组件系统边框：{error}"))?;

        // Remove the focus-sensitive system Acrylic backplate. Windows 10 does
        // not implement this attribute, so its unsupported result is harmless.
        let no_system_backdrop = 1_i32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &no_system_backdrop as *const _ as *const c_void,
            size_of::<i32>() as u32,
        );
    }

    // Clear the old whole-HWND accent effect. SetWindowRgn does not reliably
    // clip its DWM backing, so blur now lives in a geometrically clipped visual.
    let module =
        unsafe { GetModuleHandleW(w!("user32.dll")) }.map_err(|error| error.to_string())?;
    let procedure = unsafe { GetProcAddress(module, s!("SetWindowCompositionAttribute")) }
        .ok_or_else(|| "当前 Windows 无法提供组件原生背景模糊。".to_string())?;
    let set_composition: SetCompositionAttribute = unsafe { std::mem::transmute(procedure) };
    let mut policy = AccentPolicy {
        state: ACCENT_DISABLED,
        flags: 0,
        gradient_color: 0,
        animation_id: 0,
    };
    let mut data = CompositionAttributeData {
        attribute: WCA_ACCENT_POLICY,
        data: &mut policy as *mut _ as *mut c_void,
        size: size_of::<AccentPolicy>(),
    };
    if !unsafe { set_composition(hwnd, &mut data) }.as_bool() {
        return Err(format!(
            "无法设置组件原生背景模糊：{}",
            std::io::Error::last_os_error()
        ));
    }
    let enabled = transparency > 0 && transparency < 100;
    let allow_host_backdrop = i32::from(enabled);
    let host_support = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_HOSTBACKDROPBRUSH,
            &allow_host_backdrop as *const _ as *const c_void,
            size_of::<i32>() as u32,
        )
    };
    if host_support.is_err() {
        // Older Windows versions use the transparent CSS surface; do not bring
        // back the rectangular blur backing as a fallback.
        SURFACES.with(|surfaces| {
            surfaces.borrow_mut().remove(&(hwnd.0 as usize));
        });
        return Ok(());
    }
    if enabled {
        ensure_surface(hwnd).map_err(|error| format!("无法创建组件圆角毛玻璃：{error}"))?;
    }
    SURFACES.with(|surfaces| {
        if let Some(surface) = surfaces.borrow().get(&(hwnd.0 as usize)) {
            surface
                .visual
                .SetOpacity(if enabled { 1.0 } else { 0.0 })
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    })
}

fn ensure_surface(hwnd: HWND) -> windows::core::Result<()> {
    if SURFACES.with(|surfaces| surfaces.borrow().contains_key(&(hwnd.0 as usize))) {
        return Ok(());
    }
    let compositor = HOST.with(|host| -> windows::core::Result<Compositor> {
        let mut host = host.borrow_mut();
        if host.is_none() {
            let apartment = ApartmentGuard(unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.is_ok());
            let queue = if DispatcherQueue::GetForCurrentThread().is_ok() {
                None
            } else {
                Some(unsafe {
                    CreateDispatcherQueueController(DispatcherQueueOptions {
                        dwSize: size_of::<DispatcherQueueOptions>() as u32,
                        threadType: DQTYPE_THREAD_CURRENT,
                        apartmentType: DQTAT_COM_NONE,
                    })
                }?)
            };
            *host = Some(CompositionHost {
                compositor: Compositor::new()?,
                _queue: queue,
                _apartment: apartment,
            });
        }
        Ok(host.as_ref().unwrap().compositor.clone())
    })?;
    let interop: ICompositorDesktopInterop = compositor.cast()?;
    let target = unsafe { interop.CreateDesktopWindowTarget(hwnd, false) }?;
    let visual = compositor.CreateSpriteVisual()?;
    visual.SetBrush(&compositor.CreateHostBackdropBrush()?)?;
    let geometry = compositor.CreateRoundedRectangleGeometry()?;
    let clip = compositor.CreateGeometricClipWithGeometry(&geometry)?;
    visual.SetClip(&clip)?;
    target.SetRoot(&visual)?;
    SURFACES.with(|surfaces| {
        surfaces.borrow_mut().insert(
            hwnd.0 as usize,
            BackdropSurface {
                target,
                visual,
                geometry,
            },
        );
    });
    let scale = unsafe { GetDpiForWindow(hwnd) } as f64 / 96.0;
    update_geometry(hwnd, 19, scale).map_err(|error| {
        windows::core::Error::new(windows::core::HRESULT(0x80004005_u32 as i32), error)
    })
}

fn update_geometry(hwnd: HWND, radius: u8, scale: f64) -> Result<(), String> {
    let mut bounds = windows::Win32::Foundation::RECT::default();
    unsafe { GetClientRect(hwnd, &mut bounds) }.map_err(|error| error.to_string())?;
    let width = f64::from((bounds.right - bounds.left).max(0));
    let height = f64::from((bounds.bottom - bounds.top).max(0));
    // Retain fractional physical pixels, matching CSS logical pixels times DPI.
    let radius_px = (f64::from(radius) * scale)
        .min(width / 2.0)
        .min(height / 2.0);
    CORNERS.with(|corners| {
        corners.borrow_mut().insert(
            hwnd.0 as usize,
            RoundedBounds {
                width,
                height,
                radius: radius_px,
            },
        );
    });
    SURFACES.with(|surfaces| {
        let surfaces = surfaces.borrow();
        let Some(surface) = surfaces.get(&(hwnd.0 as usize)) else {
            return Ok(());
        };
        let size = Vector2 {
            X: width as f32,
            Y: height as f32,
        };
        surface
            .geometry
            .SetSize(size)
            .map_err(|error| error.to_string())?;
        surface
            .geometry
            .SetCornerRadius(Vector2 {
                X: radius_px as f32,
                Y: radius_px as f32,
            })
            .map_err(|error| error.to_string())?;
        surface
            .visual
            .SetSize(size)
            .map_err(|error| error.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, CreateRectRgn, DeleteObject, GetWindowRgn, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetWindowLongPtrW, SendMessageW, SetWindowLongPtrW,
        HTCLIENT, WS_OVERLAPPEDWINDOW,
    };

    struct HiddenWindow(HWND);

    impl Drop for HiddenWindow {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    unsafe extern "system" fn observe_activation(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        // The built-in STATIC test class itself defaults to transparent hit
        // testing. Provide the opaque client result that Tao supplies in an app.
        if message == WM_NCHITTEST {
            return LRESULT(HTCLIENT as isize);
        }
        if message == WM_NCACTIVATE {
            unsafe { *(data as *mut isize) = lparam.0 };
        }
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }

    #[test]
    fn frame_stays_borderless_after_style_reset_and_activation_is_forwarded() {
        let mut observed_activation = 0_isize;
        let window = HiddenWindow(unsafe {
            CreateWindowExW(
                WS_EX_APPWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("Vela hidden frame regression"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                332,
                450,
                None,
                None,
                None,
                None,
            )
            .expect("create hidden regression window")
        });
        unsafe {
            assert!(SetWindowSubclass(
                window.0,
                Some(observe_activation),
                1,
                &mut observed_activation as *mut isize as usize,
            )
            .as_bool());
            assert!(
                SetWindowSubclass(window.0, Some(widget_frame_proc), WIDGET_FRAME_SUBCLASS, 0)
                    .as_bool()
            );

            // Exercise the real native API when enabling, clearing, and restoring
            // blur on this machine, without showing a desktop window.
            apply_to_hwnd(window.0, 50).expect("enable native blur");
            update_geometry(window.0, 19, 1.25).expect("apply fractional DPI corner radius");
            SURFACES.with(|surfaces| {
                if let Some(surface) = surfaces.borrow().get(&(window.0 .0 as usize)) {
                    let radius = surface.geometry.CornerRadius().unwrap();
                    assert_eq!((radius.X, radius.Y), (23.75, 23.75));
                }
            });
            update_geometry(window.0, 30, 1.5).expect("apply scaled rounded backdrop clip");
            SURFACES.with(|surfaces| {
                let surfaces = surfaces.borrow();
                if let Some(surface) = surfaces.get(&(window.0 .0 as usize)) {
                    let radius = surface.geometry.CornerRadius().expect("read native radius");
                    assert_eq!((radius.X, radius.Y), (45.0, 45.0));
                    let clip: windows::UI::Composition::CompositionGeometricClip =
                        surface.visual.Clip().unwrap().cast().unwrap();
                    let clip_geometry: CompositionRoundedRectangleGeometry =
                        clip.Geometry().unwrap().cast().unwrap();
                    assert_eq!(clip_geometry, surface.geometry);
                }
            });
            apply_to_hwnd(window.0, 100).expect("clear native blur at full transparency");
            SURFACES.with(|surfaces| {
                if let Some(surface) = surfaces.borrow().get(&(window.0 .0 as usize)) {
                    assert_eq!(surface.visual.Opacity().unwrap(), 0.0);
                }
            });
            apply_to_hwnd(window.0, 50).expect("restore native blur");
            apply_to_hwnd(window.0, 0).expect("disable hidden blur under an opaque surface");

            // Rendering keeps its alpha edge rather than acquiring a binary HWND
            // region. Corner hit testing still follows the rounded shape.
            let query_region = CreateRectRgn(0, 0, 0, 0);
            assert!(!query_region.0.is_null());
            let region_type = GetWindowRgn(window.0, query_region);
            let _ = DeleteObject(HGDIOBJ(query_region.0));
            assert_eq!(
                region_type.0, 0,
                "no GDI window region should clip the alpha edge"
            );

            let hit_test = |x: i32, y: i32| {
                let mut point = POINT { x, y };
                assert!(ClientToScreen(window.0, &mut point).as_bool());
                let screen_coordinates = (point.x as u16 as u32) | ((point.y as u16 as u32) << 16);
                SendMessageW(
                    window.0,
                    WM_NCHITTEST,
                    Some(WPARAM(0)),
                    Some(LPARAM(screen_coordinates as isize)),
                )
            };
            assert_eq!(hit_test(0, 0).0, HTTRANSPARENT as isize);
            assert_ne!(hit_test(100, 100).0, HTTRANSPARENT as isize);

            // Reproduce Tao restoring normal top-level window styles on show or
            // other flag updates after our initial borderless setup.
            SetWindowLongPtrW(window.0, GWL_STYLE, WS_OVERLAPPEDWINDOW.0 as isize);
            let style = GetWindowLongPtrW(window.0, GWL_STYLE) as u32;
            assert_eq!(style & WS_CAPTION.0, 0);
            assert_eq!(
                style & (WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_SYSMENU.0),
                0
            );

            SetWindowLongPtrW(window.0, GWL_EXSTYLE, WS_EX_APPWINDOW.0 as isize);
            let extended_style = GetWindowLongPtrW(window.0, GWL_EXSTYLE) as u32;
            assert_eq!(extended_style & WS_EX_APPWINDOW.0, 0);
            assert_ne!(extended_style & WS_EX_TOOLWINDOW.0, 0);

            SendMessageW(window.0, WM_NCACTIVATE, Some(WPARAM(1)), Some(LPARAM(0)));
            assert_eq!(observed_activation, -1);
            observed_activation = 0;
            SendMessageW(window.0, WM_NCACTIVATE, Some(WPARAM(0)), Some(LPARAM(0)));
            assert_eq!(observed_activation, -1);
        }
    }
}
