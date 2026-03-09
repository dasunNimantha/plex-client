use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;

use adw::prelude::*;

use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};
use libmpv2::Mpv;

use std::cell::{Cell, RefCell};
use std::ffi::{c_void, CString};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use super::util;

type GlGetProcAddr = unsafe extern "C" fn(*const libc::c_char) -> *mut c_void;

static GL_GPA: OnceLock<Option<GlGetProcAddr>> = OnceLock::new();

fn load_gl_proc_loader() -> Option<GlGetProcAddr> {
    unsafe {
        for lib_name in &["libGL.so.1", "libGL.so"] {
            let c_lib = CString::new(*lib_name).unwrap();
            let handle = libc::dlopen(c_lib.as_ptr(), libc::RTLD_LAZY);
            if !handle.is_null() {
                let sym = CString::new("glXGetProcAddressARB").unwrap();
                let ptr = libc::dlsym(handle, sym.as_ptr());
                if !ptr.is_null() {
                    return Some(std::mem::transmute(ptr));
                }
            }
        }
        for lib_name in &["libEGL.so.1", "libEGL.so"] {
            let c_lib = CString::new(*lib_name).unwrap();
            let handle = libc::dlopen(c_lib.as_ptr(), libc::RTLD_LAZY);
            if !handle.is_null() {
                let sym = CString::new("eglGetProcAddress").unwrap();
                let ptr = libc::dlsym(handle, sym.as_ptr());
                if !ptr.is_null() {
                    return Some(std::mem::transmute(ptr));
                }
            }
        }
        eprintln!("plex-client: could not find GL proc address loader");
        None
    }
}

fn gl_get_proc_address(_ctx: &(), name: &str) -> *mut c_void {
    let gpa = GL_GPA.get_or_init(load_gl_proc_loader);
    match gpa {
        Some(func) => unsafe {
            let c_name = CString::new(name).unwrap();
            func(c_name.as_ptr())
        },
        None => std::ptr::null_mut(),
    }
}

const GL_DRAW_FRAMEBUFFER_BINDING: u32 = 0x8CA6;

fn current_gl_fbo() -> i32 {
    let ptr = gl_get_proc_address(&(), "glGetIntegerv");
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        type GlGetIntegerv = unsafe extern "C" fn(u32, *mut i32);
        let func: GlGetIntegerv = std::mem::transmute(ptr);
        let mut fbo: i32 = 0;
        func(GL_DRAW_FRAMEBUFFER_BINDING, &mut fbo);
        fbo
    }
}

fn create_mpv(hwdec: &str) -> Option<Mpv> {
    unsafe {
        libc::setlocale(libc::LC_NUMERIC, b"C\0".as_ptr() as *const _);
    }
    let hwdec_val = hwdec.to_string();
    match Mpv::with_initializer(move |init| {
        init.set_property("vo", "libmpv")?;
        init.set_property("hwdec", hwdec_val.as_str())?;
        init.set_property("video-timing-offset", 0i64)?;
        init.set_property("tls-verify", false)?;
        init.set_property("tls-ca-file", "")?;
        Ok(())
    }) {
        Ok(m) => Some(m),
        Err(e) => {
            eprintln!("plex-client: failed to create mpv: {:?}", e);
            None
        }
    }
}

pub struct PlayerWidget {
    pub widget: gtk::Box,
    video_box: gtk::Box,
    header: adw::HeaderBar,
    controls: gtk::Box,
    gl_area: Rc<RefCell<Option<gtk::GLArea>>>,
    play_pause_btn: gtk::Button,
    seek_bar: gtk::Scale,
    time_label: gtk::Label,
    title_label: gtk::Label,
    header_title: adw::WindowTitle,
    mpv: Rc<RefCell<Option<Mpv>>>,
    render_ctx: Rc<RefCell<Option<RenderContext>>>,
    is_playing: Rc<Cell<bool>>,
    playback_started: Rc<Cell<bool>>,
    seeking: Rc<Cell<bool>>,
    on_stop_cb: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    pending_url: Rc<RefCell<Option<String>>>,
    initialized: Rc<Cell<bool>>,
    hwdec: String,
    fullscreen_btn: gtk::Button,
    controls_timeout: Rc<RefCell<Option<glib::SourceId>>>,
}

impl PlayerWidget {
    pub fn new(hwdec: &str) -> Rc<Self> {
        // Outer vertical box: header -> video area -> controls
        let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
        outer.set_vexpand(true);
        outer.set_hexpand(true);

        // Header bar with window controls
        let header = adw::HeaderBar::new();
        let header_title = adw::WindowTitle::new("Plex", "");
        header.set_title_widget(Some(&header_title));

        let back_btn = gtk::Button::from_icon_name("go-previous-symbolic");
        back_btn.add_css_class("flat");
        back_btn.set_tooltip_text(Some("Back to library"));
        header.pack_start(&back_btn);

        let fullscreen_btn = gtk::Button::from_icon_name("view-fullscreen-symbolic");
        fullscreen_btn.add_css_class("flat");
        fullscreen_btn.set_tooltip_text(Some("Toggle fullscreen"));
        header.pack_end(&fullscreen_btn);

        outer.append(&header);

        // Video area (GL area gets prepended here lazily)
        let video_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        video_box.set_vexpand(true);
        video_box.set_hexpand(true);
        outer.append(&video_box);

        // Controls bar
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        controls.set_margin_start(12);
        controls.set_margin_end(12);
        controls.set_margin_top(8);
        controls.set_margin_bottom(8);
        controls.add_css_class("playback-bar");

        let play_pause_btn = gtk::Button::from_icon_name("media-playback-pause-symbolic");
        play_pause_btn.add_css_class("flat");
        controls.append(&play_pause_btn);

        let stop_btn = gtk::Button::from_icon_name("media-playback-stop-symbolic");
        stop_btn.add_css_class("flat");
        stop_btn.set_tooltip_text(Some("Stop and return to library"));
        controls.append(&stop_btn);

        let time_label = gtk::Label::new(Some("0:00 / 0:00"));
        time_label.add_css_class("playback-time");
        time_label.set_margin_start(8);
        controls.append(&time_label);

        let seek_bar = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 0.1);
        seek_bar.set_hexpand(true);
        seek_bar.set_draw_value(false);
        controls.append(&seek_bar);

        let title_label = gtk::Label::new(None);
        title_label.add_css_class("playback-title");
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.set_max_width_chars(40);
        title_label.set_margin_end(8);
        controls.append(&title_label);

        outer.append(&controls);

        let pw = Rc::new(Self {
            widget: outer,
            video_box,
            header,
            controls,
            gl_area: Rc::new(RefCell::new(None)),
            play_pause_btn,
            seek_bar,
            time_label,
            title_label,
            header_title,
            mpv: Rc::new(RefCell::new(None)),
            render_ctx: Rc::new(RefCell::new(None)),
            is_playing: Rc::new(Cell::new(false)),
            playback_started: Rc::new(Cell::new(false)),
            seeking: Rc::new(Cell::new(false)),
            on_stop_cb: Rc::new(RefCell::new(None)),
            pending_url: Rc::new(RefCell::new(None)),
            initialized: Rc::new(Cell::new(false)),
            hwdec: hwdec.to_string(),
            fullscreen_btn,
            controls_timeout: Rc::new(RefCell::new(None)),
        });

        pw.setup_control_callbacks(&stop_btn, &back_btn);
        pw.setup_fullscreen_gestures();

        pw
    }

    pub fn set_on_stop<F: Fn() + 'static>(&self, f: F) {
        *self.on_stop_cb.borrow_mut() = Some(Box::new(f));
    }

    fn fire_on_stop(&self) {
        if let Some(cb) = self.on_stop_cb.borrow().as_ref() {
            cb();
        }
    }

    fn do_loadfile(&self, url: &str) {
        let mpv_borrow = self.mpv.borrow();
        if let Some(ref mpv) = *mpv_borrow {
            if let Err(e) = mpv.command("loadfile", &[url, "replace"]) {
                eprintln!("plex-client: loadfile error: {:?}", e);
            }
        }
    }

    /// Creates the GtkGLArea, mpv, and render context on first play.
    /// Nothing GL/mpv related runs at startup.
    fn ensure_initialized(self: &Rc<Self>) {
        if self.initialized.get() {
            return;
        }

        // Create GL area and insert it ABOVE the controls bar
        let gl_area = gtk::GLArea::new();
        gl_area.set_vexpand(true);
        gl_area.set_hexpand(true);
        gl_area.set_auto_render(false);

        self.video_box.prepend(&gl_area);

        // Set up render callback
        let pw_weak = Rc::downgrade(self);
        gl_area.connect_render({
            let pw = pw_weak.clone();
            move |gl_area, _gl_ctx| {
                let Some(pw) = pw.upgrade() else {
                    return glib::Propagation::Stop;
                };
                let ctx_borrow = pw.render_ctx.borrow();
                if let Some(ref ctx) = *ctx_borrow {
                    let scale = gl_area.scale_factor();
                    let width = gl_area.width() * scale;
                    let height = gl_area.height() * scale;
                    let fbo = current_gl_fbo();
                    let _ = ctx.render::<()>(fbo, width, height, true);
                }
                glib::Propagation::Stop
            }
        });

        gl_area.connect_unrealize({
            let pw = pw_weak.clone();
            move |_| {
                let Some(pw) = pw.upgrade() else { return };
                pw.render_ctx.borrow_mut().take();
                pw.mpv.borrow_mut().take();
                pw.initialized.set(false);
            }
        });

        // Force realize so we have a GL context
        gl_area.realize();
        gl_area.make_current();

        if let Some(err) = gl_area.error() {
            eprintln!("plex-client: GLArea error: {}", err);
            return;
        }

        *self.gl_area.borrow_mut() = Some(gl_area.clone());

        GL_GPA.get_or_init(load_gl_proc_loader);

        // Create mpv
        *self.mpv.borrow_mut() = create_mpv(&self.hwdec);

        let mut mpv_borrow = self.mpv.borrow_mut();
        let Some(ref mut mpv) = *mpv_borrow else { return };

        let render_ctx = RenderContext::new(
            unsafe { mpv.ctx.as_mut() },
            vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address: gl_get_proc_address,
                    ctx: (),
                }),
            ],
        );

        let mut render_ctx = match render_ctx {
            Ok(ctx) => ctx,
            Err(e) => {
                eprintln!("plex-client: failed to create render context: {:?}", e);
                return;
            }
        };

        let needs_render = Arc::new(AtomicBool::new(false));
        let needs_render_cb = needs_render.clone();
        render_ctx.set_update_callback(move || {
            needs_render_cb.store(true, Ordering::SeqCst);
        });

        let gl_area_weak = gl_area.downgrade();
        glib::timeout_add_local(Duration::from_millis(16), move || {
            if needs_render.swap(false, Ordering::SeqCst) {
                if let Some(gl_area) = gl_area_weak.upgrade() {
                    gl_area.queue_render();
                }
            }
            glib::ControlFlow::Continue
        });

        drop(mpv_borrow);
        *self.render_ctx.borrow_mut() = Some(render_ctx);
        self.initialized.set(true);
    }

    fn setup_control_callbacks(self: &Rc<Self>, stop_btn: &gtk::Button, back_btn: &gtk::Button) {
        let pw = Rc::downgrade(self);

        self.play_pause_btn.connect_clicked({
            let pw = pw.clone();
            move |btn| {
                let Some(pw) = pw.upgrade() else { return };
                let mpv_borrow = pw.mpv.borrow();
                if let Some(ref mpv) = *mpv_borrow {
                    let paused: bool = mpv.get_property("pause").unwrap_or(false);
                    let _ = mpv.set_property("pause", !paused);
                    btn.set_icon_name(if !paused {
                        "media-playback-start-symbolic"
                    } else {
                        "media-playback-pause-symbolic"
                    });
                }
            }
        });

        stop_btn.connect_clicked({
            let pw = pw.clone();
            move |_| {
                if let Some(pw) = pw.upgrade() {
                    pw.stop();
                }
            }
        });

        back_btn.connect_clicked({
            let pw = pw.clone();
            move |_| {
                if let Some(pw) = pw.upgrade() {
                    pw.stop();
                }
            }
        });

        self.fullscreen_btn.connect_clicked({
            let pw = pw.clone();
            move |_| {
                if let Some(pw) = pw.upgrade() {
                    pw.toggle_fullscreen();
                }
            }
        });

        self.seek_bar.connect_change_value({
            let pw = pw.clone();
            move |_, _, value| {
                let Some(pw) = pw.upgrade() else {
                    return glib::Propagation::Proceed;
                };
                pw.seeking.set(true);
                let mpv_borrow = pw.mpv.borrow();
                if let Some(ref mpv) = *mpv_borrow {
                    let _ = mpv.set_property("time-pos", value);
                }
                pw.seeking.set(false);
                glib::Propagation::Proceed
            }
        });

        glib::timeout_add_local(Duration::from_millis(500), {
            let pw = pw.clone();
            move || {
                let Some(pw) = pw.upgrade() else {
                    return glib::ControlFlow::Break;
                };
                if !pw.is_playing.get() {
                    return glib::ControlFlow::Continue;
                }

                let mpv_borrow = pw.mpv.borrow();
                if let Some(ref mpv) = *mpv_borrow {
                    let pos: f64 = mpv.get_property("time-pos").unwrap_or(0.0);
                    let dur: f64 = mpv.get_property("duration").unwrap_or(0.0);
                    let paused: bool = mpv.get_property("pause").unwrap_or(false);

                    if dur > 0.0 {
                        pw.playback_started.set(true);
                    }

                    if dur > 0.0 && !pw.seeking.get() {
                        pw.seek_bar.set_range(0.0, dur);
                        pw.seek_bar.set_value(pos);
                    }

                    pw.time_label.set_label(&format!(
                        "{} / {}",
                        util::format_time_secs(pos),
                        util::format_time_secs(dur),
                    ));

                    pw.play_pause_btn.set_icon_name(if paused {
                        "media-playback-start-symbolic"
                    } else {
                        "media-playback-pause-symbolic"
                    });

                    if pw.playback_started.get() {
                        let idle: bool = mpv.get_property("idle-active").unwrap_or(false);
                        if idle && pw.is_playing.get() {
                            pw.is_playing.set(false);
                            pw.playback_started.set(false);
                            drop(mpv_borrow);
                            pw.fire_on_stop();
                            return glib::ControlFlow::Continue;
                        }
                    }
                }
                glib::ControlFlow::Continue
            }
        });
    }

    fn toggle_fullscreen(self: &Rc<Self>) {
        let Some(window) = self.widget.root().and_then(|r| r.downcast::<gtk::Window>().ok()) else {
            return;
        };
        if window.is_fullscreen() {
            self.exit_fullscreen(&window);
        } else {
            self.enter_fullscreen(&window);
        }
    }

    fn enter_fullscreen(&self, window: &gtk::Window) {
        window.fullscreen();
        self.header.set_visible(false);
        self.controls.set_visible(false);
        self.fullscreen_btn.set_icon_name("view-restore-symbolic");
        self.fullscreen_btn.set_tooltip_text(Some("Exit fullscreen"));
        self.video_box.set_cursor_from_name(Some("none"));
    }

    fn exit_fullscreen(&self, window: &gtk::Window) {
        window.unfullscreen();
        self.header.set_visible(true);
        self.controls.set_visible(true);
        self.fullscreen_btn.set_icon_name("view-fullscreen-symbolic");
        self.fullscreen_btn.set_tooltip_text(Some("Toggle fullscreen"));
        self.video_box.set_cursor(None::<&gtk::gdk::Cursor>);
        if let Some(id) = self.controls_timeout.borrow_mut().take() {
            id.remove();
        }
    }

    fn show_controls_briefly(self: &Rc<Self>) {
        let Some(window) = self.widget.root().and_then(|r| r.downcast::<gtk::Window>().ok()) else {
            return;
        };
        if !window.is_fullscreen() {
            return;
        }
        self.header.set_visible(true);
        self.controls.set_visible(true);
        self.video_box.set_cursor(None::<&gtk::gdk::Cursor>);

        if let Some(id) = self.controls_timeout.borrow_mut().take() {
            id.remove();
        }

        let pw = Rc::downgrade(self);
        let id = glib::timeout_add_local_once(Duration::from_secs(3), move || {
            let Some(pw) = pw.upgrade() else { return };
            if let Some(win) = pw.widget.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                if win.is_fullscreen() {
                    pw.header.set_visible(false);
                    pw.controls.set_visible(false);
                    pw.video_box.set_cursor_from_name(Some("none"));
                }
            }
            *pw.controls_timeout.borrow_mut() = None;
        });
        *self.controls_timeout.borrow_mut() = Some(id);
    }

    fn setup_fullscreen_gestures(self: &Rc<Self>) {
        // Double-click on inner_box to toggle fullscreen
        let dbl_click = gtk::GestureClick::new();
        dbl_click.set_button(1);
        let pw = Rc::downgrade(self);
        dbl_click.connect_released(move |gesture, n_press, _, _| {
            if n_press == 2 {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(pw) = pw.upgrade() {
                    pw.toggle_fullscreen();
                }
            }
        });
        self.video_box.add_controller(dbl_click);

        // Mouse motion shows controls briefly in fullscreen
        let motion = gtk::EventControllerMotion::new();
        let pw = Rc::downgrade(self);
        motion.connect_motion(move |_, _, _| {
            if let Some(pw) = pw.upgrade() {
                if let Some(win) = pw.widget.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                    if win.is_fullscreen() {
                        pw.show_controls_briefly();
                    }
                }
            }
        });
        self.video_box.add_controller(motion);

        // Escape key exits fullscreen
        let key_ctrl = gtk::EventControllerKey::new();
        let pw = Rc::downgrade(self);
        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(pw) = pw.upgrade() {
                    if let Some(win) = pw.widget.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                        if win.is_fullscreen() {
                            pw.exit_fullscreen(&win);
                            return glib::Propagation::Stop;
                        }
                    }
                }
            }
            glib::Propagation::Proceed
        });
        self.widget.add_controller(key_ctrl);
    }

    pub fn play(self: &Rc<Self>, url: &str, title: &str) {
        self.title_label.set_label(title);
        self.header_title.set_title(title);
        self.is_playing.set(true);
        self.playback_started.set(false);
        self.play_pause_btn
            .set_icon_name("media-playback-pause-symbolic");
        self.seek_bar.set_value(0.0);
        self.time_label.set_label("0:00 / 0:00");

        self.ensure_initialized();

        if self.render_ctx.borrow().is_some() {
            self.do_loadfile(url);
        } else {
            *self.pending_url.borrow_mut() = Some(url.to_string());
        }
    }

    pub fn stop(&self) {
        let was_playing = self.is_playing.get();
        self.is_playing.set(false);
        if was_playing {
            let mpv_borrow = self.mpv.borrow();
            if let Some(ref mpv) = *mpv_borrow {
                let _ = mpv.command("stop", &[]);
            }
        }
        if let Some(window) = self.widget.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
            if window.is_fullscreen() {
                self.exit_fullscreen(&window);
            }
        }
        self.fire_on_stop();
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.get()
    }

    pub fn get_position_ms(&self) -> i64 {
        let mpv_borrow = self.mpv.borrow();
        if let Some(ref mpv) = *mpv_borrow {
            let pos: f64 = mpv.get_property("time-pos").unwrap_or(0.0);
            return (pos * 1000.0) as i64;
        }
        0
    }

    pub fn get_paused(&self) -> bool {
        let mpv_borrow = self.mpv.borrow();
        if let Some(ref mpv) = *mpv_borrow {
            return mpv.get_property("pause").unwrap_or(false);
        }
        false
    }
}
