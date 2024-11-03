use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use glutin::context::PossiblyCurrentContext;
use glutin::prelude::GlSurface;
use glutin::surface::WindowSurface;
use glutin::surface::Surface as GlutinSurface;
use skia_safe::{gpu, Color, ColorType, Surface};
use skia_safe::gpu::{backend_render_targets, SurfaceOrigin};
use skia_safe::gpu::gl::FramebufferInfo;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, Modifiers, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::Window;
use crate::renderer;

pub(crate) fn create_surface(
    window: &Window,
    fb_info: FramebufferInfo,
    gr_context: &mut gpu::DirectContext,
    num_samples: usize,
    stencil_size: usize,
) -> Surface {
    let size = window.inner_size();
    let size = (
        size.width.try_into().expect("Could not convert width"),
        size.height.try_into().expect("Could not convert height"),
    );
    let backend_render_target =
        backend_render_targets::make_gl(size, num_samples, stencil_size, fb_info);

    gpu::surfaces::wrap_backend_render_target(
        gr_context,
        &backend_render_target,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
        .expect("Could not create skia surface")
}

// Guarantee the drop order inside the FnMut closure. `Window` _must_ be dropped after
// `DirectContext`.
//
// <https://github.com/rust-skia/rust-skia/issues/476>
pub(crate) struct Env {
    pub(crate) surface: Surface,
    pub(crate) gl_surface: GlutinSurface<WindowSurface>,
    pub(crate) gr_context: gpu::DirectContext,
    pub(crate) gl_context: PossiblyCurrentContext,
    pub(crate) window: Window,
}

pub(crate) struct Application {
    pub(crate) env: Env,
    pub(crate) fb_info: FramebufferInfo,
    pub(crate) num_samples: usize,
    pub(crate) stencil_size: usize,
    pub(crate) modifiers: Modifiers,
    pub(crate) frame: usize,
    pub(crate) previous_frame_start: Instant,
}

impl ApplicationHandler for Application {
    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let winit::event::StartCause::ResumeTimeReached { .. } = cause {
            self.env.window.request_redraw()
        }
    }

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let mut draw_frame = false;
        let frame_start = Instant::now();

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(physical_size) => {
                self.env.surface = create_surface(
                    &self.env.window,
                    self.fb_info,
                    &mut self.env.gr_context,
                    self.num_samples,
                    self.stencil_size,
                );
                /* First resize the opengl drawable */
                let (width, height): (u32, u32) = physical_size.into();

                self.env.gl_surface.resize(
                    &self.env.gl_context,
                    NonZeroU32::new(width.max(1)).unwrap(),
                    NonZeroU32::new(height.max(1)).unwrap(),
                );
            }
            WindowEvent::ModifiersChanged(new_modifiers) => self.modifiers = new_modifiers,
            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, .. },
                ..
            } => {
                if self.modifiers.state().super_key() && logical_key == "q" {
                    event_loop.exit();
                }
                self.frame = self.frame.saturating_sub(10);
                self.env.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                // draw_frame = true;
            }
            _ => (),
        }

        let expected_frame_length_seconds = 1.0 / 20.0;
        let frame_duration = Duration::from_secs_f32(expected_frame_length_seconds);

        if frame_start - self.previous_frame_start > frame_duration {
            draw_frame = true;
            self.previous_frame_start = frame_start;
        }
        if draw_frame {
            self.frame += 1;
            let canvas = self.env.surface.canvas();
            canvas.clear(Color::WHITE);
            renderer::render_frame(self.frame % 360, 12, 60, canvas);
            self.env.gr_context.flush_and_submit();
            self.env
                .gl_surface
                .swap_buffers(&self.env.gl_context)
                .unwrap();
        }

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            self.previous_frame_start + frame_duration,
        ));
    }
}