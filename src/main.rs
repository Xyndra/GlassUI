#![allow(dead_code)]

mod renderer;
mod app;

use gl::types::*;
use gl_rs as gl;
use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{ContextApi, ContextAttributesBuilder},
    display::{GetGlDisplay, GlDisplay},
    prelude::NotCurrentGlContext,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
#[allow(deprecated)]
use raw_window_handle::HasRawWindowHandle;
use skia_safe::gpu::{self, gl::FramebufferInfo};
use std::{
    ffi::CString,
    num::NonZeroU32,
    time::Instant,
};
use winit::{
    dpi::LogicalSize,
    event::Modifiers,
    event_loop::EventLoop,
    window::WindowAttributes,
};
use crate::app::*;

fn main() {
    let el = EventLoop::new().expect("Failed to create event loop");

    let window_attributes = WindowAttributes::default()
        .with_title("rust-skia-gl-window")
        .with_inner_size(LogicalSize::new(800, 800));

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_transparency(true);

    let display_builder = DisplayBuilder::new().with_window_attributes(window_attributes.into());
    let (window, gl_config) = display_builder
        .build(&el, template, |configs| {
            // Find the config with the minimum number of samples. Usually Skia takes care of
            // antialiasing and may not be able to create appropriate Surfaces for samples > 0.
            // See https://github.com/rust-skia/rust-skia/issues/782
            // And https://github.com/rust-skia/rust-skia/issues/764
            configs
                .reduce(|accum, config| {
                    let transparency_check = config.supports_transparency().unwrap_or(false)
                        & !accum.supports_transparency().unwrap_or(false);

                    if transparency_check || config.num_samples() < accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        })
        .unwrap();
    println!("Picked a config with {} samples", gl_config.num_samples());
    let window = window.expect("Could not create window with OpenGL context");
    #[allow(deprecated)]
    let raw_window_handle = window
        .raw_window_handle()
        .expect("Failed to retrieve RawWindowHandle");

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(Some(raw_window_handle));
    let not_current_gl_context = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_config
                    .display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
    };

    let (width, height): (u32, u32) = window.inner_size().into();

    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(width).unwrap(),
        NonZeroU32::new(height).unwrap(),
    );

    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .expect("Could not create gl window surface")
    };

    let gl_context = not_current_gl_context
        .make_current(&gl_surface)
        .expect("Could not make GL context current when setting up skia renderer");

    gl::load_with(|s| {
        gl_config
            .display()
            .get_proc_address(CString::new(s).unwrap().as_c_str())
    });
    let interface = gpu::gl::Interface::new_load_with(|name| {
        if name == "eglGetCurrentDisplay" {
            return std::ptr::null();
        }
        gl_config
            .display()
            .get_proc_address(CString::new(name).unwrap().as_c_str())
    })
        .expect("Could not create interface");

    let mut gr_context = gpu::direct_contexts::make_gl(interface, None)
        .expect("Could not create direct context");

    let fb_info = {
        let mut fboid: GLint = 0;
        unsafe { gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut fboid) };

        FramebufferInfo {
            fboid: fboid.try_into().unwrap(),
            format: gpu::gl::Format::RGBA8.into(),
            ..Default::default()
        }
    };

    let num_samples = gl_config.num_samples() as usize;
    let stencil_size = gl_config.stencil_size() as usize;

    let surface = create_surface(&window, fb_info, &mut gr_context, num_samples, stencil_size);

    let env = Env {
        surface,
        gl_surface,
        gl_context,
        gr_context,
        window,
    };

    let mut application = Application {
        env,
        fb_info,
        num_samples,
        stencil_size,
        modifiers: Modifiers::default(),
        frame: 0,
        previous_frame_start: Instant::now(),
    };

    el.run_app(&mut application).expect("run() failed");
}