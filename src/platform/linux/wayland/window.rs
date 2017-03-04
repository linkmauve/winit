use std::sync::{Arc, Mutex};

use wayland_client::{EventQueue, EventQueueHandle, Init, Proxy};
use wayland_client::protocol::{wl_display,wl_surface,wl_shell_surface};

use {CreationError, MouseCursor, CursorState, WindowAttributes};
use platform::MonitorId as PlatformMonitorId;

use super::{WaylandContext, EventsLoop};
use super::wayland_window;
use super::wayland_window::DecoratedSurface;

pub struct Window {
    ctxt: Arc<WaylandContext>,
    evq: Arc<Mutex<EventQueue>>,
    surface: Arc<wl_surface::WlSurface>,
    size: Mutex<(u32, u32)>,
    decorated_id: usize
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(usize);

#[inline]
pub fn make_wid(s: &wl_surface::WlSurface) -> WindowId {
    WindowId(s.ptr() as usize)
}

impl Window {
    pub fn new(evlp: &EventsLoop, ctxt: Arc<WaylandContext>, attributes: &WindowAttributes)  -> Result<Window, CreationError>
    {
        let (width, height) = attributes.dimensions.unwrap_or((800,600));

        let (surface, decorated) = ctxt.create_window::<DecoratedHandler>(evlp.get_sink());

        // init DecoratedSurface
        let evq = evlp.get_event_queue();
        let decorated_id = {
            let mut evq_guard = evq.lock().unwrap();
            let decorated_id = evq_guard.add_handler_with_init(decorated);
            {
                let mut state = evq_guard.state();
                let decorated = state.get_mut_handler::<DecoratedSurface<DecoratedHandler>>(decorated_id);
                *(decorated.handler()) = Some(DecoratedHandler::new());

                if let Some(PlatformMonitorId::Wayland(ref monitor_id)) = attributes.monitor {
                    ctxt.with_output(monitor_id.clone(), |output| {
                        decorated.set_fullscreen(
                            wl_shell_surface::FullscreenMethod::Default,
                            0,
                            Some(output)
                        )
                    });
                } else if attributes.decorations {
                    decorated.set_decorate(true);
                }
                // Finally, set the decorations size
                decorated.resize(width as i32, height as i32);
            }
            decorated_id
        };
        let me = Window {
            ctxt: ctxt,
            evq: evq,
            surface: surface,
            size: Mutex::new((width, height)),
            decorated_id: decorated_id
        };

        evlp.register_window(me.decorated_id, me.id());

        Ok(me)
    }

    #[inline]
    pub fn id(&self) -> WindowId {
        make_wid(&self.surface)
    }

    pub fn set_title(&self, title: &str) {
        let mut guard = self.evq.lock().unwrap();
        let mut state = guard.state();
        let decorated = state.get_mut_handler::<DecoratedSurface<DecoratedHandler>>(self.decorated_id);
        decorated.set_title(title.into())
    }

    #[inline]
    pub fn show(&self) {
        // TODO
    }

    #[inline]
    pub fn hide(&self) {
        // TODO
    }

    #[inline]
    pub fn get_position(&self) -> Option<(i32, i32)> {
        // Not possible with wayland
        None
    }

    #[inline]
    pub fn set_position(&self, _x: i32, _y: i32) {
        // Not possible with wayland
    }

    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        Some(self.size.lock().unwrap().clone())
    }

    #[inline]
    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        let (w, h) = self.size.lock().unwrap().clone();
        let (w, h) = super::wayland_window::add_borders(w as i32, h as i32);
        Some((w as u32, h as u32))
    }

    #[inline]
    // NOTE: This will only resize the borders, the contents must be updated by the user
    pub fn set_inner_size(&self, x: u32, y: u32) {
        let mut guard = self.evq.lock().unwrap();
        let mut state = guard.state();
        let mut decorated = state.get_mut_handler::<DecoratedSurface<DecoratedHandler>>(self.decorated_id);
        decorated.resize(x as i32, y as i32);
    }

    #[inline]
    pub fn set_cursor(&self, _cursor: MouseCursor) {
        // TODO
    }

    #[inline]
    pub fn set_cursor_state(&self, state: CursorState) -> Result<(), String> {
        use CursorState::{Grab, Normal, Hide};
        // TODO : not yet possible on wayland to grab cursor
        match state {
            Grab => Err("Cursor cannot be grabbed on wayland yet.".to_string()),
            Hide => Err("Cursor cannot be hidden on wayland yet.".to_string()),
            Normal => Ok(())
        }
    }

    #[inline]
    pub fn hidpi_factor(&self) -> f32 {
        // TODO
        1.0
    }

    #[inline]
    pub fn set_cursor_position(&self, _x: i32, _y: i32) -> Result<(), ()> {
        // TODO: not yet possible on wayland
        Err(())
    }
    
    pub fn get_display(&self) -> &wl_display::WlDisplay {
        &self.ctxt.display
    }
    
    pub fn get_surface(&self) -> &wl_surface::WlSurface {
        &self.surface
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.surface.destroy();
        self.ctxt.prune_dead_windows();
    }
}

pub struct DecoratedHandler {
    newsize: Option<(u32, u32)>
}

impl DecoratedHandler {
    fn new() -> DecoratedHandler { DecoratedHandler { newsize: None }}

    pub fn take_newsize(&mut self) -> Option<(u32, u32)> {
        self.newsize.take()
    }
}

impl wayland_window::Handler for DecoratedHandler {
    fn configure(&mut self,
                 _: &mut EventQueueHandle,
                 _: wl_shell_surface::Resize,
                 width: i32, height: i32)
    {
        use std::cmp::max;
        self.newsize = Some((max(width,1) as u32, max(height,1) as u32));
    }
}
