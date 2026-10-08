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
        CompositionRoundedRectangleGeometry, Compositor, ContainerVisual,
        Desktop::DesktopWindowTarget, SpriteVisual,
    },
};
use windows_numerics::{Vector2, Vector3};

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
    root: ContainerVisual,
    visual: SpriteVisual,
    geometry: CompositionRoundedRectangleGeometry,
    desktop_layers: Vec<DesktopBackdropLayer>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DesktopBackdropBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
}

impl DesktopBackdropBounds {
    fn from_rect(rect: &crate::features::desktop::CanvasRect, scale: f64) -> Self {
        Self {
            x: (rect.x * scale) as f32,
            y: (rect.y * scale) as f32,
            width: (rect.width * scale) as f32,
            height: (rect.height * scale) as f32,
            radius: (rect.radius * scale)
                .min(rect.width * scale / 2.0)
                .min(rect.height * scale / 2.0) as f32,
        }
    }
}

struct DesktopBackdropLayer {
    visual: SpriteVisual,
    geometry: CompositionRoundedRectangleGeometry,
    bounds: Option<DesktopBackdropBounds>,
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
    static DESKTOP_MATERIALS: RefCell<HashMap<usize, u8>> = RefCell::new(HashMap::new());
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
            let _ = DESKTOP_MATERIALS.try_with(|materials| {
                materials.borrow_mut().remove(&(hwnd.0 as usize));
            });
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
    unsafe { apply_material_to_hwnd(hwnd, transparency, false) }
}

unsafe fn apply_material_to_hwnd(
    hwnd: HWND,
    transparency: u8,
    desktop: bool,
) -> Result<(), String> {
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
        ensure_surface(hwnd, !desktop)
            .map_err(|error| format!("无法创建组件圆角毛玻璃：{error}"))?;
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

fn ensure_surface(hwnd: HWND, attach_default: bool) -> windows::core::Result<()> {
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
    let root = compositor.CreateContainerVisual()?;
    if attach_default {
        root.Children()?.InsertAtTop(&visual)?;
    }
    target.SetRoot(&root)?;
    SURFACES.with(|surfaces| {
        surfaces.borrow_mut().insert(
            hwnd.0 as usize,
            BackdropSurface {
                target,
                root,
                visual,
                geometry,
                desktop_layers: Vec::new(),
            },
        );
    });
    if attach_default {
        let scale = unsafe { GetDpiForWindow(hwnd) } as f64 / 96.0;
        update_geometry(hwnd, 19, scale).map_err(|error| {
            windows::core::Error::new(windows::core::HRESULT(0x80004005_u32 as i32), error)
        })?;
    }
    // The canvas never allocates the legacy monitor-sized blur visual.
    Ok(())
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

// An HWND region excludes the empty desktop area from OS hit testing, including
// clicks intended for windows belonging to other processes. CSS transparency alone does not.
pub fn update_desktop_regions(
    window: &tauri::WebviewWindow,
    rects: Vec<crate::features::desktop::CanvasRect>,
    editing: bool,
    transparency: u8,
) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    on_window_thread(window, move |hwnd| unsafe {
        use windows::Win32::Graphics::Gdi::{
            CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, HGDIOBJ,
            RGN_OR,
        };
        let material_changed = DESKTOP_MATERIALS.with(|materials| {
            materials.borrow().get(&(hwnd.0 as usize)).copied() != Some(transparency)
        });
        if material_changed {
            apply_material_to_hwnd(hwnd, transparency, true)?;
            DESKTOP_MATERIALS.with(|materials| {
                materials.borrow_mut().insert(hwnd.0 as usize, transparency);
            });
        }
        let mut bounds = windows::Win32::Foundation::RECT::default();
        GetClientRect(hwnd, &mut bounds).map_err(|e| e.to_string())?;
        let region = if editing {
            CreateRectRgn(0, 0, bounds.right, bounds.bottom)
        } else {
            CreateRectRgn(0, 0, 0, 0)
        };
        if region.is_invalid() {
            return Err("无法创建桌面交互区域。".into());
        }
        for rect in &rects {
            if !editing {
                let radius = (rect.radius * scale).round() as i32;
                let piece = CreateRoundRectRgn(
                    (rect.x * scale).floor() as i32,
                    (rect.y * scale).floor() as i32,
                    ((rect.x + rect.width) * scale).ceil() as i32 + 1,
                    ((rect.y + rect.height) * scale).ceil() as i32 + 1,
                    radius * 2,
                    radius * 2,
                );
                if piece.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(region.0));
                    return Err("无法创建组件交互区域。".into());
                }
                let combined = CombineRgn(Some(region), Some(region), Some(piece), RGN_OR);
                let _ = DeleteObject(HGDIOBJ(piece.0));
                if combined.0 == 0 {
                    let _ = DeleteObject(HGDIOBJ(region.0));
                    return Err("无法合并组件交互区域。".into());
                }
            }
        }
        if SetWindowRgn(hwnd, Some(region), true) == 0 {
            let _ = DeleteObject(HGDIOBJ(region.0));
            return Err("无法应用桌面交互区域。".into());
        }
        // Windows owns the successful region; do not delete it.
        CORNERS.with(|corners| {
            corners.borrow_mut().remove(&(hwnd.0 as usize));
        });
        update_desktop_backdrops(hwnd, &rects, scale, transparency)
    })
}

// Popovers and tooltips affect only the HWND region. Retain blur layers across
// those updates, and mutate geometry only for widgets whose bounds changed.
fn update_desktop_backdrops(
    hwnd: HWND,
    rects: &[crate::features::desktop::CanvasRect],
    scale: f64,
    transparency: u8,
) -> Result<(), String> {
    SURFACES.with(|surfaces| -> Result<(), String> {
        let mut surfaces = surfaces.borrow_mut();
        let Some(surface) = surfaces.get_mut(&(hwnd.0 as usize)) else {
            return Ok(());
        };
        let bounds: Vec<_> = if transparency == 0 || transparency == 100 {
            Vec::new()
        } else {
            rects
                .iter()
                .filter(|r| r.backdrop && r.width > 0.0 && r.height > 0.0)
                .map(|r| DesktopBackdropBounds::from_rect(r, scale))
                .collect()
        };
        if surface.desktop_layers.len() == bounds.len()
            && surface
                .desktop_layers
                .iter()
                .zip(&bounds)
                .all(|(layer, b)| layer.bounds == Some(*b))
        {
            return Ok(());
        }
        let children = surface.root.Children().map_err(|e| e.to_string())?;
        while surface.desktop_layers.len() > bounds.len() {
            let layer = surface.desktop_layers.last().unwrap();
            children.Remove(&layer.visual).map_err(|e| e.to_string())?;
            surface.desktop_layers.pop();
        }
        for (index, bounds) in bounds.iter().enumerate() {
            if surface.desktop_layers.len() <= index {
                let compositor = surface.root.Compositor().map_err(|e| e.to_string())?;
                let visual = compositor.CreateSpriteVisual().map_err(|e| e.to_string())?;
                visual
                    .SetBrush(
                        &compositor
                            .CreateHostBackdropBrush()
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                let geometry = compositor
                    .CreateRoundedRectangleGeometry()
                    .map_err(|e| e.to_string())?;
                visual
                    .SetClip(
                        &compositor
                            .CreateGeometricClipWithGeometry(&geometry)
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                children.InsertAtTop(&visual).map_err(|e| e.to_string())?;
                surface.desktop_layers.push(DesktopBackdropLayer {
                    visual,
                    geometry,
                    bounds: None,
                });
            }
            let layer = &mut surface.desktop_layers[index];
            if layer.bounds == Some(*bounds) {
                continue;
            }
            let size = Vector2 {
                X: bounds.width,
                Y: bounds.height,
            };
            layer.geometry.SetSize(size).map_err(|e| e.to_string())?;
            layer
                .geometry
                .SetCornerRadius(Vector2 {
                    X: bounds.radius,
                    Y: bounds.radius,
                })
                .map_err(|e| e.to_string())?;
            layer.visual.SetSize(size).map_err(|e| e.to_string())?;
            layer
                .visual
                .SetOffset(Vector3 {
                    X: bounds.x,
                    Y: bounds.y,
                    Z: 0.0,
                })
                .map_err(|e| e.to_string())?;
            layer.bounds = Some(*bounds);
        }
        Ok(())
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
    fn desktop_blur_reuses_layers_for_popovers_moves_and_dpi_changes() {
        use crate::features::desktop::CanvasRect;
        let window = HiddenWindow(unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("Vela hidden backdrop regression"),
                WS_OVERLAPPEDWINDOW,
                0,
                0,
                1000,
                800,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        });
        unsafe {
            apply_material_to_hwnd(window.0, 60, true).unwrap();
        }
        // Platforms without HostBackdropBrush use the existing CSS fallback.
        if !SURFACES.with(|surfaces| surfaces.borrow().contains_key(&(window.0 .0 as usize))) {
            return;
        }
        let mut rects = vec![
            CanvasRect {
                x: 24.0,
                y: 24.0,
                width: 170.0,
                height: 170.0,
                radius: 19.0,
                backdrop: true,
            },
            CanvasRect {
                x: 210.0,
                y: 24.0,
                width: 364.0,
                height: 170.0,
                radius: 19.0,
                backdrop: true,
            },
        ];
        update_desktop_backdrops(window.0, &rects, 1.25, 60).unwrap();
        let original = SURFACES.with(|surfaces| {
            let surfaces = surfaces.borrow();
            let s = &surfaces[&(window.0 .0 as usize)];
            assert_eq!(s.root.Children().unwrap().Count().unwrap(), 2);
            assert_eq!(
                s.visual.Size().unwrap().X,
                0.0,
                "no full-monitor blur visual"
            );
            (
                s.desktop_layers[0].visual.clone(),
                s.desktop_layers[0].geometry.clone(),
                s.desktop_layers[1].visual.clone(),
            )
        });
        rects.push(CanvasRect {
            x: 500.0,
            y: 400.0,
            width: 190.0,
            height: 100.0,
            radius: 8.0,
            backdrop: false,
        });
        update_desktop_backdrops(window.0, &rects, 1.25, 60).unwrap();
        rects[0].x = 40.0;
        update_desktop_backdrops(window.0, &rects, 1.5, 60).unwrap();
        SURFACES.with(|surfaces| {
            let surfaces = surfaces.borrow();
            let s = &surfaces[&(window.0 .0 as usize)];
            assert_eq!(s.desktop_layers.len(), 2);
            assert_eq!(s.desktop_layers[0].visual, original.0);
            assert_eq!(s.desktop_layers[0].geometry, original.1);
            assert_eq!(s.desktop_layers[1].visual, original.2);
            assert_eq!(s.desktop_layers[0].visual.Offset().unwrap().X, 60.0);
            assert_eq!(s.desktop_layers[0].geometry.CornerRadius().unwrap().X, 28.5);
        });
        rects.remove(1);
        update_desktop_backdrops(window.0, &rects, 1.5, 60).unwrap();
        SURFACES.with(|surfaces| {
            assert_eq!(
                surfaces.borrow()[&(window.0 .0 as usize)]
                    .desktop_layers
                    .len(),
                1
            )
        });
        update_desktop_backdrops(window.0, &rects, 1.5, 100).unwrap();
        SURFACES.with(|surfaces| {
            let surfaces = surfaces.borrow();
            let s = &surfaces[&(window.0 .0 as usize)];
            assert!(s.desktop_layers.is_empty());
            assert_eq!(s.root.Children().unwrap().Count().unwrap(), 0);
        });
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
